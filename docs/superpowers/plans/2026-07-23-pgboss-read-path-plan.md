# Epic 2 — pg-boss v10 Read Path — Runbook

Companion to `docs/superpowers/specs/2026-07-23-pgboss-read-path-design.md`. One section per child. A driver session reads **only** its child issue + the spec + this runbook — each recipe is self-sufficient. Builds on the E1 skeleton ([design](../specs/2026-07-21-queue-boss-skeleton-sandbox-design.md) / [runbook](2026-07-21-queue-boss-skeleton-sandbox-plan.md)); the E1 seam, sandbox, conformance suite, Tauri poller, Angular layers, and the stubbed `SecretStore` port already exist.

## Global conventions (apply to every child)

- **Outside-in TDD is the gate, not a suggestion.** Write the failing test first, watch it fail for the right reason, minimal change to green, refactor green. Start at the outermost reachable layer; drop to a unit test only when the outer test can't reach the behavior. (`tdd-evidence` gate.)
- **Strictly declarative Angular.** Presentational components: inputs/outputs only, no injected data services, no `invoke`. All Tauri access in `queue-backend.service.ts`; state in signal facades; presentational logic in directives/pipes. (`ng-declarative-purity` gate.)
- **Read-only, always.** No `INSERT`/`UPDATE`/`DELETE`/`fetchNextJob`/migration ever issued against the user's database. Every SQL string the adapter's **query/mapping** modules run is a `SELECT`. (The `#[cfg(feature="pg-integration")]` **seed helper** is test-only and legitimately runs pg-boss v10 DDL — it is exempt; see the scoped grep in E2-2a/E2-2b Verification.) A write statement in a query module → park.
- **Raw driver strings never reach the UI.** Map every sqlx error to a typed `BackendError` (`crates/core/src/error.rs:6`); the **only** outward message is the self-authored `Unsupported` version copy (spec §3.8).
- **Minimal code comments — tests are the documentation of record.**
- **Conventional Commits, NO AI-attribution trailers** (no `Co-Authored-By: Claude`, no "Generated with Claude Code"). Squash-merge.
- **context-mode routing** — no raw curl/wget; route large command output through the sandbox.
- **Toolchain** (from `.claude/epic.yaml`): `npm run test:ci` · `npm run lint` · `npm run tauri:build` · `npm run e2e` · `cargo mutants --workspace`.
- **Per-PR CI gates on Linux only** (`.claude/epic.yaml` `merge.required_checks`): `claude-review`, `lint`, `test`, `build (ubuntu-latest)`, `e2e`. The **3-OS matrix is the nightly** (`.github/workflows/nightly-crossplatform.yml`) — `xplat-build-smoke` children (here: **E2-2a** and **E2-4** [both edit the CI workflow] and **E2-3** [platform crate]) must confirm the nightly is green (or trigger it via `workflow_dispatch`) before merge.

## Verified toolchain facts (2026)

Anchor every child to these; do not re-derive.

