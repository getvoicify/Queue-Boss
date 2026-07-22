//! A transport-agnostic conformance suite every [`QueueBackend`] adapter must
//! pass. It is the executable contract: `SandboxBackend` (C4) and the future
//! `PgBossBackend` (E2) both run it. The suite drives a shared [`ManualClock`]
//! so it can advance simulated time and observe lifecycle progression, and it
//! imports nothing from any adapter crate.

use std::collections::BTreeSet;

use crate::backend::QueueBackend;
use crate::clock::ManualClock;
use crate::model::{JobFilter, JobState, QueueSummary, TimelineEvent};

const PAGE_LIMIT: u32 = 3;
const FULL_LIMIT: u32 = 100_000;
const WARMUP_MS: u64 = 2_000;
const SWEEP_STEPS: u32 = 24;
const SWEEP_STEP_MS: u64 = 500;

fn filter_all(limit: u32, cursor: Option<String>) -> JobFilter {
    JobFilter {
        queue: None,
        states: None,
        time_window: None,
        search: None,
        cursor,
        limit,
    }
}

/// Assert that `backend` satisfies the read-only [`QueueBackend`] contract.
/// `clock` must be the same clock the backend reads from, so advancing it here
/// drives the backend's lifecycle forward. Panics with a descriptive message on
/// the first violation.
pub async fn assert_backend_conforms<B: QueueBackend>(backend: &B, clock: &ManualClock) {
    // Warm simulated time so several lifecycle states coexist for the static checks.
    clock.advance(WARMUP_MS);

    assert_queue_invariants(backend).await;
    assert_pagination(backend).await;
    assert_state_filter(backend).await;
    assert_timeline_ordered(backend).await;
    assert_progression_over_time(backend, clock).await;
}

async fn assert_queue_invariants<B: QueueBackend>(backend: &B) {
    let queues = backend
        .list_queues()
        .await
        .expect("list_queues must succeed");
    assert!(
        !queues.is_empty(),
        "list_queues must return at least one queue"
    );
    for queue in &queues {
        assert_summary_consistent(queue);
    }
}

fn assert_summary_consistent(queue: &QueueSummary) {
    let sum = queue
        .counts_by_state
        .values()
        .copied()
        .fold(0u64, u64::saturating_add);
    assert_eq!(
        sum, queue.total_depth,
        "queue {:?}: counts_by_state sum ({sum}) must equal total_depth ({})",
        queue.name, queue.total_depth
    );

    let waiting: u64 = queue
        .counts_by_state
        .iter()
        .filter(|(state, _)| state.is_waiting())
        .map(|(_, count)| *count)
        .sum();
    assert_eq!(
        queue.oldest_waiting_age.is_some(),
        waiting > 0,
        "queue {:?}: oldest_waiting_age ({:?}) must be present iff waiting jobs exist ({waiting})",
        queue.name,
        queue.oldest_waiting_age
    );
}

async fn assert_pagination<B: QueueBackend>(backend: &B) {
    let full = backend
        .list_jobs(filter_all(FULL_LIMIT, None))
        .await
        .expect("full-limit list_jobs must succeed");
    assert!(!full.has_more, "a full-limit page must not report has_more");
    assert!(
        full.next_cursor.is_none(),
        "a full-limit page must not carry a cursor"
    );
    let full_ids: Vec<_> = full.items.iter().map(|job| job.id.clone()).collect();
    let full_set: BTreeSet<_> = full_ids.iter().cloned().collect();
    assert_eq!(
        full_set.len(),
        full_ids.len(),
        "full listing must not contain duplicate ids"
    );

    let mut seen: Vec<_> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0u32;
    loop {
        let page = backend
            .list_jobs(filter_all(PAGE_LIMIT, cursor.clone()))
            .await
            .expect("paged list_jobs must succeed");
        assert!(
            page.items.len() as u32 <= PAGE_LIMIT,
            "page returned more items than the limit"
        );
        assert_eq!(
            page.has_more,
            page.next_cursor.is_some(),
            "has_more must hold exactly when next_cursor is present"
        );
        seen.extend(page.items.iter().map(|job| job.id.clone()));
        pages += 1;
        assert!(pages <= FULL_LIMIT, "pagination did not terminate");
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }

    let seen_set: BTreeSet<_> = seen.iter().cloned().collect();
    assert_eq!(
        seen_set.len(),
        seen.len(),
        "pagination yielded duplicate ids"
    );
    assert_eq!(
        seen_set, full_set,
        "walking the cursor must yield exactly the full listing (no gaps, no extras)"
    );
    if full_ids.len() as u32 > PAGE_LIMIT {
        assert!(
            pages > 1,
            "expected multiple pages for {} items at limit {PAGE_LIMIT}",
            full_ids.len()
        );
    }
}

