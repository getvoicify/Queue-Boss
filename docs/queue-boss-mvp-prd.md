# Queue Boss — MVP Product Requirements Document

**Status:** Draft v0.2 (brainstorm decisions incorporated)
**Product:** Queue Boss — an open-source, teaching-first desktop inspector for background job queues
**Platform:** Native desktop (Tauri 2 + Angular 22), Rust backend
**MVP target backend:** pg-boss (Postgres), **v10+ only**

---

## 1. Summary

Queue Boss is a native desktop application that lets developers see and understand their background job queues. The MVP focuses on **pg-boss**, but the product's north star is a **generic pane of glass** for queues of any kind, with a teaching-first bent: the primary goal is to make someone who has never thought in queues *comfortable* with them.

The MVP's job is to prove one thesis: **that visualizing a queue's lifecycle — rather than dumping its rows into a table — is a materially better way to understand and debug background work.** Everything in scope serves that thesis or the minimum needed to make it usable.

**The MVP is strictly read-only.** It inspects and explains; it never mutates the queue.

---

## Key decisions (resolved in brainstorm)

1. **MVP is read-only.** No write path ships in v1 — no retry, cancel, delete, or enqueue. This de-risks the release, eliminates version-correct mutation SQL, and removes the need for a read-only toggle (the app simply *is* read-only). Write actions are the first post-MVP addition, and the architecture is designed to accommodate them.
2. **The lifecycle flow animation is P0.** Jobs visibly moving between states is the core draw for newcomers and carries the teaching thesis, so the animated diagram is required to ship — not a polish item.
3. **The sandbox is an in-memory simulator.** A zero-setup synthetic queue with no database required. It doubles as the second adapter implementation, proving the abstraction is transport-agnostic before a real second backend lands.
4. **pg-boss v10+ only.** Supporting the older v9 schema is out of scope for the MVP to avoid doubling every query path. This floor is stated clearly in the README and surfaced in-app when a user connects to an older or unrecognized schema.
5. **Multiple saved connections.** Users can save and switch between several connections, each with its own live poller. A unified cross-connection rollup dashboard is out of scope — you view one connection at a time, but you are not limited to one.

---

## 2. Problem

Background job queues are operationally critical and conceptually opaque. The tools that exist fall into two camps, and neither serves a newcomer:

- **Generic database GUIs** (DBeaver, pgAdmin, TablePlus) can read pg-boss's tables, but they present rows, not a queue. They expose no lifecycle, no semantics, no notion of "why is this job stuck." You have to already understand the system to use them.
- **Framework-specific dashboards** (Bull Board, Sidekiq web UI) are closer, but they're bolted to one library, assume you already think in queues, and are typically embedded web UIs rather than a real tool you keep open.

Nobody teaches the model while showing you the data. A junior engineer who inherits a queue, or a team adopting pg-boss for the first time, has no on-ramp.

---

## 3. Goals & non-goals

### Goals (MVP)

- Let a user connect to a pg-boss database and immediately understand the state of their queues.
- Make the **job lifecycle** the primary lens, so users learn the model by watching it move.
- Surface the metrics that build correct intuition — especially **latency** (how long work waits), not just volume.
- Stay strictly **read-only** — inspect and explain without ever mutating the user's queue.
- Provide a **zero-infrastructure sandbox** so anyone can experience a live queue with no database of their own.
- Establish a backend-agnostic internal architecture so pg-boss is the first adapter, not the only possible one.

### Non-goals (explicitly deferred)

- **Any job mutation** — retry, cancel, delete, or enqueue. v1 is strictly read-only; write actions are the first post-MVP addition.
- **Multi-backend support** (BullMQ, SQS, Sidekiq). The abstraction is designed for it; no second real adapter ships in the MVP beyond the sandbox simulator.
- **pg-boss v9 support.** v10+ only for the MVP.
- **Cross-connection rollup dashboards.** You can save and switch between multiple connections, but there is no unified cross-queue/cross-connection view.
- **Alerting / background notifications.** Tempting on desktop, but scope creep for an inspector MVP.
- **Teams, auth, roles, sharing, cloud sync.** It's a local tool.
- **Historical analytics / long-range trend charts.** Near-live state only.

---

## 4. Target users

**Primary — The Learner.** A junior/mid engineer who has just started working with a queue (inherited a service, joined a team using pg-boss, or is learning background processing). Needs orientation, plain language, and a safe place to look around. Success = "I now understand what my queue is doing."

**Secondary — The Debugger.** An experienced engineer with a queue misbehaving in staging or prod. Needs to find the stuck/failed jobs fast and see payloads and errors. Success = "I found the problem faster than I would have in psql." This user drives adoption and repo stars.

**Stakeholder — The OSS contributor.** Will judge the codebase by how clean the adapter seam is. Not a user of the UI per se, but the architecture must invite contribution.

---

## 5. Success metrics

Because this is open source, metrics blend adoption and qualitative signal:

