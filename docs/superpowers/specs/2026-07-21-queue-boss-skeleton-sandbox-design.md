# Epic 1 — Skeleton & Sandbox — Design

**Product:** Queue Boss (getvoicify/Queue-Boss) — teaching-first, read-only desktop inspector for background job queues.
**Epic:** 1 of the MVP program (E1 Skeleton & Sandbox → E2 pg-boss v10 Read Path → {E3 Hero + Teaching, E4 Cross-platform Release}).
**Source PRD:** `queue-boss-mvp-prd.md` (Draft v0.2). This spec scopes **Milestone 1** of that PRD.
**Board:** org Project #3. **Planning:** self-hosted in this repo.

---

## 1. Problem this epic solves

The MVP's thesis is that *visualizing a queue's lifecycle* teaches background work better than a row dump. Before any of that can be proven against a real database, two foundations must exist and be trustworthy:

1. **A backend-agnostic architecture** whose adapter seam is clean enough that pg-boss is "the first adapter, not the only one" — the artifact the OSS contributor judges.
2. **A zero-infrastructure way to see a live queue** — the sandbox — which doubles as the *second* adapter, proving the abstraction is transport-agnostic *before* a real second backend exists.

Epic 1 delivers exactly these, plus the desktop shell and the Angular layering discipline every later epic inherits. It de-risks the UI and the abstraction with **no database dependency**.

## 2. Success criteria (epic is "done" when…)

- **Deliverable:** launching the app and choosing "Sandbox" shows a **live-updating fake queue** — per-queue depth and per-state counts that change on each poll — with **no database and no setup**.
- The Rust `QueueBackend` trait exists with **two-adapter validation deferred to nothing**: `SandboxBackend` passes a shared conformance suite that any future adapter will also run.
- The Angular layering is enforced by the `ng-declarative-purity` gate: presentational components are logic-free; all Tauri access is in one interface service; state lives in facades; presentational logic lives in directives/pipes.
- Every behavioral child shipped via outside-in TDD (`tdd-evidence` gate) with Rust logic covered under `rust-mutation-coverage`.
- CI is green on **macOS, Windows, and Linux**; the `claude-review` gate is enforcing from C2 onward.

## 3. Chosen architecture

### 3.1 Monorepo shape
Single repo. Tauri 2 shell hosts an Angular 22 SPA in the webview. SSR is **off** (desktop SPA). Layout:

```
/ (repo root)
  package.json, angular.json        # Angular workspace + tauri scripts (test:ci, lint, tauri:build, e2e)
  Cargo.toml                        # workspace root — [workspace] members = ["src-tauri", "crates/core", "crates/backends", "crates/platform"]
  src/                              # Angular app (SPA)
  src-tauri/                        # Tauri binary crate (wires the workspace crates)
    tauri.conf.json                # beforeDevCommand/devUrl/frontendDist → Angular
    Cargo.toml                     # binary-crate manifest (workspace member)
  crates/
    core/                          # package qb-core (lib qb_core): QueueBackend trait + domain model + Capabilities/extensions (platform-agnostic)
    backends/                      # package qb-backends: SandboxBackend now; PgBossBackend in E2
    platform/                      # package qb-platform: OS seams — SecretStore port trait + NoopSecretStore stub in E1 (real keychain E2)
  tests/e2e/                       # WebdriverIO + tauri-driver
  .github/workflows/               # ci.yml + claude-review.yml (C2)
```