async fn assert_state_filter<B: QueueBackend>(backend: &B) {
    let full = backend
        .list_jobs(filter_all(FULL_LIMIT, None))
        .await
        .expect("full-limit list_jobs must succeed");
    let present: BTreeSet<JobState> = full.items.iter().map(|job| job.state).collect();
    assert!(
        !present.is_empty(),
        "expected at least one job to exercise state filtering"
    );

    for state in present {
        let filter = JobFilter {
            states: Some(vec![state]),
            ..filter_all(FULL_LIMIT, None)
        };
        let filtered = backend
            .list_jobs(filter)
            .await
            .expect("filtered list_jobs must succeed");
        assert!(
            filtered.items.iter().all(|job| job.state == state),
            "state filter for {state:?} returned an off-state job"
        );
        let expected: BTreeSet<_> = full
            .items
            .iter()
            .filter(|job| job.state == state)
            .map(|job| job.id.clone())
            .collect();
        let got: BTreeSet<_> = filtered.items.iter().map(|job| job.id.clone()).collect();
        assert_eq!(
            got, expected,
            "state filter for {state:?} returned the wrong set of jobs"
        );
    }
}

async fn assert_timeline_ordered<B: QueueBackend>(backend: &B) {
    let full = backend
        .list_jobs(filter_all(FULL_LIMIT, None))
        .await
        .expect("full-limit list_jobs must succeed");
    let sample = full
        .items
        .first()
        .expect("expected at least one job to inspect");
    let detail = backend
        .get_job(&sample.id)
        .await
        .expect("get_job must succeed for a listed job");
    assert_timeline_valid(&detail.timeline, detail.summary.state);
}

async fn assert_progression_over_time<B: QueueBackend>(backend: &B, clock: &ManualClock) {
    let mut seen_states: BTreeSet<JobState> = BTreeSet::new();
    let mut seen_waiting = false;
    let mut seen_idle = false;

    for step in 0..=SWEEP_STEPS {
        if step > 0 {
            clock.advance(SWEEP_STEP_MS);
        }
        let queues = backend
            .list_queues()
            .await
            .expect("list_queues must succeed during the sweep");
        for queue in &queues {
            assert_summary_consistent(queue);
            match queue.oldest_waiting_age {
                Some(_) => seen_waiting = true,
                None => seen_idle = true,
            }
        }
        let jobs = backend
            .list_jobs(filter_all(FULL_LIMIT, None))
            .await
            .expect("list_jobs must succeed during the sweep");
        for job in &jobs.items {
            seen_states.insert(job.state);
        }
    }

    assert!(
        seen_states.contains(&JobState::Created),
        "never observed a Created job"
    );
    assert!(
        seen_states.contains(&JobState::Active),
        "never observed an Active job"
    );
    let reached_terminal = seen_states.contains(&JobState::Completed)
        || seen_states.contains(&JobState::DeadLetter)
        || seen_states.contains(&JobState::Cancelled);
    assert!(
        reached_terminal,
        "no job reached a terminal state over simulated time (saw {seen_states:?})"
    );
    assert!(
        seen_waiting,
        "never observed a queue with waiting jobs (oldest_waiting_age present)"
    );
    assert!(
        seen_idle,
        "never observed a queue that drained to no waiting jobs (oldest_waiting_age absent)"
    );

    let terminal = JobFilter {
        states: Some(vec![
            JobState::Completed,
            JobState::DeadLetter,
            JobState::Cancelled,
        ]),
        ..filter_all(FULL_LIMIT, None)
    };
    let terminal_jobs = backend
        .list_jobs(terminal)
        .await
        .expect("list_jobs for terminal states must succeed");
    let sample = terminal_jobs
        .items
        .first()
        .expect("expected a terminal job after the sweep");
    let detail = backend
        .get_job(&sample.id)
        .await
        .expect("get_job must succeed for a terminal job");
    assert_timeline_valid(&detail.timeline, detail.summary.state);
    assert!(
        matches!(
            detail.summary.state,
            JobState::Completed | JobState::DeadLetter | JobState::Cancelled
        ),
        "a job listed as terminal has non-terminal state {:?}",
        detail.summary.state
    );
}