- **Time to first insight:** from launching the app to correctly reading your queue's state. Target: under 2 minutes for someone with a connection string; ~10 seconds for the sandbox.
- **Sandbox engagement:** share of first sessions that open the sandbox. It's the top of the funnel for the teaching mission.
- **Connect success rate:** share of connection attempts that succeed against real pg-boss v10+ instances without an error we should have handled.
- **Adoption signal:** GitHub stars, releases installed, and issues that read like "this helped me understand X" rather than only bug reports.
- **Safety by construction:** because v1 cannot mutate anything, accidental production changes are impossible — the guardrail is the architecture itself. (An explicit safety metric returns when the write path lands.)

---

## 6. Scope — feature list (prioritized)

**P0 = must ship for MVP. P1 = ship if cheap, otherwise fast-follow.**

| # | Feature | Priority |
|---|---------|----------|
| F1 | Connection management (multiple saved connections, switch between them) | P0 |
| F2 | Schema & version detection (pg-boss v10+ only) | P0 |
| F3 | Queue overview (list, depth, per-state counts, oldest-waiting age) | P0 |
| F4 | Lifecycle view — **animated** live state diagram (the hero) | P0 |
| F5 | Job explorer (filter, search, virtual scroll) | P0 |
| F6 | Job detail (payload, output, timeline, retry/backoff readout) | P0 |
| F7 | Teaching layer (plain-language annotations) | P0 |
| F8 | Sandbox mode (in-memory simulated live queue, no DB) | P0 |
| F9 | Near-live polling engine | P0 |
| F10 | Basic throughput sparkline per queue | P1 |
| F11 | Light theme (dark is default) | P1 |

---

## 7. Feature detail

### F1 — Connection management
- Connect to a Postgres instance via connection string or discrete fields (host, port, db, user, password, SSL mode, optional schema name — pg-boss defaults to `pgboss` but is configurable).
- **Multiple saved connections.** Users create, name, save, and switch between several connections. Each active connection runs its own poller; the UI focuses one connection at a time (no cross-connection rollup).
- Secrets stored via the OS keychain through Tauri — never plaintext on disk.
- **All access is read-only in v1** — there is no write mode and therefore no read-only toggle; the app is read-only by design.
- Connection status indicator (connected / connecting / error) always visible in the app chrome, per connection.

### F2 — Schema & version detection (v10+ only)
- On connect, locate the pg-boss schema and read `pgboss.version` to confirm the schema generation.
- **The MVP supports pg-boss v10 and later only.** v10 made queues first-class (`pgboss.queue`), added dead-letter configuration, and partitions the job table per queue; targeting a single schema generation keeps the query layer tractable for v1.
- If the user connects to a v9 (or older/unrecognized) schema, **fail gracefully with a clear, plain-language message** stating that v10+ is required — never a raw SQL error. This requirement is also documented prominently in the README.

### F3 — Queue overview
- List all queues with, per queue: total depth, a breakdown of per-state counts, and **age of the oldest waiting job**.
- **Oldest-waiting age is first-class**, shown with equal or greater prominence than depth. Teaching users to read latency over volume is a core pedagogical goal: a 10k backlog draining fast is healthy; a 50-job backlog stuck for an hour is broken.
- Clicking a queue drills into its lifecycle view and scopes the job explorer.

### F4 — Lifecycle view (hero, animated)
- A live state-machine visualization of `created → active → completed | failed | cancelled`, with `retry` and `dead-letter` as distinct states.
- Each node shows its **live count** for the selected queue; counts update on each poll.
- **The flow animation is required.** Jobs visibly move along the edges between states as the queue works — this is the single biggest draw for newcomers and the clearest expression of the teaching thesis. (See the animation-performance note in Risks: motion should be driven by aggregate rates, not one sprite per job, so it stays smooth on busy queues.)
- **Teaching annotations (F7)** attach here: hovering/selecting a state explains, in human terms, what it means and why jobs land there.

### F5 — Job explorer
- A dense, virtual-scroll-ready table of jobs, scoped by queue and filterable by state and time window.
- Search within the job `data` payload (JSON containment / text match).
- State-colored rows using the palette. Columns: id, state, created, started/completed, attempts, priority.
- Selecting a row opens the job detail panel (master–detail layout).

### F6 — Job detail
- Full `data` payload (JSON viewer) and `output` (result or error).
- **Lifecycle timeline** for the job: created → started → completed/failed, with timestamps.
- **Retry/backoff readout** (read-only): attempt N of M, next scheduled run, backoff strategy — the "why is this job here?" answer in concrete terms. This *describes* pg-boss's scheduled behavior; it does not trigger anything.
- **Capability-aware detail rows** render backend-specific fields (pg-boss `singleton_key`, `priority`, `policy`) via the extensions mechanism, so the panel stays honest across future backends.

### F7 — Teaching layer
- Plain-language annotations throughout: every state and every non-obvious field carries a human explanation available on hover/inspect.
- Written for someone who does not yet think in queues. This is a differentiator, not documentation — it lives in the UI, not a separate help page.

### F8 — Sandbox mode
- A **simulated, in-memory queue backend** implementing the same adapter interface as pg-boss, driven by a synthetic producer/consumer that generates jobs which succeed, fail into retry, and occasionally dead-letter.
- Requires **no database and no setup** — launch the app, enter the sandbox, watch a real-behaving queue immediately.
- Doubles as (a) the onboarding/demo story and (b) the second adapter implementation, which validates that the abstraction is genuinely transport-agnostic *before* a real second backend (e.g. BullMQ) is attempted.