**Decision:** Rust is a **Cargo workspace of three library crates** — `qb-core` (lib `qb_core`), `qb-backends`, `qb-platform`, living in dirs `crates/{core,backends,platform}` with the package `name` set per Cargo.toml — plus the `src-tauri` binary. Crate boundaries — not module convention — enforce the "core knows nothing about a specific backend or OS" rule: `qb-core` depends on neither `qb-backends` nor `qb-platform`; `qb-backends`/`qb-platform` depend on `qb-core`; `src-tauri` depends on all three and does the wiring. (The package prefix `qb-` avoids shadowing Rust's builtin `core` crate, which breaks `async-trait`/`thiserror` macro expansion in dependents.) This is the seam the OSS contributor evaluates.

### 3.2 The `QueueBackend` contract (`qb-core` crate)
Read-only in v1; write methods are intentionally absent and arrive with the post-MVP write path.

```rust
#[async_trait]
pub trait QueueBackend: Send + Sync {
    async fn test_connection(&self) -> Result<BackendInfo, BackendError>;
    async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError>;
    async fn list_jobs(&self, filter: JobFilter) -> Result<Page<JobSummary>, BackendError>;
    async fn get_job(&self, id: &JobId) -> Result<JobDetail, BackendError>;
    fn capabilities(&self) -> Capabilities;
    // job_action(...) intentionally omitted in v1 (read-only).
}
```

Domain model (core):
- `JobState` = `Created | Active | Completed | Failed | Cancelled | Retry | DeadLetter` (PRD F4 lifecycle; retry + dead-letter are first-class states). The enum itself carries **`#[serde(rename_all = "camelCase")]`** at the **ENUM level**, so its variants serialize camelCase (`created`…`deadLetter`) — the wire structs' *struct-level* `rename_all` renames struct FIELDS only and does **not** cover enum variants used as `countsByState` map keys, so this enum-level rename is what makes the map keys camelCase (the map-key-casing bug being fixed).
- `QueueSummary` = `{ name, total_depth, counts_by_state: Map<JobState,u64>, oldest_waiting_age: Option<Seconds> }`. **Oldest-waiting age is a first-class field**, not derived UI-side.
- `JobSummary` = `{ id, queue, state, created_at, started_at?, completed_at?, attempts, priority }`.
- `JobDetail` = `JobSummary + { data: Json, output: Json, timeline: Vec<TimelineEvent>, retry: RetryReadout, extensions: Map<String,Json> }`.
- `Capabilities` = feature flags the backend supports (`priority`, `singleton`, `dead_letter`, declared extension keys) — drives capability-aware rendering later.
- `Page<T>` = `{ items, next_cursor?, has_more }` (cursor-based, per repo pagination pattern).
- `JobFilter` = `{ queue?, states?, time_window?, search?, cursor?, limit }`.
- `BackendError` = typed error enum (connection, unsupported, not_found, internal) — never a raw driver string leaking to the UI.

**Extensions map** carries backend-specific fields (pg-boss `singleton_key`, `policy`, etc. in E2) so the model stays honest across backends without polluting the core types.

### 3.3 Invoke/event bridge + polling (src-tauri)
- Read methods are exposed as `#[tauri::command]` wrappers that resolve the active backend from Tauri managed `State` and delegate to the trait. Commands take a `connectionId` (E1: the fixed `"sandbox"` id) so the contract is stable when E2 adds multiple connections.
- A **per-connection background poll task** (Tokio) runs cheap grouped-count queries on an interval and **pushes** the results to the webview. **The UI is event-driven, not request-per-render.** (Streaming mechanism — global `emit` vs per-invoke `Channel` — is fixed in the runbook against verified Tauri 2 API; the design requirement is: aggregate counts pushed per poll, keyed by `connectionId`, torn down cleanly on disconnect.) **Default poll interval is 1000ms (configurable).**
- **`QueueCounts` payload (pinned contract).** The wire structs carry **`#[serde(rename_all = "camelCase")]`**, so the Rust snake_case fields serialize to **camelCase on the wire** and the TS models mirror those camelCase keys 1:1. Each poll pushes one `QueueCounts` snapshot with this shape: `{ connectionId, queues: [{ queue, totalDepth, countsByState, oldestWaitingAge }], polledAt }`. `countsByState` is a `JobState`→count map whose **keys are the camelCase `JobState` values** (`created`, `active`, `completed`, `failed`, `cancelled`, `retry`, `deadLetter`) — see the enum-level `#[serde(rename_all = "camelCase")]` in §3.2; `oldestWaitingAge` is seconds (nullable); `polledAt` is the snapshot timestamp as **epoch milliseconds (`u64`)** (the TS mirror uses `number`, ms). The **Angular TS models (C6/C7) mirror this section** — children read this spec, not each other's source.
- **Aggregate-rate friendliness:** counts are emitted as per-queue/per-state aggregates, never per-job events — this is the seam that lets E3's hero animation be driven by *rates*, not one sprite per job (PRD's top build risk).

### 3.4 Angular layers (strictly declarative)
Four layers, one direction of dependency:
1. **Tauri-interface service** — the **only** place that calls `invoke` or subscribes to Tauri events. Typed against the command contract. Converts events into observables/signals.
2. **Facade services** — hold presentation-facing state as **signals**; expose read-only signals + intent methods to components. No component touches the interface service directly.
3. **Presentational components** — **dumb**: inputs in (signals), outputs out, zero business logic, no injected data services, no `invoke`. Fed entirely by facades via a thin container binding.
4. **Directives & pipes** — absorb presentational logic (e.g. an `age`/relative-time pipe for oldest-waiting, a `stateColor` directive). Keep components lean.

### 3.5 Sandbox simulator (`qb-backends` crate)
`SandboxBackend` implements `QueueBackend` over an in-memory store driven by a synthetic producer/consumer: jobs are created, become active, then complete / fail-into-retry / occasionally dead-letter, at configurable rates. **Deterministic under an injected clock + seed** so tests are reproducible; free-running under a real clock in the app. It requires no DB and no setup — the onboarding/demo story and the abstraction's second proof.