- **pg-boss schema is grounded in `plans.js`** — v10 from tag `10.3.2`, v11 from tag `11.0.1` (`src/plans.js`, `src/migrationStore.js`). Do **not** guess column names; the spec §3.3 table is the source of truth, transcribed from that DDL.
- **pg-boss v10 job states = SIX**, numeric-ordered enum: `created(0) < retry(1) < active(2) < completed(3) < cancelled(4) < failed(5)`. Waiting = `state < 'active'`. **There is no `deadLetter` state** — Queue Boss's 7th state is **derived** (spec §3.4).
- **`pgboss.version.version` is an integer schema version** (verified against each tag's `version.json`), not a semver:

  | release | schema | | release | schema |
  |---|---|---|---|---|
  | v9 / older | ≤ **20** | | v10.1.1 | **23** |
  | v10.0.0 | **21** | | v10.1.5 / v10.3.2 | **24** |
  | v10.0.6 | **22** | | v11.0.0 / v11.0.1 | **25** |

  **v10 supported band = 21–24 (floor 21).** Detect by reading the integer: `21–24` → v10 ok; `≤20` → `Unsupported` (v9/older); `25` → v11 (E2-7, else `Unsupported`); missing `version` table → `Unsupported`. Message: `pg-boss v10 required (schema versions 21–24); found schema vN`.
- **Dead-letter is queue-level**: `queue.dead_letter text REFERENCES queue(name)`. pg-boss's `failJobs`→`dlq_jobs` CTE inserts the DLQ copy `WHERE state = 'failed' AND dead_letter IS NOT NULL AND NOT name = dead_letter`. Queue Boss's DeadLetter derivation uses the **same** predicate: `state='failed' AND dead_letter IS NOT NULL AND dead_letter <> name` (no `retry_count` clause — `failed` already implies retries exhausted).
- **`job` is `PARTITION BY LIST (name)`** — one child partition per queue (`j<sha224(name)>`). `get_job` selects from the **parent** `pgboss.job` by `id` alone (uuid unique; no queue in the core signature). `list_jobs` keysets `(created_on, id) DESC`.
- **sqlx = runtime queries only** — `sqlx::query_as::<_, Row>(sql).bind(..).fetch_all(&pool)`; `#[derive(sqlx::FromRow)]` row structs. **Never** `query!`/`query_as!` macros (they need a build-time DB — CI has none). Cargo: `sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json", "derive"] }` — the **`derive`** feature backs `FromRow` (NOT `macros`, which is the checked-query feature we don't use).
- **testcontainers** — `testcontainers` + `testcontainers-modules` (postgres module) as **dev-dependencies** of `qb-backends` (and `src-tauri` for E2-4). Seed = raw pg-boss v10 DDL from `plans.js`'s `create()` shape (schema + `version` row [21–24] + `queue` + partitioned `job` + a per-queue partition), then `INSERT` fixture jobs. A `pub` seed helper lives in `qb-backends` so the adapter conformance test and the E2-4 command integration test share it.
- **PG integration tests are cargo-feature-gated** — behind `--features pg-integration` (a `PGBOSS_IT` env may additionally guard) so plain local `cargo test --workspace` needs **no Docker**. **`--features` is NOT allowed at a virtual-workspace root** — always scope with `-p`: `cargo test -p qb-backends --features pg-integration` (E2-2a), `cargo test -p queue-boss --features pg-integration` (E2-4). The CI ubuntu `test` job runs these (Docker present) and a **sentinel** proves they actually ran (E2-2a).
- **keyring** — the real `OsSecretStore` needs `keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }`. A **bare `keyring = "3"` enables the built-in MOCK** (no OS keystore) → a silently-noop store that passes CI *and* the nightly. Tests use an **in-memory fake `SecretStore`** (a `Mutex<HashMap>` implementing the trait — **not** keyring's mock); the real impl is exercised only on the **3-OS nightly**.
- **Core model facts** (verified against `crates/core/src/model.rs`): `Capabilities.extensions: Vec<String>` (`:178`) — **not** `extension_keys`. `RetryReadout` (`:142`) = `{ attempts: u32, max_attempts: Option<u32>, next_retry_at: Option<u64> }` (epoch-ms) — **no** backoff/delay field. `JobDetail.extensions: BTreeMap<String, Json>` (`:159`). `JobSummary.attempts: u32` (`:127`). The trait is `crates/core/src/backend.rs:12` (the `#[async_trait]` is at `:11`).
- **Tauri managed state mutation** — `AppState.backends` moves from a plain `HashMap` behind `.manage()` to an interior-mutable `Mutex`/`RwLock<HashMap<…>>`; commands lock briefly to resolve/register/remove. Poll-task teardown reuses E1's `AbortHandle` retention (`state.rs:50` `abort_task` is the disconnect hook).
- **Crate package names ≠ dir names** (unchanged from E1): dirs `crates/{core,backends,platform}`; packages `qb-core` (lib `qb_core`), `qb-backends`, `qb-platform`. `-p qb-core`, `use qb_core::conformance`. The `src-tauri` binary crate's package is **`queue-boss`** (`-p queue-boss`); scope Rust commands with `--workspace` or `-p`.
- **Angular 22 = Vitest** (`ng test --no-watch --no-progress`), **run under Node 24** (`nvm use 24`); `a11y-audit` = `vitest-axe` in jsdom (structure/labels/keyboard; **no** color-contrast — deferred to the real-webview / manual-record C8 pattern); e2e = WebdriverIO + `tauri-driver` (Linux+Windows) under xvfb on CI, owned by the C2 `e2e` job; the interface service is the **only** file importing `@tauri-apps/api`. E1 e2e `data-testid` convention: `enter-sandbox`, `queue-row`, `count-<queue>-<state>`.

---

## E2-1 — Conformance split (`assert_static_conformance`)  *(P0; blocked by: none)*

**Intent:** Extract the **clock-free** invariants from E1's `assert_backend_conforms` into **two composable halves + a full entry point** — `assert_queue_conformance` (queue-level, touches only `list_queues`), `assert_job_conformance` (job-level, calls `list_jobs`/`get_job`), and `assert_static_conformance` = both — so any backend (incl. a live DB) can be checked, **and E2-2a can run the queue half while its job methods are still `todo!()`**. Keep the time-driven suite for the sandbox. This unblocks the E2-2a↔E2-2b split without weakening coverage.

**Files/modules:** `crates/core/src/conformance.rs` (add the three `pub async fn`s; keep `assert_backend_conforms(&B, &ManualClock)` at `:34`); `crates/backends/tests/sandbox_conformance.rs` (now calls the full static suite **and** the time-driven one).

**Contract (three clock-free fns — a clean partition of the existing harness; spec §3.5):** the existing suite already groups this way — `assert_queue_invariants` (`:45`) calls **only** `list_queues`, while `assert_pagination`/`assert_state_filter`/`assert_timeline_ordered` call `list_jobs`/`get_job`, and `assert_progression_over_time` is the clock-driven part.
- **`pub async fn assert_queue_conformance<B: QueueBackend>(&B)`** — queue half (calls only `list_queues`): (1) `list_queues` returns **≥ 1** queue; (2) each queue's `counts_by_state` **sums to `total_depth`** (`QueueSummary::new` saturating sum, `model.rs:78/88`); (3) `oldest_waiting_age` is `Some` **iff** the backend has a due waiting job in that queue, else `None`.
- **`pub async fn assert_job_conformance<B: QueueBackend>(&B)`** — job half (calls `list_jobs`/`get_job`): (4) `list_jobs` cursor round-trip → **no gaps, no dupes**, `has_more == next_cursor.is_some()` at every page (`page.rs` keyset `{created_at,id}`); (5) **state-filter exactness** — `JobFilter{states:[s]}` returns exactly the jobs in state `s`; (6) `get_job` **timeline ordered** + every adjacent pair passes `is_valid_transition` (**private to `conformance.rs`** — the harness asserts it; adapters cannot call it).
- **`pub async fn assert_static_conformance<B: QueueBackend>(&B)`** — full entry point = `assert_queue_conformance(b).await; assert_job_conformance(b).await;`.

**TDD order (unit — refactor under green, TDD the *new* seams using the EXISTING harness machinery):**
1. **Red first for each half:** `conformance.rs` already carries the `Break` enum (`:381`) + the `assert_rejects(broken)` helper (`:700`) + an internal `broken`-configurable fake backend — reuse that (**not** `qb_core::testing::FakeBackend`, a fixed canned struct with no break-injection). Route the **queue-half** breaks (`Break::{NoQueues, BadDepth}`, plus a waiting/oldest-waiting break if not present) through a new `assert_queue_rejects(broken)` calling **`assert_queue_conformance`**, and the **job-half** breaks (`Break::{OffStateFilter, BrokenPaging, UnorderedTimeline, IllegalTransition}`) through `assert_job_rejects(broken)` calling **`assert_job_conformance`**; each asserts the half **rejects** its violation. Red proves each invariant is guarded by the correct half.
2. Refactor the existing clock-free asserts in `assert_backend_conforms` into the two halves; define `assert_static_conformance` as the pair; have `assert_backend_conforms` call `assert_static_conformance` first, then its time-driven work. `cargo test -p qb-core` → green (behavior preserved).
3. Update `sandbox_conformance.rs` to run the full static suite **and** the time-driven one; `cargo test -p qb-backends` → green.

**Verification:** `cargo test -p qb-core -p qb-backends` · `cargo clippy -p qb-core -- -D warnings` · `cargo fmt --check` · `cargo mutants -p qb-core --file crates/core/src/conformance.rs` (scope to the changed file).

**Gate notes:** `tdd-evidence` — the `assert_queue_rejects`/`assert_job_rejects` cases (each violated invariant must fail its half) are the evidence, not just "tests pass". `rust-mutation-coverage` — mutants on each invariant check (sum, waiting-iff, gap/dupe, `has_more`/cursor equivalence, transition validity) must be killed by a reject case; a survivor that flips an invariant means the harness is toothless — kill or justify.

**Done when:** all three fns (`assert_queue_conformance`, `assert_job_conformance`, `assert_static_conformance`) are `pub` at `qb_core::conformance` and form a clean partition (queue half never calls `list_jobs`/`get_job`), each invariant is guarded (proven via the `Break` reject machinery), the sandbox conformance test runs the full static + time-driven suites green, and mutants on `conformance.rs` are clean.

---

## E2-2a — `PgBossBackend` v10 core (test_connection + list_queues + capabilities + seed + CI enablement)  *(P0; blocked by: E2-1)*

**Intent:** Stand up `PgBossBackend` over `sqlx::PgPool`, **read-only**, with the version-detect seam, queue overview, capabilities, the testcontainers **seed helper**, and the **CI wiring** that runs the feature-gated PG tests. Pass **`assert_queue_conformance`** (E2-1's queue half — it touches only `list_queues`) against a seeded v10 container, with `list_jobs`/`get_job` as `todo!()` stubs. (If E2-2 had not been split, this is its first half; E2-2b completes it with the full `assert_static_conformance`.)

**Files/modules:** `crates/backends/src/pgboss/{mod.rs, queries.rs, rows.rs, map.rs, seed.rs}` (add `pub mod pgboss` to `crates/backends/src/lib.rs`); `crates/backends/tests/pgboss_conformance.rs`; **`.github/workflows/ci.yml`** (enable the PG tests in the ubuntu `test` job) + **`package.json`** (if a script alias helps — optional). Add to `crates/backends/Cargo.toml`: `sqlx` (features per Verified facts), `chrono`/`time`, `uuid`; dev-deps `testcontainers`, `testcontainers-modules` (postgres); a `pg-integration` feature guarding the container tests. (The CI sentinel keys off the **test name** `pg_integration_sentinel`, so no crate-level marker const is needed.)

**Contract / schema mapping (spec §3.3 + §3.4):**
- **`test_connection`** → `SELECT version FROM <schema>.version` (schema default `pgboss`). Integer **21–24** → `Ok(BackendInfo)` + resolve `SchemaFlavor::V10`; **≤20** / missing table → `BackendError::Unsupported("pg-boss v10 required (schema versions 21–24); found schema vN")`; **25** → `Unsupported` (until E2-7).
- **`list_queues`** → read `<schema>.queue` for the queue set + `dead_letter` route + `retry_*`/`policy`; aggregate per-state counts from `<schema>.job` with the DeadLetter `CASE`:
  ```sql
  SELECT name,
         CASE WHEN state = 'failed' AND dead_letter IS NOT NULL AND dead_letter <> name
              THEN 'deadLetter' ELSE state::text END AS qb_state,
         count(*) AS size
  FROM <schema>.job
  GROUP BY name, qb_state;
  ```
  Build `counts_by_state` (every job in exactly one bucket → sum invariant holds), `total_depth = Σ`, and `oldest_waiting_age`:
  ```sql
  SELECT EXTRACT(epoch FROM now() - min(start_after))::bigint
  FROM <schema>.job WHERE name = $1 AND state < 'active' AND start_after <= now();
  ```
  (`NULL` → `None`.) Queues in `queue` with no jobs → **drained** (all-zero, `oldest_waiting_age = None`).
- **`capabilities`** → `Capabilities { priority: true, singleton: true, dead_letter: true, extensions: vec!["singletonKey".into(), "policy".into(), "priority".into()] }` (the field is `extensions: Vec<String>`, `model.rs:178`).
- **`seed.rs`** (`pub`, `#[cfg(feature = "pg-integration")]`) → applies pg-boss v10 DDL (schema + `version=24` row + `queue` + partitioned `job` + per-queue partitions) then `INSERT`s a fixture spanning **all six native states** + a **derived dead-letter** case (an origin queue with `dead_letter='<dlq>'` holding a `failed` row where `dead_letter <> name`, plus the DLQ's `created` copy) + a **waiting** queue (due Created/Retry) + a **drained** queue (present, zero jobs).
- `list_jobs`/`get_job` are **E2-2b** — in E2-2a they are `todo!()` stubs so the crate compiles; `assert_queue_conformance` never calls them, so the queue-half test passes with the stubs in place.

**CI enablement (this child owns it):** in `.github/workflows/ci.yml`'s ubuntu `test` job, add a step **after** `npm run test:ci` — **`shell: bash` with `set -o pipefail`** so a RED `cargo test` fails the job instead of being swallowed by the pipe into `tee`:
```yaml
- name: pg-boss integration tests
  shell: bash
  run: |
    set -o pipefail
    cargo test -p qb-backends --features pg-integration -- --nocapture 2>&1 | tee pg_it.log
    grep -q 'pg_integration_sentinel ... ok' pg_it.log || { echo 'PG integration tests did not run (feature off?)'; exit 1; }
```
and a **sentinel** in the crate: `#[cfg(feature = "pg-integration")] #[tokio::test] async fn pg_integration_sentinel() { /* boot a container, seed, assert one seeded row reads back */ }`. With `pipefail`, a failing cargo run reddens the step directly; the `grep` additionally catches the "feature off → 0 tests ran" case. (`-p qb-backends` is required — `--features` errors at the virtual-workspace root.)

**TDD order (outside-in — conformance is the failing spec):**
1. Write `crates/backends/tests/pgboss_conformance.rs` (`#[cfg(feature = "pg-integration")]`): spin a Postgres testcontainer, run `seed::seed_v10(&pool)`, build `PgBossBackend::new(pool)`, call **`assert_queue_conformance`** (E2-1's queue half). **Stub every trait method with `todo!()`** so the crate **compiles** and the test **fails at runtime** (panics in `list_queues`) — red-for-the-right-reason, not a compile error. (`assert_queue_conformance` never calls the `list_jobs`/`get_job` stubs.)
2. Implement `test_connection` (version-detect) — unit-test the pure integer→flavor/`Unsupported` mapping fn with in-process cases (no container needed): 20→Unsupported, 21/22/23/24→V10, 25→Unsupported, missing→Unsupported. Red → green.
3. Implement `list_queues` (counts + DeadLetter `CASE` + `oldest_waiting_age`) + `capabilities` → green `assert_queue_conformance`.

**Verification:** `cargo test -p qb-backends` (no-Docker subset green) · `cargo test -p qb-backends --features pg-integration` (container `assert_queue_conformance` + sentinel green) · `cargo mutants -p qb-backends --features pg-integration --file crates/backends/src/pgboss/map.rs --file crates/backends/src/pgboss/queries.rs` · clippy · fmt. **Read-only grep scoped to the query/mapping modules** — `grep -nEi 'insert|update|delete|drop|alter|truncate' crates/backends/src/pgboss/{queries,map,mod}.rs` must be empty (`seed.rs` is test-only and **exempt** — it runs the DDL by design). **Confirm the nightly is green** (CI workflow change → `xplat-build-smoke`).

**Gate notes:** `tdd-evidence` — the failing container conformance test (via `todo!()` stubs) is the outer red; the version-mapping unit test is the inner red. `rust-mutation-coverage` — kill survivors on the DeadLetter `CASE`, the `oldest_waiting_age` predicate (`state < 'active' AND start_after <= now()`), and the version-band boundaries (20/21 and 24/25); justify any survivor. `xplat-build-smoke` — this child **edits `.github/workflows/ci.yml`** (`required_when: touches CI workflows`); confirm the latest nightly is green (or `workflow_dispatch` it) and note it in the PR body.

**Done when:** `PgBossBackend` passes **`assert_queue_conformance`** against the seeded v10 testcontainer with `list_jobs`/`get_job` still `todo!()` stubs (completed in E2-2b); version-detect maps 21–24→V10 and everything else→sanitized `Unsupported`; DeadLetter is derived; the CI job runs the PG tests under `pipefail` and the sentinel proves they ran; no write SQL in the query modules.

---

## E2-2b — `PgBossBackend` v10 jobs (list_jobs + get_job)  *(P0; blocked by: E2-2a)*

**Intent:** Complete the adapter — `list_jobs` (keyset pagination + filters) and `get_job` (detail/timeline/retry/extensions) — so `PgBossBackend` passes the **full** `assert_static_conformance` (adding the pagination/filter/timeline invariants) against the seeded v10 container.

**Files/modules:** `crates/backends/src/pgboss/{queries.rs, rows.rs, map.rs}` (fill the `todo!()` stubs); `crates/backends/tests/pgboss_conformance.rs` (upgrade to the **full** `assert_static_conformance`).

**Contract / schema mapping (spec §3.3):**
- **`list_jobs`** → keyset over `(created_on, id) DESC` from the parent `<schema>.job`; predicate `(created_on, id) < ($cursorCreatedAt, $cursorId)`, `LIMIT $limit`; `has_more` via fetch-one-past / EXISTS. `JobFilter` (`model.rs:192`): `queue?`→`name = $q`; `states?`→ filter on the DeadLetter-`CASE` projection; `time_window?`→`created_on` bounds; `search?`→`data @> $json` / payload text match. Project the DeadLetter `CASE` into `state`; `attempts = retry_count`.
- **`get_job`** → `SELECT … FROM <schema>.job WHERE id = $1 LIMIT 1` (parent table, by uuid; **no** queue — spec §3.3). Map to `JobDetail`: `data`, `output`; `timeline` from `created_on`→`started_on`→`completed_on` (state-labelled terminal, ordered, valid transitions per `is_valid_transition`); `RetryReadout { attempts: retry_count, max_attempts: Some(retry_limit), next_retry_at: (state==Retry ? start_after as epoch-ms : None) }` (**no** backoff/delay field — `model.rs:142`); `extensions` (`BTreeMap<String,Json>`) = `{ "singletonKey": singleton_key, "policy": policy, "priority": priority }` (+ `"deadLetter": dead_letter` route). Not found → `BackendError::NotFound`.

**TDD order (outside-in):**
1. Upgrade `pgboss_conformance.rs`: swap the `assert_queue_conformance` call for the **full** `assert_static_conformance` (queue half + job half). With `list_jobs`/`get_job` still `todo!()`, the queue half passes but the job half **panics** — red-for-the-right-reason on the pagination/filter/timeline invariants. Remove the stubs as you implement.
2. Implement `list_jobs` (keyset + filters) → green the pagination/filter/state-filter invariants.
3. Implement `get_job` (timeline/retry/extensions) → green the timeline invariant. Complete.

**Verification:** `cargo test -p qb-backends --features pg-integration` (full conformance green) · `cargo mutants -p qb-backends --features pg-integration --file crates/backends/src/pgboss/queries.rs --file crates/backends/src/pgboss/map.rs` · clippy · fmt. **Read-only grep scoped to `crates/backends/src/pgboss/{queries,map,mod}.rs`** must be empty (`seed.rs` exempt).

**Gate notes:** `tdd-evidence` — the full-conformance upgrade is the outer red. `rust-mutation-coverage` — kill survivors on the keyset comparison, the filter clauses, and the timeline/transition derivation (incl. the `next_retry_at`-only-when-Retry branch); justify any survivor. **Read-only** absolute.

**Done when:** `PgBossBackend` passes the **full** `assert_static_conformance` against the seeded v10 container; `RetryReadout`/`extensions` match the real `model.rs` shapes; no write SQL in the query modules; mutants clean.

---

## E2-3 — Real keychain `SecretStore` + wire `qb-platform`  *(P0; blocked by: none)*

**Intent:** Turn E1's get-only, unwired `SecretStore` port into a real OS-keychain-backed store with `set`/`delete`, plus an in-memory fake for tests, and **wire `qb-platform` into `src-tauri` + `AppState`**. Independent of the adapter — can land in parallel with E2-1/E2-2a/E2-2b.

**Files/modules:** `crates/platform/src/lib.rs` (extend the existing `SecretStore` trait with `set`/`delete`; extend the existing `SecretStoreError` enum — `:2` — with any needed variant; add `OsSecretStore` (keyring) + `InMemorySecretStore` fake); `crates/platform/Cargo.toml` (add `keyring` with real-backend features); `src-tauri/Cargo.toml` (add `qb-platform` dep — **it is not currently a dependency**); `src-tauri/src/state.rs` (thread a `SecretStore` into `AppState`).

**Contract (extend the EXISTING trait + error — do not introduce a new error type):**
```rust
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError>;
    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError>;   // NEW
    fn delete(&self, key: &str) -> Result<(), SecretStoreError>;             // NEW
}
```
Cargo: `keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }` — **a bare `keyring = "3"` links only the built-in MOCK** (no real keychain, green-but-fake). `OsSecretStore` maps to `keyring::Entry::new(service, key)` `set_password`/`get_password`/`delete_credential`; a missing entry on `get` → `Ok(None)` (not an error). `InMemorySecretStore` is a `Mutex<HashMap>` implementing the trait (**not** keyring's mock — it exercises the seam directly). **No secret value ever logged or in an error message.**

**TDD order (unit):**
1. Trait-level tests against `InMemorySecretStore`: `set` then `get` round-trips; `get` of an absent key → `Ok(None)`; `delete` then `get` → `Ok(None)`; `delete` of an absent key is a no-op `Ok(())`. Red → implement the fake → green. (These are the CI-safe contract — they run on the headless Linux job.)
2. `OsSecretStore` behind an env-gated / `#[ignore]`-by-default test so the pure logic is testable without a real Secret Service; the **real** OS round-trip is asserted only where a keychain exists (the nightly / a driver's machine), documented in the PR body.
3. Wire `qb-platform` into `src-tauri`: `AppState` holds `Arc<dyn SecretStore>`; a boot test asserts the app constructs with `OsSecretStore` in the real build and `InMemorySecretStore` in tests.

**Verification:** `cargo test -p qb-platform` · `cargo test --workspace` (wiring compiles + boots) · `cargo mutants -p qb-platform --file crates/platform/src/lib.rs` · clippy · fmt. **Confirm the latest nightly (`nightly-crossplatform.yml`) is green on all 3 OSes** (or `workflow_dispatch` it) — `keyring`'s real backends link platform-specific code.

**Gate notes:** `tdd-evidence`. `rust-mutation-coverage` — kill survivors on the get/set/delete branches and the absent-key `Ok(None)` path. `xplat-build-smoke` — `keyring` pulls **OS-specific** backends; the Linux PR job uses the **fake**, so the **real** impl's platform behavior is only proven on the nightly — **note in the PR body which OS behavior the nightly actually exercised**. A red nightly on any OS → park.

**Done when:** `SecretStore` has `set`/`delete` (on the existing trait/error), `OsSecretStore` (real-backend features) + `InMemorySecretStore` exist and are tested, `qb-platform` is a `src-tauri` dependency threaded into `AppState`, and the 3-OS nightly is green.

---

## E2-4 — Runtime connect/disconnect commands  *(P0; blocked by: E2-2b, E2-3)*

**Intent:** Make `AppState.backends` interior-mutable and add `connect_pgboss(config)` / `disconnect(connectionId)` commands that build/register/tear down a real `PgBossBackend` at runtime, store creds via the keychain, and let the **sanitized** unsupported-version message flow to the UI. Prove it with a `src-tauri` integration test that drives a real `PgBossBackend` (testcontainer) **through the command layer**.

**Files/modules:** `src-tauri/src/state.rs:13` (wrap `backends` in `Mutex`/`RwLock`; `build_app_state` `lib.rs:87` still seeds the sandbox; `.manage()` `lib.rs:101` unchanged); `src-tauri/src/commands.rs` (add `connect_pgboss`/`disconnect`; extend `CommandError` `:29` with the message-carrying `Unsupported` path — spec §3.8); `src-tauri/src/lib.rs` (register the new commands); reuse `poller.rs` + `counts.rs:8` unchanged; `src-tauri/tests/pgboss_command_integration.rs`; **`.github/workflows/ci.yml`** (extend E2-2a's PG-test step with `cargo test -p queue-boss --features pg-integration`, same `shell: bash` + `set -o pipefail`). `src-tauri/Cargo.toml`: add `qb-backends` container test-deps to dev-deps **and** a `[features] pg-integration = ["qb-backends/pg-integration"]` forwarding entry, so the gated integration test can call `qb_backends::pgboss::seed` (the seed helper is `#[cfg(feature="pg-integration")]`, so the feature must propagate to `qb-backends`).

**Contract:**
- `connect_pgboss(config: PgConnectConfig) -> Result<ConnectionId, CommandError>` — `PgConnectConfig` is the **exact wire shape pinned in spec §3.9** (Rust: `#[serde(rename_all = "camelCase")]`, a union of `{ connectionString }` OR `{ host, port, database, user, password, sslMode, schema? }`). Build a `PgPool`; run `test_connection`; on `Unsupported` return the **sanitized message** through `CommandError` (no driver string); on success, persist the password via `SecretStore::set` (keyed by connection), insert `Arc<PgBossBackend>` into `backends` under a new `ConnectionId`, spawn the poll task (retain its `AbortHandle`), return the id.
- `disconnect(connection_id) -> Result<(), CommandError>` — abort the poll task (`state.rs:50` `abort_task` hook), remove the backend, `SecretStore::delete` the cred. Disconnecting the **sandbox** id is rejected/no-op (the sandbox is always present).
- Existing `list_queues`/`list_jobs`/`get_job`/`test_connection` commands (each already `connection_id`-keyed, `lib.rs:28-84`) now resolve against the mutable map; unknown id → typed error.

**TDD order (outside-in — command integration first):**
1. `src-tauri/tests/pgboss_command_integration.rs` (`#[cfg(feature = "pg-integration")]`): seed a v10 container (reuse `qb_backends::pgboss::seed`), invoke `connect_pgboss` through the command layer with an `InMemorySecretStore`, assert it returns an id and that `list_queues(id)` returns the seeded queues with correct counts; then `disconnect(id)` and assert the backend is gone + the poll task stopped. Red (no commands).
2. Inner unit tests: `connect_pgboss` over a **fake** backend / injected pool builder — success registers + spawns; `Unsupported` propagates the **sanitized** message and registers nothing; `disconnect` aborts + removes + deletes the cred; concurrent connect/disconnect don't deadlock the lock. Red → green.
3. Wire the `CommandError` `Unsupported` message path (assert a raw SQL string is **never** in the surfaced message).

**Verification:** `cargo test --workspace` · `cargo test -p queue-boss --features pg-integration` (command integration green) · `cargo mutants -p queue-boss --file src-tauri/src/commands.rs --file src-tauri/src/state.rs` · clippy · fmt.

**Gate notes:** `tdd-evidence` — the container-through-commands test is the outer red. `rust-mutation-coverage` — kill survivors on the connect success/failure branches, the `Unsupported` message path, the disconnect teardown (task abort + map removal + cred delete), and the sandbox-protection guard. `xplat-build-smoke` — this child **edits `.github/workflows/ci.yml`** (adds the `-p queue-boss --features pg-integration` step) → `required_when: touches CI workflows`; confirm the latest nightly is green (or `workflow_dispatch` it) and note it in the PR body. Reuse E1's aggregate-counts-only poller — **no per-job events**.

**Done when:** a real `PgBossBackend` connects and disconnects through the command layer at runtime (proven against a container, CI step extended to `-p queue-boss`), the sandbox is untouched, unsupported versions surface a sanitized message, and creds live in the keychain.

---

## E2-5 — FE connect UI + per-connection status + active-connection rekeying  *(P0; blocked by: E2-4)*

**Intent:** A connect form + a connections facade (connect/disconnect intents) + a route + **per-connection** status (E1's global `ConnectionStatus` becomes keyed by `connectionId`) + an **active-connection selection signal** that rekeys the overview/lifecycle off the hardcoded sandbox. (PRD F1 — the scoped, one-live-connection-plus-sandbox form.) **Run under Node 24 (`nvm use 24`).**

**Files/modules:** `src/app/features/connect/*` (dumb connect-form component: connection-string **or** discrete host/port/db/user/password/SSL-mode/schema fields, validation via a pipe/validator, submit emits an output); `src/app/core/facades/connections.facade.ts` (signals: `status` map keyed by `connectionId`, an **`activeConnectionId`** signal, `connect(config)`/`disconnect(id)` intents); update `src/app/core/facades/connection.facade.ts` (`ConnectionStatus` single global signal → **per-connectionId** map); `src/app/features/overview/overview-container.component.ts` (replace the hardcoded `SANDBOX_CONNECTION_ID` at `:11`/`:46-47` — bind the queues + connection facades to the **`activeConnectionId`** signal so overview/lifecycle follow the selected connection); `src/app/core/tauri/queue-backend.service.ts` (add `connectPgboss(config)`/`disconnect(id)` over the new commands — **still the only file importing `@tauri-apps/api`**); `src/app/app.routes.ts` (connect route); `src/app/shell/*` (status region renders per-connection status); update `tests/e2e/sandbox.e2e.ts` for connect-screen navigation. TS `PgConnectConfig` mirrors **spec §3.9**, not sibling source. **`data-testid`s** (E1 convention): `open-connect` (nav affordance), `connect-form`, `connect-mode-toggle`, `connect-submit`, `connection-status-<connectionId>`.

**TDD order:**
1. Interface service (mock `@tauri-apps/api` `invoke`): `connectPgboss(config)` calls `invoke('connect_pgboss', {config})` and returns the id; `disconnect(id)` calls `invoke('disconnect', {connectionId:id})`; error payloads surface the sanitized message. Red → green.
2. `ConnectionsFacade` over a mocked interface service: `connect` transitions status `connecting → connected` (or `→ error` with the message) for **that** id only, and sets `activeConnectionId` to the new id on success; `disconnect` clears it (and falls back active to sandbox). Red → green.
3. Overview container: asserts it reads counts for `activeConnectionId` (not a hardcoded sandbox id) — connecting a pg-boss id rekeys the rendered counts. Red → green.
4. Connect-form component: given inputs, renders both entry modes, emits a valid config on submit, disables submit while connecting — **pure, no service calls, no `invoke`**. `vitest-axe` (labelled fields incl. a **typed password field**, keyboard-reachable). Red → green.
5. Extend the sandbox e2e: launch → `open-connect` → assert the `connect-form` + the sandbox's status chip coexist.

**Verification:** `ng test --no-watch --no-progress` (incl. `vitest-axe`, Node 24) · `npm run lint` · `npm run e2e` (Linux CI job).

**Gate notes:** `tdd-evidence`. `ng-declarative-purity` — the form is dumb (inputs/outputs only); all Tauri access stays in `queue-backend.service.ts` (the gate greps for stray `invoke`/`@tauri-apps/api`); status + `activeConnectionId` are facade signals. `a11y-audit` — `vitest-axe` (jsdom) for the form's labels/keyboard; **password field must be labelled + typed**; color-contrast deferred to the real-webview/manual record.

**Done when:** a user can navigate to Connect, submit a config, see per-connection status (connecting/connected/error with the sanitized message) alongside the always-present sandbox, and **the overview/lifecycle show the connected pg-boss connection's live-polled counts (not the sandbox's)**; only the interface service touches Tauri; axe clean.

---

## E2-6 — FE job explorer + job detail  *(P0; blocked by: E2-5)*

**Intent:** The read-path drill-down — a job-list screen (`list_jobs`, keyset pagination + state/time/search filters) and a job-detail screen (`get_job`, capability-aware extension rows). Dumb components + facades + pipes. (PRD F5 + F6; the animated hero stays in E3.) **Run under Node 24 (`nvm use 24`).**

**Files/modules:** `src/app/features/jobs/{job-list,job-detail}/*` (dumb components: list = state-colored rows [id, state, created, started/completed, attempts, priority] with a "load more"/cursor affordance + filter inputs; detail = data/output JSON viewers, timeline, retry/backoff readout, capability-aware extension rows); `src/app/core/facades/jobs.facade.ts` (signals: page, filter, selected job; `loadPage(cursor)`/`setFilter(f)`/`select(id)` intents over the interface service, keyed by `activeConnectionId`); `src/app/core/tauri/queue-backend.service.ts` (`listJobs(filter)`/`getJob(id)` wrappers — extend if needed); `src/app/shared/pipes/*` (reuse `stateColor`, add a `jsonPreview` pipe and an `attempts` pipe rendering "N of M" from `RetryReadout` `attempts`/`max_attempts`); `src/app/shared/directives/*`; `src/app/app.routes.ts` (jobs + detail routes, master–detail); update `tests/e2e/sandbox.e2e.ts` for job-explorer navigation. **Capability-aware rows**: render an extension row only if the connection's `Capabilities.extensions` list declares its key (spec §3.9). **`data-testid`s** (E1 convention): `queue-row` (reuse), `job-row`, `jobs-load-more`, `job-filter-state`, `job-detail`, `job-extension-<key>`.

**TDD order:**
1. Interface service: `listJobs(filter)` → `invoke('list_jobs', {filter})` maps to `Page<JobSummary>`; `getJob(id)` → `invoke('get_job', {id})` maps to `JobDetail` incl. the `extensions` map + `RetryReadout` (`attempts`/`maxAttempts`/`nextRetryAt`). Red → green.
2. `JobsFacade` over a mocked interface service: `loadPage` appends the next keyset page and tracks `hasMore`; `setFilter` resets pagination; `select(id)` loads detail. Red → green.
3. Components: job-list renders rows from an input signal + emits filter/next/select outputs (pure); job-detail renders timeline + retry readout ("N of M" via the `attempts` pipe) + **only the declared** extension rows from inputs. `vitest-axe` on both (table semantics, row keyboard-reachability, labelled controls). Red → green.
4. Pipes/directives unit-tested (`attempts` "N of M", `jsonPreview`, `stateColor`).
5. Extend the sandbox e2e: navigate to the job explorer (`job-row`), assert rows render and a `job-detail` opens.

**Verification:** `ng test --no-watch --no-progress` (incl. `vitest-axe`, Node 24) · `npm run lint` · `npm run e2e`.

**Gate notes:** `tdd-evidence`. `ng-declarative-purity` — list/detail are dumb (inputs/outputs); pagination/filter/selection state lives in `JobsFacade`; no `invoke` outside the interface service; presentational logic in pipes/directives. `a11y-audit` — `vitest-axe` (jsdom) for table semantics + keyboard reachability of rows/filters/detail; contrast deferred to the real-webview/manual pattern.

**Done when:** a user browses paginated/filtered jobs against the active pg-boss connection and opens a job detail showing payload/output/timeline/retry readout ("N of M") + capability-aware extension rows (singletonKey/policy/priority); components dumb; axe clean.

---

## E2-7 — pg-boss **v11** support (stretch)  *(P1; blocked by: E2-2b)*

**Intent:** Additively extend the version-detect seam to route **v11** (schema 25) query mapping, proven against a v11 testcontainer. Off the critical path — **cut if unstable**; a cut v11 simply reports `Unsupported`, leaving the P0 deliverable intact.

**Files/modules:** `crates/backends/src/pgboss/{mod.rs, queries.rs, map.rs}` (add a `SchemaFlavor::V11` arm + v11 query variants; do **not** touch the v10 path); `crates/backends/src/pgboss/seed.rs` (add `pub fn seed_v11`); `crates/backends/tests/pgboss_v11_conformance.rs`.

**Contract (v10↔v11 diff — grounded in `11.0.1` `plans.js`; spec §3.1/§4):**
- **Same** 6-value `job_state` enum + `dead_letter` semantics → **the DeadLetter derivation and the model mapping are shared** (no model change).
- **Version:** v11 baseline schema integer = **25** (verified against `11.0.0`/`11.0.1` `version.json`). Set the seam's v10/v11 boundary from this (≤24 → V10, 25 → V11).
- **Job storage differs:** v11 introduces a **common `job_common` table** with **optional** per-queue partitioning (`queue.partition bool default false`), vs v10's **mandatory** `PARTITION BY LIST (name)` per-queue partitions. v11 queue enumeration/count reads must target the v11 layout (`getQueueStats`/cached stats) instead of v10's live `countStates`/`getQueueSize`.
- **Column renames:** v11 `queue.retention_minutes` → `retention_seconds`, adds `deletion_seconds`/`retry_delay_max`/`warning_queued`, drops `partition_name`; v11 `version` table is slimmer (`version int PK, cron_on tz` — no `maintained_on`/`monitored_on`).

**TDD order (outside-in):**
1. `pgboss_v11_conformance.rs` (`#[cfg(feature = "pg-integration")]`): seed a v11 container, build `PgBossBackend` (flavor auto-detected as V11 from schema 25), call `assert_static_conformance`. Red.
2. Add the `V11` flavor arm + v11 query variants + `seed_v11` → green, **without regressing the v10 conformance test**.
3. Unit-test the v10-vs-v11 boundary in the version-mapping fn (24→V10, 25→V11).

**Verification:** `cargo test -p qb-backends --features pg-integration` (**both** v10 and v11 conformance green) · `cargo mutants -p qb-backends --features pg-integration --file crates/backends/src/pgboss/map.rs` on the flavor routing · clippy · fmt.

**Gate notes:** `tdd-evidence`, `rust-mutation-coverage` — kill survivors on the flavor-routing branch (24/25 boundary) and the v11 count/stat reads. **Read-only** still absolute (query modules; `seed.rs` exempt).

**Done when:** version-detection routes v10 vs v11 (24→V10, 25→V11), both flavors pass static conformance against their own containers, and the v10 path is byte-for-byte unchanged. (If cut: v11 connections report the sanitized `Unsupported` and this child is parked to a fast-follow.)

---

## Drive order

`E2-1 → E2-2a → E2-2b` on the adapter spine; `E2-3` in **parallel** (independent). `{E2-2b, E2-3} → E2-4` (the join) `→ E2-5 → E2-6` on the FE spine. `E2-7` (P1) hangs off `E2-2b`, off the critical path, cut-if-unstable. Land E2-3 early so the nightly `xplat-build-smoke` signal is available well before the FE work; E2-2a's and E2-4's CI edits also trip `xplat-build-smoke`, so confirm the nightly on each. Eight children total.
