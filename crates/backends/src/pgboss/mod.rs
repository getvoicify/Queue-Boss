//! `PgBossBackend` — a read-only [`QueueBackend`] over a pg-boss v10 Postgres
//! schema. Version-detect + queue overview + capabilities land in E2-2a; the
//! job read path (`list_jobs`/`get_job`) is E2-2b.

mod map;
mod queries;
mod rows;

#[cfg(feature = "pg-integration")]
pub mod seed;

use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::PgPool;

use qb_core::{
    decode_cursor, encode_cursor, BackendError, BackendInfo, Capabilities, Cursor, JobDetail,
    JobFilter, JobId, JobSummary, Page, QueueBackend, QueueSummary, Seconds,
};

use self::map::{build_summaries, classify_version, to_detail, to_summary, SchemaFlavor};
use self::rows::{
    JobDetailRow, JobSummaryRow, OldestAgeRow, QueueNameRow, StateCountRow, VersionRow,
};

const DEFAULT_SCHEMA: &str = "pgboss";

/// Read-only adapter over a pg-boss v10 `PgPool`.
pub struct PgBossBackend {
    pool: PgPool,
    schema: String,
    flavor: SchemaFlavor,
}

impl PgBossBackend {
    /// Build a backend over `pool` for the default `pgboss` schema. v10 is the
    /// only flavor implemented in P0; `test_connection` gates the live schema
    /// version before the connect flow (E2-4) trusts this backend.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            schema: DEFAULT_SCHEMA.to_owned(),
            flavor: SchemaFlavor::V10,
        }
    }

    /// Oldest still-waiting (Created/Retry, due) age in seconds for one queue.
    async fn oldest_waiting_age(&self, queue: &str) -> Result<Option<Seconds>, BackendError> {
        let row: Option<OldestAgeRow> = sqlx::query_as(&queries::oldest_waiting_age(&self.schema))
            .bind(queue)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?;
        Ok(row.and_then(|r| r.age).map(|secs| secs.max(0) as Seconds))
    }
}

#[async_trait]
impl QueueBackend for PgBossBackend {
    async fn test_connection(&self) -> Result<BackendInfo, BackendError> {
        let version: Option<i32> =
            match sqlx::query_as::<_, VersionRow>(&queries::version(&self.schema))
                .fetch_optional(&self.pool)
                .await
            {
                Ok(row) => row.map(|r| r.version),
                // Missing `version` table -> not a pg-boss schema (Unsupported),
                // never a Connection error.
                Err(e) if is_undefined_table(&e) => None,
                Err(_) => {
                    return Err(BackendError::Connection(
                        "could not read the pg-boss schema version".to_owned(),
                    ))
                }
            };
        // Gate: an out-of-band or missing version yields `Unsupported` carrying
        // self-authored product copy (never a driver string).
        classify_version(version)?;
        let detail =
            version.map(|v| format!("pg-boss {} schema (version {v})", self.flavor.label()));
        Ok(BackendInfo {
            name: "pg-boss".to_owned(),
            healthy: true,
            detail,
        })
    }

    async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError> {
        let names: Vec<QueueNameRow> = sqlx::query_as(&queries::queue_names(&self.schema))
            .fetch_all(&self.pool)
            .await
            .map_err(internal)?;
        let counts: Vec<StateCountRow> = sqlx::query_as(&queries::state_counts(&self.schema))
            .fetch_all(&self.pool)
            .await
            .map_err(internal)?;

        let names: Vec<String> = names.into_iter().map(|r| r.name).collect();
        let mut age_pairs: Vec<(String, Option<Seconds>)> = Vec::with_capacity(names.len());
        for name in &names {
            let age = self.oldest_waiting_age(name).await?;
            age_pairs.push((name.clone(), age));
        }
        let ages: BTreeMap<String, Option<Seconds>> = age_pairs.into_iter().collect();
        build_summaries(&names, &counts, &ages)
    }

    async fn list_jobs(&self, filter: JobFilter) -> Result<Page<JobSummary>, BackendError> {
        let cursor = filter.cursor.as_deref().map(decode_cursor).transpose()?;
        let sql = queries::list_jobs(&self.schema, &filter, cursor.as_ref());
        let mut q = sqlx::query_as::<_, JobSummaryRow>(&sql);
        if let Some(c) = &cursor {
            q = q.bind(c.created_at as i64).bind(c.id.0.clone());
        }
        if let Some(queue) = &filter.queue {
            q = q.bind(queue.clone());
        }
        if let Some(states) = &filter.states {
            let wire: Vec<String> = states.iter().map(|s| s.to_string()).collect();
            q = q.bind(wire);
        }
        if let Some(tw) = &filter.time_window {
            q = q.bind(tw.from as i64).bind(tw.to as i64);
        }
        if let Some(search) = &filter.search {
            q = q.bind(format!("%{search}%"));
        }
        q = q.bind(filter.limit as i64 + 1);
        let rows: Vec<JobSummaryRow> = q.fetch_all(&self.pool).await.map_err(internal)?;
        let (rows, has_more) = map::paginate(rows, filter.limit);
        let items: Vec<JobSummary> = rows.into_iter().map(to_summary).collect::<Result<_, _>>()?;
        let next_cursor = if has_more {
            items.last().map(|s| {
                encode_cursor(&Cursor {
                    created_at: s.created_at,
                    id: s.id.clone(),
                })
            })
        } else {
            None
        };
        Ok(Page {
            items,
            next_cursor,
            has_more,
        })
    }

    async fn get_job(&self, id: &JobId) -> Result<JobDetail, BackendError> {
        if !map::is_uuid(&id.0) {
            return Err(BackendError::NotFound(id.to_string()));
        }
        let sql = queries::get_job(&self.schema);
        let row: Option<JobDetailRow> = sqlx::query_as(&sql)
            .bind(id.0.clone())
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?;
        let row = row.ok_or_else(|| BackendError::NotFound(id.to_string()))?;
        to_detail(row)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            priority: true,
            singleton: true,
            dead_letter: true,
            extensions: vec![
                "singletonKey".to_owned(),
                "policy".to_owned(),
                "priority".to_owned(),
            ],
        }
    }
}

/// Collapse any driver error onto a sanitized `Internal` — a raw SQL/driver
/// string must never reach the UI.
fn internal(_e: sqlx::Error) -> BackendError {
    BackendError::Internal("pg-boss query failed".to_owned())
}

/// SQLSTATE `42P01` = `undefined_table`: the `version` table is absent.
fn is_undefined_table(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(|db| db.code())
        .map(|code| code.as_ref() == "42P01")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capabilities_advertise_pgboss_priority_singleton_and_dead_letter() {
        // connect_lazy never touches the network, so this needs no database.
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let backend = PgBossBackend::new(pool);
        let caps = backend.capabilities();
        assert!(caps.priority);
        assert!(caps.singleton);
        assert!(caps.dead_letter);
        assert_eq!(caps.extensions, vec!["singletonKey", "policy", "priority"]);
    }
}