### 3.6 Shell, chrome, theme
App chrome reads as an application (code-editor / observability-client), not a web page: a shell with a connection-status indicator region (per PRD F1, populated by the sandbox's `test_connection`), primary nav between Overview and a basic Lifecycle counts view, and the **dark theme as default**. Light theme is E3 (P1).

## 4. Rejected alternatives

- **Electron instead of Tauri** — rejected by the PRD (native, small, Rust core). Not reopened.
- **Single Rust crate, module-only split** — rejected: module boundaries are convention-only; crates enforce the core/platform/backends separation the OSS seam depends on.
- **NgRx/observ-heavy state** — rejected in favor of **signals + facades**: simpler, declarative, less boilerplate for a desktop SPA; matches the "lean, declarative" mandate.
- **Per-job events to the UI** — rejected: does not scale on busy queues and would couple the UI to job volume. Aggregate counts only.
- **Building the animated hero now** — deferred to E3. E1 shows live *counts* (numeric), which already proves the event pipeline without the animation risk.
- **Real keychain/secrets in E1** — deferred to E2 (no secrets needed for a read-only sandbox). The `qb-platform` crate ships the **port stubbed in E1**: a `SecretStore` trait with a `NoopSecretStore` implementation. The real keychain-backed implementation lands in E2.
- **Multi-connection management in E1** — deferred to E2. E1 has one implicit backend (sandbox) but a `connectionId`-shaped contract so E2 is additive.

## 5. Out of scope (this epic)

pg-boss adapter, connection management, keychain, schema/version detection (E2); animated lifecycle flow, teaching annotations, oldest-waiting *prominence/UX*, throughput sparkline, light theme (E3); cross-platform packaging/signing, README, demo assets (E4); any job mutation (post-MVP write path).

**Feature-flag policy:** N/A for this epic. The org Flagsmith gating policy targets the gangan product (gangan-api is the sole Flagsmith consumer); Queue Boss is a standalone desktop MVP with no flag infrastructure and no live release to gate. Revisit if Queue Boss ships user-visible features against a live user base post-GA.

## 6. Children & dependency graph

Contract-first; the Rust trait precedes adapters and the FE; the command contract precedes the facades.

| # | Child | Blocked by | Prio | Gates |
|---|-------|-----------|------|-------|
| C1 | Scaffold & TDD harness | — | P0 | tdd-evidence, rust-mutation-coverage, xplat-build-smoke |
| C2 | CI pipeline + `claude-review` gate + branch protection | C1 | P0 | xplat-build-smoke |
| C3 | Rust core: `QueueBackend` trait + domain model | C1 | P0 | tdd-evidence, rust-mutation-coverage |
| C4 | SandboxBackend adapter + shared conformance suite | C3 | P0 | tdd-evidence, rust-mutation-coverage |
| C5 | Tauri command + event/polling bridge | C3 | P0 | tdd-evidence, rust-mutation-coverage |
| C6 | Angular interface + facade layer | C5 | P0 | tdd-evidence, ng-declarative-purity |
| C7 | Angular presentational UI + chrome + dark theme | C6 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |
| C8 | End-to-end sandbox wiring + e2e test | C2, C4, C7 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit, xplat-build-smoke |

**DAG:** C1 → {C2, C3}; C3 → {C4, C5}; C5 → C6 → C7; {C2, C4, C7} → C8 (C5 reaches C8 transitively via C6→C7).

Per-child recipes (files, TDD order, verification commands, gate notes) are in the companion runbook.

## 7. Risks / open questions

- **Tauri e2e on macOS:** `tauri-driver` platform support is confirmed in the runbook; if macOS is unsupported, the C8 e2e runs on Linux/Windows CI and macOS relies on build-smoke + unit/integration coverage. (Resolved in runbook.)
- **Angular 22 test runner:** Karma is deprecated; the runbook pins the current default (Vitest-based) and the headless CI command.
- **Polling cost:** irrelevant against the in-memory sandbox, but the query shape is written now to be aggregate-count-based so E2's real pg-boss queries inherit the pattern.
- **Required-check names:** the `epic.yaml` build-matrix check names are provisional until C2's CI job names are final; reconcile before enabling branch protection.
- **Fork-PR review gap (documented; hardening deferred):** fork PRs skip `claude-review` via the workflow's job-level `if`; GitHub may treat the skipped required check as *satisfied*, so external fork PRs are **not** gated by the bot and — with `required_approvals: 0` — must be **manually reviewed by a maintainer and are NEVER driver-auto-merged**. Hardening (an always-run guard job, or requiring 1 human approval on fork PRs) is deferred to the OSS-contributor-flow work (E4+).