### F9 — Polling engine
- pg-boss does not emit `LISTEN/NOTIFY` on job state changes, so the MVP is **near-live via polling**, not real-time.
- A background Tokio task per active connection runs cheap grouped-count queries on an interval and emits Tauri events to the Angular layer; the UI is event-driven, not request-per-render.
- Poll interval is configurable with a sensible default; queries are written to be efficient against v10's partitioned job table.

---

## 8. Technical architecture (MVP)

- **Shell:** Tauri 2. Angular 22 SPA in the webview; Rust core for all data access and long-running tasks.
- **Frontend ↔ backend:** Angular calls Rust `#[tauri::command]` functions via `invoke`; the backend pushes updates as Tauri events. No queue database credentials or query logic live in the webview.
- **Data access:** `sqlx` against Postgres using **runtime `query_as`**, not compile-time `query!` macros — the schema lives in the *user's* database, not ours at build time.
- **Read-only model:** every access path is read-only SQL against the user's database; the app has no code path that writes to the queue in v1. Reads work safely against staging/prod precisely because nothing can be mutated.
- **Backend abstraction:** a `QueueBackend` trait with a `capabilities()` method and an extensions map on the domain model for backend-specific fields. Only two implementations ship: `PgBossBackend` and `SandboxBackend`. Mutation methods arrive with the post-MVP write path.

```rust
#[async_trait]
trait QueueBackend {
    async fn test_connection(&self) -> Result<BackendInfo>;
    async fn list_queues(&self) -> Result<Vec<QueueSummary>>;
    async fn list_jobs(&self, filter: JobFilter) -> Result<Page<JobSummary>>;
    async fn get_job(&self, id: &JobId) -> Result<JobDetail>;
    fn capabilities(&self) -> Capabilities;
    // job_action(...) intentionally omitted in v1 (read-only); added with the write path.
}
```

- **Security/secrets:** connection secrets stored via the OS keychain; never in plaintext config. No telemetry in the MVP (or strictly opt-in, off by default).
- **Packaging:** cross-platform builds for macOS, Windows, and Linux. Dark theme default.

---

## 9. UX principles

- **Lifecycle-first, not table-first.** The state machine is the home of the app; the job table is a drill-down.
- **Latency over volume.** Age-of-oldest-waiting is a headline metric.
- **Calm and legible.** Serves a nervous newcomer and a stressed debugger at once via progressive disclosure. Reads as an application (think code editor / observability client), not a web page.
- **Read-only by design.** v1 cannot change anything; the user can explore freely with zero risk.
- **Honest across backends.** Capability-driven rendering; never show a concept the current backend doesn't have.

---

## 10. Milestones (suggested phasing)

1. **Skeleton + sandbox:** Tauri+Angular shell, app chrome, dark theme, the `QueueBackend` trait, and the sandbox simulator. Deliverable: a running app showing a fake live queue with no DB dependency. *(De-risks the UI and the abstraction first.)*
2. **Read path:** pg-boss v10 adapter — connect, version-detect, overview, lifecycle counts, job explorer, job detail. Deliverable: a fully functional read-only inspector against a real database.
3. **Hero + teaching:** the animated lifecycle flow, the annotation layer, oldest-waiting metric surfacing, and near-live polling tuning. P1s (sparkline, light theme) as capacity allows.
4. **Release:** cross-platform packaging, a README that clearly states the pg-boss v10+ requirement, and a sandbox-driven demo (GIF/screens) for the repo.

---

## 11. Open questions & risks

- **Animation performance (top build risk).** Now that the flow animation is P0, the hardest engineering problem is rendering smooth motion in a webview on a busy queue. The mitigation is to drive motion from **aggregate flow rates** (sampled particles representing throughput) rather than one sprite per job, and to choose the right rendering tech (canvas/WebGL over lots of animated DOM/SVG). Worth a spike early.
- **Polling cost at scale.** Grouped-count queries against v10's per-queue **partitioned** job table can get expensive on large instances. Queries must be written and indexed carefully, and the poll interval kept sane.
- **Sandbox fidelity.** An in-memory simulator isn't real pg-boss SQL, so it can't teach pg-boss's exact quirks. Acceptable for building queue *intuition* — worth naming honestly in the UI/README.
- **Multiple concurrent connections.** Several pollers running at once has resource and state-management implications; keep per-connection tasks cheap and cleanly torn down on disconnect.
- **Secrets across platforms.** OS keychain behavior differs across macOS/Windows/Linux; needs testing on all three.

---

## 12. Post-MVP (directional)

The **write path** (retry / cancel / delete) with dry-run/SQL-preview and a read-only toggle — the first addition, and the reason the architecture keeps that seam ready. Then: a second backend (likely BullMQ — a Redis transport forces the abstraction to prove itself), native notifications/alerting, cross-connection dashboards, throughput history and trends, saved views/filters, and richer diagnostics (stuck-job detection, backlog projections).
