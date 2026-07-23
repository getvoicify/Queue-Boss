# Epic 2 — pg-boss v10 Read Path — Design

**Product:** Queue Boss (getvoicify/Queue-Boss) — teaching-first, read-only desktop inspector for background job queues.
**Epic:** 2 of the MVP program (E1 Skeleton & Sandbox → **E2 pg-boss v10 Read Path** → {E3 Hero + Teaching, E4 Cross-platform Release}).
**Source:** [Queue Boss MVP PRD](../../queue-boss-mvp-prd.md) (Draft v0.2) — the product brief of record; this spec scopes its **Milestone 2** ("Read path: pg-boss v10 adapter — connect, version-detect, overview, lifecycle counts, job explorer, job detail. Deliverable: a fully functional read-only inspector against a real database."). Feature IDs touched: **F1** (connection management), **F2** (schema & version detection), **F3** (queue overview), **F5** (job explorer), **F6** (job detail), **F9** (near-live polling — inherited from E1's poller, now pointed at a real backend).
**Predecessor:** [E1 Skeleton & Sandbox Design](2026-07-21-queue-boss-skeleton-sandbox-design.md) + [Runbook](../plans/2026-07-21-queue-boss-skeleton-sandbox-plan.md). E1 shipped the `QueueBackend` seam, the `SandboxBackend` second-adapter proof, the conformance suite, the Tauri command/poller bridge, the strictly-declarative Angular layers, and the `qb-platform` `SecretStore` port **stubbed** (`NoopSecretStore`). E2 makes the seam earn its keep against a real database.
**Board:** org Project #3. **Planning:** self-hosted in this repo.

---

## 1. Problem this epic solves

E1 proved the abstraction is transport-agnostic *without a database* — two adapters (`SandboxBackend` plus the conformance suite's implicit contract) validate the seam, and the UI shows a live-updating fake queue. What E1 could **not** prove is the product's actual promise: **point Queue Boss at a real pg-boss database and read it safely.** That is E2.

Three things must become true, and each is load-bearing:

1. **A real adapter behind the E1 seam.** `PgBossBackend` implements the exact same `QueueBackend` trait (`crates/core/src/backend.rs:12`, under the `#[async_trait]` at `:11`) as the sandbox, over a live Postgres connection, **read-only**. It is "the first *real* adapter, not the only one" — the artifact the OSS contributor judges, now against a database whose schema we did not design.
2. **Honest schema/version handling.** pg-boss's on-disk schema is versioned by an **integer** in `pgboss.version.version` (not a semver), and it changed shape between majors. E2 supports **v10** (P0) behind a **version-detect seam**, degrades gracefully on anything else (`BackendError::Unsupported`, sanitized to the UI), and leaves **v11** as an additive P1 child through the same seam.
3. **Runtime connection lifecycle + real secrets.** E1's `AppState.backends` is boot-frozen (`src-tauri/src/state.rs:13`, `.manage()` at `lib.rs:101`). E2 makes it interior-mutable so a user can **connect** to a pg-boss instance and **disconnect** at runtime, one connection at a time alongside the always-present sandbox, with credentials in the **OS keychain** (the real `keyring`-backed implementation of E1's stubbed `SecretStore` port).

The conformance suite is the tension point. E1's suite is **`ManualClock`-coupled** (`crates/core/src/conformance.rs:34`, `assert_backend_conforms(&B, &ManualClock)`) — it *advances a fake clock* to force jobs through their lifecycle. A live Postgres cannot be clock-driven, so E2 must split the suite (see §3.5) rather than weaken it.

Everything E2 ships is **read-only**. No write path, no `LISTEN/NOTIFY`, no migrations run against the user's database.

## 2. Success criteria (epic is "done" when…)

- **Deliverable:** a user launches the app, opens a **connect form**, enters a connection string (or discrete host/port/db/user/password/SSL/schema fields, per PRD F1), connects to a **real pg-boss v10 database**, and sees the **overview** (per-queue depth + per-state counts + oldest-waiting age, live-polled for **that** connection), the **lifecycle counts**, a **job explorer** (paginated + state/time/search filtered), and **job detail** (payload, output, timeline, retry/backoff readout, capability-aware extension rows) — **all read-only, no writes issued**.
- Connecting to a **non-pg-boss / v9 / unrecognized** schema surfaces a **sanitized** `pg-boss v10 required (schema versions 21–24); found schema vN` message in the UI — never a raw SQL/driver string (`crates/core/src/error.rs:6`).
- **The sandbox still works, unchanged.** The sandbox is always present; connecting/disconnecting a pg-boss instance never disturbs it, and the overview/lifecycle **rekey to the active connection**.
- `PgBossBackend` passes the new **`assert_static_conformance(&B)`** against a **testcontainers-seeded pg-boss v10 dataset** covering all six native states + a derived dead-letter case + a waiting queue + a drained queue; the sandbox continues to pass **both** the static suite and the time-driven `assert_backend_conforms`.
- Every behavioral child shipped via outside-in TDD (`tdd-evidence`) with Rust logic covered under `rust-mutation-coverage`; FE children clean under `ng-declarative-purity` + `a11y-audit`; the CI-touching + keychain children green on the 3-OS nightly (`xplat-build-smoke`).
- **v11 (E2-7) is a stretch:** if it lands, version-detection routes v10 vs v11 mapping and passes conformance against a v11 container; if cut, v11 (schema 25) connections cleanly report `Unsupported` and the P0 deliverable is unaffected.

## 3. Chosen architecture

### 3.1 The version-detect seam (v10 P0; v11 additive P1)

`test_connection` reads `SELECT version FROM <schema>.version` (schema defaults to `pgboss`, configurable per PRD F1). The value is an **integer schema-migration version**, *not* the pg-boss semver. The exact integers (from each tag's `version.json`) are:

| pg-boss release | schema version | Queue Boss verdict |
|---|---|---|
| v9 and older | ≤ **20** | `Unsupported` (below floor) |
| v10.0.0 | **21** | v10 — supported |
| v10.0.6 | **22** | v10 — supported |
| v10.1.1 | **23** | v10 — supported |
| v10.1.5 / v10.3.2 | **24** | v10 — supported |
| v11.0.0 / v11.0.1 | **25** | v11 — `Unsupported` in P0; routed to v11 mapping once E2-7 lands |

- **v10 supported band = schema versions 21–24 (floor 21).** An integer in 21–24 → `Ok(BackendInfo{ … })`.
- An integer **≤ 20** (v9 and older — the non-partitioned era), or a **missing `version` table** (not a pg-boss schema at all) → `BackendError::Unsupported("pg-boss v10 required (schema versions 21–24); found schema vN")` (or `… found no pgboss schema`).
- An integer **= 25** (v11) → `Unsupported` in P0, **routed to the v11 query mapping** once E2-7 lands. **The exact v11 baseline is 25** (this resolves what was an open question — see §7).

The seam is a small internal enum — `SchemaFlavor::{V10, V11}` resolved once at connect time from the version integer — that selects the query set. **v10 is the only implemented flavor in P0.** v11 is *additive*: E2-7 adds a `SchemaFlavor::V11` arm and its query variants without touching the v10 path. We do **not** run pg-boss's own migration/contract machinery (that would write to the user's DB); we only read the integer and classify it.

### 3.2 sqlx **runtime** queries (not compile-time macros)

`PgBossBackend` is built over `sqlx::PgPool` using **runtime** query APIs — `sqlx::query_as::<_, Row>(sql).bind(..).fetch_all(&pool)` — **never** the compile-time-checked macros (`query!`, `query_as!`). Rationale: the macros require a live database (or a checked-in `.sqlx` offline cache) **at build time**, which would (a) make `cargo build`/`cargo test --workspace`/`npm run tauri:build` depend on Postgres in CI where **there is none today**, and (b) pin the build to one schema shape when the whole point of the seam is to select SQL by detected flavor at runtime. Runtime queries keep the build DB-free and let the flavor seam choose SQL strings dynamically. Row structs derive `sqlx::FromRow` (the sqlx **`derive`** feature — *not* `macros`); typed decode failures map to `BackendError::Internal` (sanitized), never a raw driver string to the UI.

### 3.3 pg-boss v10 schema → core model mapping

Grounded directly in pg-boss `10.3.2` `src/plans.js` (the canonical DDL). The core model is unchanged from E1 (`crates/core/src/model.rs`); this table is the adapter's contract.

**Source tables (v10):**
- `pgboss.version` — `version int PK, maintained_on tz, cron_on tz, monitored_on tz`.
- `pgboss.queue` — `name text PK, policy text, retry_limit int, retry_delay int, retry_backoff bool, expire_seconds int, retention_minutes int, dead_letter text REFERENCES queue(name), partition_name text, created_on tz, updated_on tz`.
- `pgboss.job` — `PARTITION BY LIST (name)`, one child partition per queue (`j<sha224(name)>`): `id uuid, name text, priority int, data jsonb, state pgboss.job_state, retry_limit int, retry_count int, retry_delay int, retry_backoff bool, start_after tz, started_on tz, singleton_key text, singleton_on ts, expire_in interval, created_on tz, completed_on tz, keep_until tz, output jsonb, dead_letter text, policy text`.
- `pgboss.job_state` ENUM — **six** values, **numeric-ordered** (declaration order is the sort order): `created(0) < retry(1) < active(2) < completed(3) < cancelled(4) < failed(5)`. This ordering is load-bearing: pg-boss's own predicates use `state < 'active'` (waiting) and `state < 'completed'` (not-yet-terminal).

| Core model field | pg-boss v10 source | Mapping notes |
|---|---|---|
| `JobState::Created` | `state = 'created'` | Waiting (`is_waiting`, `model.rs:50`). |
| `JobState::Retry` | `state = 'retry'` | Waiting; `start_after` holds the (possibly future) backoff target. |
| `JobState::Active` | `state = 'active'` | In flight; `started_on` set. |
| `JobState::Completed` | `state = 'completed'` | Terminal; `completed_on` set, `output` = result. |
| `JobState::Cancelled` | `state = 'cancelled'` | Terminal. |
| `JobState::Failed` | `state = 'failed'` **AND NOT** dead-letter predicate | Terminal failure with **no** DLQ route (see §3.4). |
| `JobState::DeadLetter` | **derived** — `state = 'failed' AND dead_letter IS NOT NULL AND dead_letter <> name` | pg-boss has **no** `deadLetter` state; predicate mirrors pg-boss's own `failJobs`→`dlq_jobs` WHERE clause (see §3.4). |
| `QueueSummary.name` | `queue.name` (and `job.name`) | Queue name = partition key. |
| `QueueSummary.counts_by_state` | grouped count over `job` with the §3.4 `CASE` bucketing `failed`→`deadLetter` | Each job lands in **exactly one** bucket → sum invariant preserved (`QueueSummary::new`, `model.rs:78/88`). |
| `QueueSummary.total_depth` | `sum(counts_by_state)` | Saturating sum per `::new`. |
| `QueueSummary.oldest_waiting_age` | `now() - min(start_after)` over `state < 'active' AND start_after <= now()` | Waiting = Created+Retry, filtered to **due** jobs (excludes future-dated backoff retries); `None` when no due waiting job. Seconds. |
| `JobSummary.id` | `job.id` (uuid) | `JobId` wraps the uuid. |
| `JobSummary.queue` | `job.name` | |
| `JobSummary.state` | as above | |
| `JobSummary.created_at` | `created_on` | Keyset cursor field. |
| `JobSummary.started_at?` | `started_on` | Nullable. |
| `JobSummary.completed_at?` | `completed_on` | Nullable (set for completed/failed/cancelled). |
| `JobSummary.attempts` | `retry_count` | `JobSummary.attempts: u32` (`model.rs:127`). |
| `JobSummary.priority` | `priority` | |
| `JobDetail.data` | `data` (jsonb) | Payload JSON viewer. |
| `JobDetail.output` | `output` (jsonb) | Result or error. |
| `JobDetail.timeline` | derived from `created_on` → `started_on` → `completed_on` | Ordered events, state-labelled terminal; must satisfy the core `is_valid_transition` whitelist. |
| `JobDetail.retry` (`RetryReadout`, `model.rs:142`) | `attempts = retry_count`; `max_attempts = Some(retry_limit)`; `next_retry_at = start_after` **as epoch-ms** when `state = 'retry'`, else `None` | `RetryReadout` has **only** `attempts: u32`, `max_attempts: Option<u32>`, `next_retry_at: Option<u64>` — there is **no** backoff/delay field on the struct; the "backoff strategy" story is told in the timeline/teaching layer, not this struct. |
| `JobDetail.extensions` (`BTreeMap<String,Json>`, `model.rs:159`) | `singleton_key`→`singletonKey`, `policy`→`policy`, `priority`→`priority` (+ `dead_letter`→`deadLetter` route) | camelCase keys; carried verbatim through the core `extensions` map (test-locked at `model.rs:308`). |
| `Capabilities` (`model.rs:174`) | `{ priority: true, singleton: true, dead_letter: true, extensions: ["singletonKey","policy","priority"] }` | The declared-extension list field is **`extensions: Vec<String>`** (`model.rs:178`); it drives capability-aware FE rows. |

**Queue enumeration.** `list_queues` reads `pgboss.queue` for the queue set (names, `policy`, `dead_letter` route, `retry_*`) and aggregates per-state counts from `pgboss.job`. pg-boss's own `countStates` (`SELECT name, state, count(*) FROM job GROUP BY rollup(name), rollup(state)`) counts only the **native six** states; the adapter substitutes its own aggregate carrying the §3.4 `CASE` so the `DeadLetter` bucket exists. A queue present in `pgboss.queue` with zero jobs is a **drained** queue (all-zero counts, `oldest_waiting_age = None`) — a first-class case in the conformance fixture.

**`get_job` partition note.** pg-boss's own `getJobById` is `WHERE name = $1 AND id = $2` (partition-pruned, because `job` is `PARTITION BY LIST (name)`). The core trait's `get_job(&self, id: &JobId)` is **queue-agnostic** — it carries no queue. The adapter therefore selects from the **parent** partitioned table by id alone (`SELECT … FROM pgboss.job WHERE id = $1 LIMIT 1`): the uuid is globally unique (`gen_random_uuid()`), so this is correct across all partitions, at the cost of partition pruning — acceptable for a read-only, human-paced detail lookup. (Recorded as a deliberate deviation in §7.)

**`list_jobs` pagination.** Keyset over `(created_on, id)` ordered `DESC`, matching the core `Page` cursor `{created_at, id}` base64url (`crates/core/src/page.rs`): predicate `(created_on, id) < ($cursorCreatedAt, $cursorId)`, `LIMIT $limit`, `has_more` via the EXISTS/fetch-one-past pattern. `JobFilter` (`model.rs:192`) maps: `queue?` → `WHERE name = $q`; `states?` → filter on the DeadLetter-`CASE` projection; `time_window?` → `created_on` between bounds; `search?` → `data @> $json` / text containment on the payload (PRD F5).

### 3.4 Dead-letter derivation (the one place pg-boss and the model disagree)

pg-boss has **no `deadLetter` job state** — the `job_state` enum is six values. Dead-lettering is a **queue-level** mechanism: `queue.dead_letter` names another queue. When a job exhausts retries (`retry_count = retry_limit`), pg-boss's `failJobs` sets its state to `'failed'` **in place**, and — in its `dlq_jobs` CTE — inserts a **fresh copy** as `state = 'created'` into the dead-letter queue, gated by exactly `WHERE state = 'failed' AND dead_letter IS NOT NULL AND NOT name = dead_letter` (the last clause prevents a queue dead-lettering into itself). So a dead-lettered unit of work appears as **two** rows: a terminal `failed` row in the origin queue, and a new `created` row in the DLQ.

Queue Boss surfaces the model's 7th state, `DeadLetter`, by **deriving it at the adapter** (never reading a column that does not exist), using the **same predicate pg-boss uses**, so the two agree exactly. The origin-row bucketing is exhaustive over all `failed` rows:

```
state = 'failed' AND dead_letter IS NOT NULL AND dead_letter <> name  →  DeadLetter
state = 'failed' AND (dead_letter IS NULL OR dead_letter = name)      →  Failed
```

Applied as a `CASE` in both the `counts_by_state` aggregate and the `list_jobs`/`get_job` projections:

```sql
CASE WHEN state = 'failed' AND dead_letter IS NOT NULL AND dead_letter <> name
     THEN 'deadLetter' ELSE state::text END
```

So:
- **The sum invariant holds** — every `failed` row is either Failed or DeadLetter, never both and never neither (the two prose lines partition the `failed` set exhaustively); every non-`failed` row keeps its native state. Each job maps to exactly one bucket.
- **The teaching story is honest** — Failed = "terminally failed, nowhere to go"; DeadLetter = "terminally failed and routed to `<dead_letter>` for triage". The DLQ's own `created` copy shows up (correctly) as a Created job in that queue, and the queue is recognizable as a DLQ because its `name` appears in another queue's `dead_letter`.

(Note: `state = 'failed'` already implies retries were exhausted — `failJobs` only sets `failed` when `retry_count = retry_limit`, otherwise `retry` — so no separate `retry_count >= retry_limit` clause is needed; adding one would risk a non-exhaustive prose split.)

`Capabilities.dead_letter = true` for pg-boss, so the FE renders the DeadLetter bucket + the "routed to …" affordance; a backend without dead-letter would omit it.

### 3.5 Conformance split — static invariants vs the time-driven suite

E1's `assert_backend_conforms(&B, &ManualClock)` (`crates/core/src/conformance.rs:34`) **advances a fake clock** to force jobs Created→Active→(Completed|Failed→Retry→…). A real Postgres cannot be clock-driven — `PgBossBackend` reads whatever the seeded rows say — so E2-1 **splits** the suite rather than diluting it. The existing harness already partitions cleanly (`assert_queue_invariants` calls **only** `list_queues`; `assert_pagination`/`assert_state_filter`/`assert_timeline_ordered` call `list_jobs`/`get_job`; `assert_progression_over_time` is the clock-driven part), so E2-1 exposes **two composable clock-free halves plus a full entry point**:

- **`assert_queue_conformance(&B)`** — the **queue-level** clock-free invariants, which touch **only `list_queues`** (never `list_jobs`/`get_job`): `list_queues` returns **≥ 1** queue; each queue's `counts_by_state` **sums to `total_depth`**; `oldest_waiting_age` is `Some` **iff** the fixture seeded a due waiting job in that queue, else `None`.
- **`assert_job_conformance(&B)`** — the **job-level** clock-free invariants (these call `list_jobs`/`get_job`): `list_jobs` **cursor round-trips** — paginating the full set yields **no gaps, no dupes**, and `has_more == next_cursor.is_some()` at every page; **state-filter exactness** — filtering by a state returns exactly the jobs in that state; `get_job` **timeline is ordered** and every adjacent pair is a **valid transition** per the core `is_valid_transition` whitelist (**private to the conformance harness** — adapters cannot call it; the harness asserts it).
- **`assert_static_conformance(&B)`** — the **full** clock-free entry point = `assert_queue_conformance` **then** `assert_job_conformance` (unchanged behaviour, both halves).
- **`assert_backend_conforms(&B, &ManualClock)`** — **kept unchanged** for the sandbox; it calls `assert_static_conformance` first, then drives the state machine over simulated time (the invariant `assert_static_conformance` cannot express against a live DB).

Why two halves matter for the E2-2a↔E2-2b split: E2-2a implements `list_queues`/`capabilities`/`test_connection` while `list_jobs`/`get_job` are `todo!()` stubs, so its testcontainer test calls **`assert_queue_conformance`** (which never reaches the stubs); E2-2b implements the job methods and upgrades its test to the **full `assert_static_conformance`**. The sandbox conformance test runs **both** the static suite and the time-driven `assert_backend_conforms`; `PgBossBackend` runs only the clock-free entry points.

All three clock-free fns are `pub` at `qb_core::conformance`; the seeded pg-boss dataset is produced by a `pub` seed helper in `qb-backends` (used by the adapter's own conformance test and reusable by `src-tauri`'s integration test in E2-4).

### 3.6 Connection lifecycle — one pg-boss connection at a time + the sandbox, runtime-mutable

The PRD's F1 envisions *multiple saved connections*; the operator has **scoped E2 down** to **one active pg-boss connection at a time plus the always-present sandbox, switchable**, with **no saved-connection manager** (deferred — see §4, §5). This keeps E2 focused on the read path, not on credential-store CRUD and a connection library.

- E1's `AppState.backends: HashMap<ConnectionId, Arc<dyn QueueBackend>>` is **boot-frozen** (populated once in `build_app_state`, `src-tauri/src/lib.rs:87`, `.manage()` at `:101`). E2-4 makes it **interior-mutable** (`Mutex`/`RwLock` around the map) so backends can be added/removed at runtime. The sandbox stays registered under its fixed id; a pg-boss connection registers under a distinct `ConnectionId`.
- New commands: `connect_pgboss(config)` — builds a `PgBossBackend` over a fresh `PgPool`, runs `test_connection` (version-detect gate), persists credentials via the keychain (§3.7), registers the backend, and **starts its poll task**; `disconnect(connectionId)` — **aborts the poll task** (the `abort_task` scaffold at `state.rs:50` is the disconnect hook) and removes the backend. Existing commands already take `connection_id` (`lib.rs:28-84`), so they are unchanged.
- The E1 poller (`src-tauri/src/poller.rs`, `QueueCounts` payload `counts.rs:8`) is reused verbatim — a real pg-boss connection gets the **same** aggregate-counts-only stream (no per-job events), keyed by `connectionId`, torn down on disconnect. This is exactly the seam E1 built for (F9 near-live polling).

### 3.7 Real keychain via the `keyring` crate

E1 shipped `qb-platform`'s `SecretStore` as a **get-only** port with a `NoopSecretStore` (`crates/platform/src/lib.rs`; the error type is `SecretStoreError`, `:2`), and the crate is **UNWIRED** (not yet a dependency of `src-tauri/Cargo.toml`). E2-3:

- Extends the existing `SecretStore` trait with **`set`/`delete`** (it was get-only) and the existing `SecretStoreError` with any variants those need (no new error type);
- Adds an `OsSecretStore` backed by the **`keyring`** crate (the real OS Secret Service / macOS Keychain / Windows Credential Manager). **Critical:** a bare `keyring = "3"` dependency enables **no** OS backend — keyring defaults to a built-in **mock** store, which would make `OsSecretStore` silently never touch a real keychain (and pass both PR CI and the nightly). The dependency **must** enable real backends: `keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }`. Credentials are **never** written plaintext to disk (PRD F1);
- Adds an **in-memory fake** `SecretStore` for tests (a `Mutex<HashMap>` — **not** keyring's built-in mock; the fake exercises the trait seam directly);
- **Wires `qb-platform` into `src-tauri`** and threads a `SecretStore` into `AppState`.

**CI reality:** `keyring`'s real backends need an OS Secret Service, which is **absent on the headless ubuntu CI runner**. So unit/integration tests use the **in-memory fake** `SecretStore`, and the **real `OsSecretStore`'s OS behavior is exercised on the 3-OS nightly** (`xplat-build-smoke`), never on the Linux PR job. E2-3 must state which OS behavior the nightly actually exercised in its PR body.

### 3.8 Error sanitization — letting the version message through, sanitized

The core rule (`crates/core/src/error.rs:6`) is that a **raw driver/SQL string must never reach the UI**. E1's `CommandError{kind,message}` (`src-tauri/src/commands.rs:29`) maps `BackendError` with **generic** messages. E2 needs one carefully-scoped exception: the **`Unsupported` version message** ("pg-boss v10 required (schema versions 21–24); found schema v20") is *product copy*, not a driver string, and must reach the user. E2-4 adds a **message-carrying path** for `Unsupported` (and only sanitized, self-authored messages) through `CommandError` — a value the adapter constructs from the detected integer, **never** interpolating driver output. Connection/Internal errors stay generic. This is the only new outward-facing message channel.

### 3.9 Frontend — connect UI, per-connection status, active-connection rekeying, job explorer + detail

The E1 Angular layering is inherited unchanged: `src/app/core/tauri/queue-backend.service.ts` is the **sole** Tauri touchpoint; facades hold signal state; presentational components are dumb; presentational logic lives in pipes/directives (`ng-declarative-purity`).

**Pinned `PgConnectConfig` wire shape** (so E2-5's driver — which reads only spec + runbook — can author the `invoke('connect_pgboss', {config})` test without E2-4's source). Serialized **camelCase** on the wire (the Rust `PgConnectConfig` carries `#[serde(rename_all = "camelCase")]`; the TS model mirrors it 1:1). It is a **union** of a connection-string form OR discrete fields:

```ts
type PgConnectConfig =
  | { connectionString: string }                       // full libpq/URL string
  | { host: string; port: number; database: string;
      user: string; password: string;
      sslMode: string;      // e.g. "disable" | "prefer" | "require" | "verify-full"
      schema?: string };    // optional; omitted ⇒ backend defaults to "pgboss"
```

Field casing is exact (`connectionString`, `sslMode`, `database`, `user`, `password`, `schema`). In the discrete form every field except `schema` is required; `schema` is optional and defaults to `pgboss`. The command envelope is `invoke('connect_pgboss', { config })` and returns a `ConnectionId` (string). E2-4 owns the Rust `PgConnectConfig`; E2-5 owns the TS mirror of **this** shape.

- **E2-5 (connect + status + active-connection rekeying):** a **connect form** (connection string **or** the discrete fields above, per PRD F1) as a dumb component fed by a new **connections facade** exposing `connect`/`disconnect` intents; a **route** for it; **per-connection status** — E1's `ConnectionStatus` is a **single global signal** (`connection.facade.ts`), which E2-5 makes **keyed by `connectionId`** (connected/connecting/error per connection, always visible in the chrome); and an **active-connection selection signal** that **rekeys the overview/lifecycle** away from the hardcoded sandbox. Today `overview-container.component.ts:11` hardcodes `SANDBOX_CONNECTION_ID` (and connects the queues/connection facades to it at `:46-47`); E2-5 introduces a selected-connection signal so that, on a successful `connect_pgboss`, the overview/lifecycle + the queues facade **rekey to the new `connectionId`** and show **that** connection's live-polled counts (not the sandbox's). The interface service gains `connectPgboss`/`disconnect` wrappers over the new commands. The sandbox e2e is updated for connect-screen navigation.
- **E2-6 (job explorer + detail):** a **job-list** screen (calls `list_jobs`, keyset pagination + state/time/search filters, state-colored rows) and a **job-detail** screen (`get_job`; **capability-aware** rows that render the `extensions` map — `singletonKey`/`policy`/`priority` — only for keys the backend's `Capabilities.extensions` list declares), as dumb components + facades + pipes. The sandbox e2e is updated for job-explorer navigation. (The animated hero lifecycle stays in E3; E2-6 is the table/detail drill-down.)

FE gates: `ng-declarative-purity` (only the interface service imports `@tauri-apps/api`), `a11y-audit` (vitest-axe in jsdom for structure/labels/keyboard; color-contrast via the C8 real-webview / manual-record pattern), `tdd-evidence`.

### 3.10 Testing — testcontainers, feature-gated so plain `cargo test` needs no Docker

Adapter and conformance tests run against an **ephemeral Postgres** (via `testcontainers` / `testcontainers-modules`) seeded with a **pg-boss v10 schema + a dataset** spanning all six native states, a derived dead-letter case, a waiting queue, and a drained queue. **There is no Postgres in CI today**, and we do not want to force Docker on every local `cargo test --workspace`. So the PG integration tests are **gated behind a cargo feature** (`pg-integration`; a `PGBOSS_IT` env may additionally guard): plain `cargo test --workspace` skips them (no Docker needed), and **CI must explicitly enable them** — **E2-2a owns the CI wiring** (`.github/workflows/ci.yml`): the ubuntu `test` job runs `cargo test -p qb-backends --features pg-integration` (the `-p` form is required — `--features` at a virtual-workspace root errors) **and asserts the tests actually ran** (a sentinel that reddens CI if the feature is off — see runbook E2-2a). E2-4 extends the same CI wiring for its `src-tauri` gated integration test (`cargo test -p queue-boss --features pg-integration`). The seed is applied as raw pg-boss v10 DDL (from `plans.js`'s `create()` shape), not by running pg-boss itself.

## 4. Rejected alternatives

- **Seeded-dataset-only conformance (no split), or skipping conformance for pg-boss** — rejected. Skipping loses the whole point of the seam (the OSS contributor judges the adapter by its tests). Reusing the time-driven suite verbatim is impossible (a live DB can't be clock-advanced). Splitting out `assert_static_conformance` keeps a *shared, real* contract for `PgBossBackend` while preserving the strong time-driven suite for the sandbox. Chosen.
- **sqlx compile-time macros (`query!`/`query_as!`)** — rejected: they require a live DB or a checked-in offline cache **at build time**, coupling `cargo build`/CI to Postgres (which CI doesn't have) and pinning the build to one schema shape. Runtime `query_as` (+ the `derive` feature for `FromRow`) keeps the build DB-free and lets the flavor seam pick SQL at runtime. Chosen.
- **Restart-to-connect (keep `AppState.backends` boot-frozen)** — rejected: connecting to a database is the core E2 interaction; forcing an app restart per connection is user-hostile and breaks the "switchable, sandbox always present" model. Interior-mutable `AppState` chosen.
- **Session-only credentials (no keychain) / plaintext config** — rejected by the operator and the PRD (F1: "Secrets stored via the OS keychain … never plaintext on disk"). The operator explicitly chose **real keychain now** (`keyring` crate, real backends enabled) over deferring it, accepting the CI-headless wrinkle (fake store on Linux PR, real store on the nightly). Chosen.
- **Multiple saved connections + a connection manager in E2** — deferred. The PRD lists it P0 under F1, but the operator scoped E2 to **one live pg-boss connection at a time + the sandbox, switchable, no saved-connection store**. The saved-connection manager (naming, persistence, a connection library UI) is post-E2. The `connectionId`-keyed contract (from E1) means it is *additive* when it lands. (Deviation from the PRD, recorded in §5 and §7.)
- **v11 support in P0** — rejected as P0; v11 is a P1 stretch (E2-7) added *through* the version-detect seam. v11 (schema 25) changes job storage (a `job_common` table + optional partitioning vs v10's mandatory per-queue LIST partitions) and stats reads (cached `getQueueStats` vs live `countStates`/`getQueueSize`), so it is real work; but the 6-state enum and dead-letter semantics are stable across v10/v11, so the model mapping is largely shared. Cut-if-unstable.
- **`LISTEN/NOTIFY` push instead of polling** — rejected: pg-boss does not emit a read-friendly notification stream for arbitrary queue state, and E1's aggregate-count poller already satisfies F9. Push-based reads are out of scope.
- **Threading the queue name into `get_job` for partition pruning** — rejected in favor of a by-id parent-table lookup (keeps the core `JobId` queue-agnostic; uuid is globally unique; read-only detail lookups are human-paced). See §3.3.

## 5. Out of scope (this epic)

Multiple **saved** connections / a connection manager / cross-connection rollup (deferred; E2 is one live pg-boss connection + the sandbox); **any write path** — retry, cancel, delete, enqueue, or a read-only toggle (post-MVP; the app *is* read-only); `LISTEN/NOTIFY`; schema **older than v10** (v9 / schema ≤ 20 — surfaced as `Unsupported`, never supported); the **animated hero** lifecycle, teaching-annotation layer, oldest-waiting *prominence/UX*, throughput sparkline, light theme (E3); cross-platform packaging/signing, README, demo assets (E4). **v11** (schema 25) is present only as the P1 stretch child E2-7.

**Feature-flag policy:** N/A for this epic (as E1). The org Flagsmith gating policy targets the gangan product; Queue Boss is a standalone desktop MVP with no flag infrastructure and no live release to gate.

## 6. Children & dependency graph

Contract-first: the conformance split and the adapter precede the command wiring; the command layer precedes the FE; the keychain seam is independent and can land in parallel.

| # | Child | Blocked by | Prio | Gates |
|---|-------|-----------|------|-------|
| E2-1 | Conformance split (`assert_static_conformance`) | — | P0 | tdd-evidence, rust-mutation-coverage |
| E2-2a | `PgBossBackend` v10 **core** (test_connection/version-detect + list_queues + capabilities + seed + partial conformance + **CI enablement**) | E2-1 | P0 | tdd-evidence, rust-mutation-coverage, **xplat-build-smoke** |
| E2-2b | `PgBossBackend` v10 **jobs** (list_jobs + get_job; **full** `assert_static_conformance`) | E2-2a | P0 | tdd-evidence, rust-mutation-coverage |
| E2-3 | Real keychain `SecretStore` + wire `qb-platform` | — | P0 | tdd-evidence, rust-mutation-coverage, xplat-build-smoke |
| E2-4 | Runtime connect/disconnect commands | E2-2b, E2-3 | P0 | tdd-evidence, rust-mutation-coverage, xplat-build-smoke |
| E2-5 | FE connect UI + per-connection status + active-connection rekeying | E2-4 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |
| E2-6 | FE job explorer + job detail | E2-5 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |
| E2-7 | pg-boss **v11** support (stretch) | E2-2b | **P1** | tdd-evidence, rust-mutation-coverage |

**DAG:** `E2-1 → E2-2a → E2-2b`; `E2-3` is independent (parallel with E2-1/E2-2a/E2-2b); `{E2-2b, E2-3} → E2-4 → E2-5 → E2-6`; `E2-2b → E2-7` (P1, off the critical path). Eight children total. E2-3 can run first/in parallel; E2-2b and E2-3 join at E2-4.

Per-child recipes (files, schema/contract, TDD order, verification commands, gate notes) are in the companion runbook.

## 7. Risks / open questions

- **Schema-version bands (RESOLVED).** The integers are grounded in each tag's `version.json`: v9/older ≤ 20, v10 = **21–24** (floor 21), v11 = **25**. The seam maps 21–24 → v10, ≤ 20 → `Unsupported`, 25 → v11 (E2-7, else `Unsupported`). This closes the earlier "exact v11 floor" question — it is 25. Re-confirm 25 against the v11 container in E2-7 (belt-and-suspenders), but the band logic is fail-closed: anything above 24 is `Unsupported` until E2-7 explicitly routes it.
- **`get_job` without a queue.** The by-id parent-table lookup (§3.3) is correct (uuid is unique) but not partition-pruned; on a very large multi-partition `job` table this is a broad scan. Acceptable for a human-paced read-only detail view; revisit only if a real dataset shows it hurting (an optional `JobId` that carries the queue is the additive escape hatch).
- **Dead-letter derivation fidelity.** The predicate mirrors pg-boss's own `failJobs`→`dlq_jobs` clause (`state='failed' AND dead_letter IS NOT NULL AND dead_letter <> name`), reading pg-boss's denormalized `job.dead_letter`. A queue whose `dead_letter` config was changed *after* jobs were enqueued could show stale routing on old rows — a fidelity limitation of reading pg-boss's own column, not a Queue Boss bug. Documented, not fixed.
- **keyring silent-mock trap (mitigated).** A bare `keyring = "3"` links only the built-in mock, producing a green-but-fake `OsSecretStore`. Mitigated by pinning the real-backend features (`apple-native`/`windows-native`/`sync-secret-service`, §3.7) and by exercising the real store only on the 3-OS nightly (`xplat-build-smoke`). The residual risk is a real-keychain regression only the nightly catches — mitigated by keeping the nightly required-green.
- **PRD F1 deviation (multiple saved connections).** E2 ships **one** live pg-boss connection + the sandbox, not the PRD's multi-saved-connection F1. This is an explicit operator scope decision; the `connectionId` contract keeps the manager additive. Flag to the PRD owner so Milestone-2 "done" is judged against the scoped deliverable, not the full F1.
- **Testcontainers must actually run in CI.** The `test` job must have Docker available and E2-2a's `cargo test -p qb-backends --features pg-integration` step must execute; the sentinel (runbook E2-2a) reddens CI if the feature is off, so "silently skipped" cannot masquerade as green. E2-4 must keep the equivalent `-p queue-boss` step wired for its src-tauri integration test.