fn assert_timeline_valid(timeline: &[TimelineEvent], current: JobState) {
    assert!(
        !timeline.is_empty(),
        "timeline must record at least the Created event"
    );
    assert_eq!(
        timeline.first().unwrap().state,
        JobState::Created,
        "timeline must start at Created"
    );
    for pair in timeline.windows(2) {
        assert!(
            pair[0].at <= pair[1].at,
            "timeline must be ordered ascending by at ({} then {})",
            pair[0].at,
            pair[1].at
        );
        assert!(
            is_valid_transition(pair[0].state, pair[1].state),
            "invalid lifecycle transition {:?} -> {:?}",
            pair[0].state,
            pair[1].state
        );
    }
    assert_eq!(
        timeline.last().unwrap().state,
        current,
        "the last timeline state must equal the current job state"
    );
}

fn is_valid_transition(from: JobState, to: JobState) -> bool {
    use JobState::*;
    matches!(
        (from, to),
        (Created, Active)
            | (Created, Cancelled)
            | (Active, Completed)
            | (Active, Failed)
            | (Active, Cancelled)
            | (Failed, Retry)
            | (Failed, DeadLetter)
            | (Retry, Active)
            | (Retry, Cancelled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::QueueBackend;
    use crate::clock::{Clock, ManualClock};
    use crate::error::BackendError;
    use crate::model::{oldest_waiting_age as core_oldest_waiting_age, JobState::*};
    use crate::model::{BackendInfo, Capabilities, JobDetail, JobId, JobSummary, RetryReadout};
    use crate::page::{decode_cursor, encode_cursor, Cursor, Page};
    use async_trait::async_trait;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[test]
    fn valid_transitions_are_accepted() {
        for (from, to) in [
            (Created, Active),
            (Created, Cancelled),
            (Active, Completed),
            (Active, Failed),
            (Failed, Retry),
            (Failed, DeadLetter),
            (Retry, Active),
        ] {
            assert!(is_valid_transition(from, to), "{from:?} -> {to:?}");
        }
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        for (from, to) in [
            (Created, Completed),
            (Completed, Active),
            (Active, Retry),
            (Retry, Completed),
            (DeadLetter, Active),
            (Cancelled, Active),
            (Completed, DeadLetter),
        ] {
            assert!(!is_valid_transition(from, to), "{from:?} -> {to:?}");
        }
    }

    /// Deliberate contract violations, used by the adversarial tests to prove the
    /// suite *rejects* non-conforming backends (and to kill the suite's own
    /// assertion mutants under `cargo mutants -p qb-core`).
    #[derive(Clone, Copy, PartialEq)]
    enum Break {
        None,
        NoQueues,
        BadDepth,
        BrokenPaging,
        NeverActive,
        OffStateFilter,
        UnorderedTimeline,
        IllegalTransition,
    }

    /// A minimal clock-driven backend that satisfies the suite, so the suite's
    /// own logic is exercised (and mutated) under `cargo mutants -p qb-core`.
    struct ConformingBackend {
        clock: Arc<dyn Clock>,
        jobs: Vec<FixtureJob>,
        broken: Break,
    }

    struct FixtureJob {
        id: JobId,
        queue: &'static str,
        created_at: u64,
        events: Vec<(u64, JobState)>,
    }

    impl FixtureJob {
        fn state_at(&self, now: u64) -> Option<JobState> {
            let mut state = None;
            for (at, s) in &self.events {
                if *at <= now {
                    state = Some(*s);
                } else {
                    break;
                }
            }
            state
        }

        fn timeline_at(&self, now: u64) -> Vec<TimelineEvent> {
            self.events
                .iter()
                .filter(|(at, _)| *at <= now)
                .map(|(at, state)| TimelineEvent {
                    at: *at,
                    state: *state,
                })
                .collect()
        }

        fn summary_at(&self, now: u64) -> Option<JobSummary> {
            let state = self.state_at(now)?;
            Some(JobSummary {
                id: self.id.clone(),
                queue: self.queue.to_owned(),
                state,
                created_at: self.created_at,
                started_at: self
                    .events
                    .iter()
                    .find(|(at, s)| *s == Active && *at <= now)
                    .map(|(at, _)| *at),
                completed_at: self
                    .events
                    .iter()
                    .find(|(at, s)| *s == Completed && *at <= now)
                    .map(|(at, _)| *at),
                attempts: self
                    .events
                    .iter()
                    .filter(|(at, s)| *s == Active && *at <= now)
                    .count() as u32,
                priority: 0,
            })
        }
    }

    impl ConformingBackend {
        fn new(clock: Arc<dyn Clock>) -> Self {
            Self::with_break(clock, Break::None)
        }

        fn with_break(clock: Arc<dyn Clock>, broken: Break) -> Self {
            let epoch = clock.now_ms();
            let job = |id: &str, queue, offsets: &[(u64, JobState)]| FixtureJob {
                id: JobId(id.to_owned()),
                queue,
                created_at: epoch,
                events: offsets.iter().map(|(o, s)| (epoch + o, *s)).collect(),
            };
            // A dead-letter lifecycle so the only terminal state seen is DeadLetter
            // (keeps the reached-terminal `||` chain honest under mutation).
            let dead_letter = |base: u64| {
                vec![
                    (0, Created),
                    (base, Active),
                    (base + 200, Failed),
                    (base + 250, Retry),
                    (base + 400, Active),
                    (base + 600, Failed),
                    (base + 650, DeadLetter),
                ]
            };
            // "aaa-0" sorts first and stays Created forever, so "steady" always
            // has a waiting job and the first row is never terminal.
            let jobs = if broken == Break::NeverActive {
                // Passes every static check but never progresses — only the
                // over-time progression assertions can reject it.
                vec![
                    job("s-0", "steady", &[(0, Created)]),
                    job("s-1", "steady", &[(0, Created)]),
                    job("s-2", "steady", &[(0, Created)]),
                    job("s-3", "steady", &[(0, Created)]),
                ]
            } else {
                vec![
                    job("aaa-0", "steady", &[(0, Created)]),
                    job("steady-1", "steady", &dead_letter(100)),
                    // Late lifecycle so Active/Failed/Retry are seen during the sweep.
                    job("steady-2", "steady", &dead_letter(3_000)),
                    // "batch": drains to DeadLetter, then sits idle (no waiting).
                    job("batch-0", "batch", &dead_letter(100)),
                    job("batch-1", "batch", &dead_letter(120)),
                    job("batch-2", "batch", &dead_letter(140)),
                ]
            };
            Self {
                clock,
                jobs,
                broken,
            }
        }

        fn now(&self) -> u64 {
            self.clock.now_ms()
        }
    }

    #[async_trait]
    impl QueueBackend for ConformingBackend {
        async fn test_connection(&self) -> Result<BackendInfo, BackendError> {
            Ok(BackendInfo {
                name: "fixture".to_owned(),
                healthy: true,
                detail: None,
            })
        }

        async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError> {
            if self.broken == Break::NoQueues {
                return Ok(Vec::new());
            }
            let now = self.now();
            let mut counts: BTreeMap<&str, BTreeMap<JobState, u64>> = BTreeMap::new();
            let mut waiting: BTreeMap<&str, Vec<(JobState, u64)>> = BTreeMap::new();
            for job in &self.jobs {
                if let Some(summary) = job.summary_at(now) {
                    *counts
                        .entry(job.queue)
                        .or_default()
                        .entry(summary.state)
                        .or_default() += 1;
                    waiting
                        .entry(job.queue)
                        .or_default()
                        .push((summary.state, (now - job.created_at) / 1_000));
                }
            }
            let mut summaries: Vec<QueueSummary> = counts
                .into_iter()
                .map(|(name, counts_by_state)| {
                    let oldest =
                        core_oldest_waiting_age(waiting.get(name).cloned().unwrap_or_default());
                    QueueSummary::new(name, counts_by_state, oldest)
                })
                .collect();
            if self.broken == Break::BadDepth {
                if let Some(first) = summaries.first_mut() {
                    first.total_depth += 1;
                }
            }
            Ok(summaries)
        }

        async fn list_jobs(&self, filter: JobFilter) -> Result<Page<JobSummary>, BackendError> {
            let now = self.now();
            let mut jobs: Vec<JobSummary> = self
                .jobs
                .iter()
                .filter_map(|job| job.summary_at(now))
                .filter(|s| {
                    filter
                        .states
                        .as_ref()
                        .is_none_or(|st| st.contains(&s.state))
                })
                .filter(|s| filter.queue.as_ref().is_none_or(|q| &s.queue == q))
                .collect();
            jobs.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
            if let Some(encoded) = &filter.cursor {
                let cursor = decode_cursor(encoded)?;
                jobs.retain(|s| (s.created_at, &s.id) > (cursor.created_at, &cursor.id));
            }
            let limit = filter.limit as usize;
            let has_more = limit > 0 && jobs.len() > limit;
            let mut items: Vec<JobSummary> = jobs.into_iter().take(limit).collect();
            if self.broken == Break::OffStateFilter
                && filter.states.as_ref().is_some_and(|s| s.contains(&Created))
            {
                // Return an off-state job for a non-terminal filter (progression's
                // terminal filter never asks for Created, so it stays clean).
                items.push(JobSummary {
                    id: JobId("intruder".to_owned()),
                    queue: "steady".to_owned(),
                    state: DeadLetter,
                    created_at: now,
                    started_at: None,
                    completed_at: None,
                    attempts: 0,
                    priority: 0,
                });
            }
            if self.broken == Break::BrokenPaging {
                // has_more without a cursor: violates the pagination invariant.
                return Ok(Page {
                    items,
                    next_cursor: None,
                    has_more: true,
                });
            }
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
            let now = self.now();
            let job = self
                .jobs
                .iter()
                .find(|job| &job.id == id)
                .ok_or_else(|| BackendError::NotFound(id.to_string()))?;
            let summary = job
                .summary_at(now)
                .ok_or_else(|| BackendError::NotFound(id.to_string()))?;
            // Corrupt only the first-sorted job's timeline, so the terminal-chain
            // job inspected later in the sweep stays valid and each break trips
            // exactly one assertion.
            let base = summary.created_at;
            let timeline = match self.broken {
                Break::UnorderedTimeline if id.0 == "aaa-0" => vec![
                    TimelineEvent {
                        at: base,
                        state: Created,
                    },
                    TimelineEvent {
                        at: base + 300,
                        state: Active,
                    },
                    TimelineEvent {
                        at: base + 100,
                        state: Failed,
                    },
                ],
                Break::IllegalTransition if id.0 == "aaa-0" => vec![
                    TimelineEvent {
                        at: base,
                        state: Created,
                    },
                    TimelineEvent {
                        at: base + 100,
                        state: Completed,
                    },
                ],
                _ => job.timeline_at(now),
            };
            Ok(JobDetail {
                summary,
                data: crate::model::Json::Null,
                output: crate::model::Json::Null,
                timeline,
                retry: RetryReadout {
                    attempts: 0,
                    max_attempts: Some(4),
                    next_retry_at: None,
                },
                extensions: BTreeMap::new(),
            })
        }

        fn capabilities(&self) -> Capabilities {
            Capabilities {
                priority: false,
                singleton: false,
                dead_letter: true,
                extensions: Vec::new(),
            }
        }
    }

    #[tokio::test]
    async fn the_suite_accepts_a_conforming_backend() {
        let clock = ManualClock::new(1_000_000);
        let backend = ConformingBackend::new(Arc::new(clock.clone()) as Arc<dyn Clock>);
        assert_backend_conforms(&backend, &clock).await;
    }

    async fn assert_rejects(broken: Break) {
        let clock = ManualClock::new(1_000_000);
        let backend =
            ConformingBackend::with_break(Arc::new(clock.clone()) as Arc<dyn Clock>, broken);
        assert_backend_conforms(&backend, &clock).await;
    }

    #[tokio::test]
    #[should_panic(expected = "at least one queue")]
    async fn the_suite_rejects_a_backend_with_no_queues() {
        assert_rejects(Break::NoQueues).await;
    }

    #[tokio::test]
    #[should_panic(expected = "total_depth")]
    async fn the_suite_rejects_counts_that_do_not_sum_to_depth() {
        assert_rejects(Break::BadDepth).await;
    }

    #[tokio::test]
    #[should_panic(expected = "has_more")]
    async fn the_suite_rejects_pagination_that_breaks_the_cursor_invariant() {
        assert_rejects(Break::BrokenPaging).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Active")]
    async fn the_suite_rejects_a_backend_whose_jobs_never_progress() {
        assert_rejects(Break::NeverActive).await;
    }

    #[tokio::test]
    #[should_panic(expected = "off-state")]
    async fn the_suite_rejects_a_state_filter_that_returns_off_state_jobs() {
        assert_rejects(Break::OffStateFilter).await;
    }

    #[tokio::test]
    #[should_panic(expected = "ordered ascending")]
    async fn the_suite_rejects_an_out_of_order_timeline() {
        assert_rejects(Break::UnorderedTimeline).await;
    }

    #[tokio::test]
    #[should_panic(expected = "invalid lifecycle transition")]
    async fn the_suite_rejects_an_illegal_lifecycle_transition() {
        assert_rejects(Break::IllegalTransition).await;
    }
}
