# Epic 1 — Skeleton & Sandbox — Runbook

Companion to `docs/superpowers/specs/2026-07-21-queue-boss-skeleton-sandbox-design.md`. One section per child. A driver session reads **only** its child issue + the spec + this runbook — each recipe is self-sufficient.

## Global conventions (apply to every child)

- **Outside-in TDD is the gate, not a suggestion.** Write the failing test first, watch it fail for the right reason, minimal change to green, refactor green. Start at the outermost reachable layer; drop to a unit test only when the outer test can't reach the behavior. (`tdd-evidence` gate.)
- **Strictly declarative Angular.** Presentational components: inputs/outputs only, no injected data services, no `invoke`. All Tauri access in the interface service; state in signal facades; presentational logic in directives/pipes. (`ng-declarative-purity` gate.)
- **Minimal code comments — tests are the documentation of record.**
- **Conventional Commits, NO AI-attribution trailers** (no `Co-Authored-By: Claude`, no "Generated with Claude Code"). Squash-merge.
- **context-mode routing** per repo CLAUDE.md (no raw curl/wget; sandbox for big output).
- **Toolchain** (from `.claude/epic.yaml`): `npm run test:ci` · `npm run lint` · `npm run tauri:build` · `npm run e2e` · `cargo mutants`.
- **Verified toolchain facts** (2026): scaffold = `create-tauri-app` Angular template; Angular 22 tests = **Vitest** (`ng test --no-watch --no-progress`); Angular app builder output = `dist/<app>/browser`; Tauri dev port **4200**, SSR **off**; streaming = `tauri::ipc::Channel<T>`; e2e = WebdriverIO, `tauri-driver` is **Windows+Linux only** (macOS via `@wdio/tauri-service`), Linux CI headless via **xvfb**; mutants config = `.cargo/mutants.toml`, changed-scope = `cargo mutants --in-diff`.
- **Crate package names ≠ directory names.** The three library crates live in dirs `crates/{core,backends,platform}` but are **packaged** `qb-core` (lib `qb_core`), `qb-backends`, `qb-platform` — the package `name` is set in each Cargo.toml (the `qb-` prefix avoids shadowing Rust's builtin `core` crate, which breaks `async-trait`/`thiserror` macro expansion in dependents). Cargo `-p` flags and Rust `use` paths take the PACKAGE/lib name (`-p qb-core`, `qb_core::conformance`); only workspace-member entries and on-disk file paths use the dir (`crates/core/...`). `--workspace` commands are unaffected.
- **Gating ordering:** C1 merges **ungated** (branch protection can't require a not-yet-existing workflow); the `claude-review` + CI gates switch on from **C2**.

---

## C1 — Scaffold & TDD harness  *(P0; blocked by: none)*

**Intent:** Stand up the Tauri 2 + Angular 22 monorepo as a Cargo workspace with both test runners green and lint/format wired; app boots blank.

**Files/modules:**
- Scaffold with `npm create tauri-app@latest queue-boss -- --template angular --manager npm` into a temp dir, then reconcile into the existing repo root (preserve `.gitignore`, `LICENSE`; the repo's `.gitignore` is already the Rust template).
- Convert Rust to a **workspace**: root `Cargo.toml` `[workspace] members = ["src-tauri", "crates/core", "crates/backends", "crates/platform"]`; create the three library crates as empty-but-compiling skeletons, **each Cargo.toml setting its package `name`** — `qb-core` (lib `qb_core`), `qb-backends`, `qb-platform` (dirs stay `crates/{core,backends,platform}`); make `src-tauri` a member. `qb-platform` ships the OS-seam **port stubbed**: a `SecretStore` trait with a `NoopSecretStore` implementation (real keychain-backed impl deferred to E2).
- `tauri.conf.json` `build`: `beforeDevCommand: "ng serve"`, `beforeBuildCommand: "ng build"`, `devUrl: "http://localhost:4200"`, `frontendDist: "../dist/queue-boss/browser"`. SSR off. Baseline `app.security.csp`.
- `package.json` scripts: `test:ci` = `ng test --no-watch --no-progress && cargo test --workspace`; `lint` = biome/eslint + `prettier --check` + `cargo clippy --workspace -- -D warnings` + `cargo fmt --all --check`; `tauri:build` = `tauri build`; `e2e` = WebdriverIO driven by `wdio.conf` against a launch-smoke spec (C8 extends it with the live-update assertion).
- **e2e harness (launch-smoke):** `wdio.conf.ts` + `tests/e2e/launch-smoke.e2e.ts` — ONE trivial spec that boots the app and asserts the window title, so `npm run e2e` **exits 0 before C8 exists** (a never-passing required check would wedge every C3–C7 PR). `wdio.conf` points the app-binary path at the **release** build (`src-tauri/target/release/<bin-name>`) — C2's `e2e` job builds release via `npm run tauri:build`. C8 extends/replaces this spec with the live-update assertion; the CI `e2e` job that runs it is owned by C2.
- Config files: `rustfmt.toml`, `.cargo/mutants.toml` (workspace baseline), `biome.json` (or eslint+prettier).
- **Dev tooling installs** (so the C3/C4/C5 mutation gate and the C7/C8 a11y + e2e harness have their binaries): `cargo install cargo-mutants`; npm devDependencies `@wdio/cli`, `@wdio/tauri-service`, `@wdio/local-runner`, `@wdio/mocha-framework`, and `vitest-axe` (the runner `@wdio/local-runner` + framework `@wdio/mocha-framework` are required for `npm run e2e` to boot). (`tauri-driver` is NOT installed here — it is Windows+Linux only, so it is installed in C2's Linux `e2e` CI job and is not needed/installed on macOS driver machines.)

**TDD order:**
1. In `crates/core`, add a trivial unit test that **fails** (e.g. asserts a placeholder `version()` returns `"0.1.0"` before it exists) → run `cargo test -p qb-core` → red → implement → green. Proves the Rust runner executes.
2. In `crates/platform`, add a failing unit test for the `SecretStore` port (e.g. `NoopSecretStore::get(key)` returns `Ok(None)`, a no-op) → `cargo test -p qb-platform` → red → implement the `SecretStore` trait + `NoopSecretStore` stub → green. Proves the platform-crate seam is real and testable (and seeds the `rust-mutation-coverage` gate below).
3. In the Angular app, add a trivial component/pipe spec that fails → `ng test --no-watch` → red → implement → green. Proves the Vitest runner executes.

**Verification:** `npm ci` · `npm run lint` · `npm run test:ci` (both suites green) · `npm run tauri:build` (debug bundle builds and app boots to a blank shell) · `cargo clippy --workspace -- -D warnings` · `cargo fmt --all --check`.

**Gate notes:** `tdd-evidence` — the red→green cycles above are the evidence. `rust-mutation-coverage` — C1 adds **minimal** Rust logic (`qb_core::version()` + the `NoopSecretStore` no-op), so run `cargo mutants` on the changed crates and kill or justify survivors; the tiny surface makes this trivially satisfied, but the gate fires because C1 touches the platform crate. `xplat-build-smoke` — verifiable only on the driver's OS at this point (the 3-OS matrix arrives in C2); note the local build OS in the PR body.

**Done when:** clean checkout → `npm ci && npm run test:ci && npm run tauri:build` succeeds; app launches to a blank dark shell.

---

## C2 — CI pipeline + `claude-review` gate + branch protection  *(P0; blocked by: C1)*

**Prerequisite:** `ANTHROPIC_API_KEY` must be readable by this **public** repo (operator widened the org secret to include Queue-Boss). If absent, `claude-review` cannot run — confirm before enabling protection.

**Intent:** GitHub Actions CI (lint + test + build matrix on macOS/Windows/Linux) plus the org-standard Claude review gate, then branch protection.

**Files/modules:**
- `.github/workflows/ci.yml` — **job topology matches `epic.yaml` `required_checks` exactly:** `lint` and `test` are **single ubuntu-latest jobs** (NOT matrixed — one report each: `npm run lint` and `npm run test:ci`); `build (<os>)` is the **3-OS matrix** (`os: [ubuntu-latest, macos-latest, windows-latest]` running `npm run tauri:build`, reported per-OS as `build (ubuntu-latest)` / `build (macos-latest)` / `build (windows-latest)`); the `e2e` job (below) is **ubuntu-only**. Every job first does setup Node + Rust and `npm ci`. **On every ubuntu job** (`lint`, `test`, `build (ubuntu-latest)`, `e2e`) install Tauri's Linux system deps FIRST (`sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev build-essential webkit2gtk-driver xvfb`) — the Linux `tauri:build`/e2e reds on the first run without them (`webkit2gtk-driver` = the `WebKitWebDriver` binary `tauri-driver` proxies to; `xvfb` = the headless display for `xvfb-run`). These job names must match the `epic.yaml` `required_checks` (reconcile before enabling protection).
- **Linux e2e CI job (`e2e`) — C2 OWNS e2e execution.** A dedicated ubuntu-latest job: install the Linux system deps above, then `cargo install tauri-driver` (installed **here**, not in C1 — `tauri-driver` is Windows+Linux only and is not needed/installed on macOS driver machines), then **build the app** with `npm run tauri:build`, then run it headless under **xvfb** (`xvfb-run -a npm run e2e`) — the build MUST precede the e2e run so the driver has a built binary to launch. Name it `e2e` to match `epic.yaml` `required_checks`. C1 ships the `wdio.conf` + launch-smoke spec this job runs from day one; C8 supplies the live-update e2e **spec + app wiring**; this job is the CI harness that runs them.
- `.github/workflows/claude-review.yml` — **replicate gangan-api's** (`getvoicify/gangan-api/.github/workflows/claude-review.yml`): `name: Claude Review`, job id **`claude-review`**, `on: pull_request [opened, synchronize, reopened, ready_for_review]` to `main`, per-PR concurrency cancel-in-progress, `permissions: contents:read pull-requests:write issues:write id-token:write`, pinned `anthropics/claude-code-action@…v1`, `anthropic_api_key: ${{ secrets.ANTHROPIC_API_KEY }}`. **Two adaptations:** (a) the job `if` — gangan-api's workflow **already** carries `github.event.pull_request.draft == false`, so the **only NEW clause** is the fork-repo guard `github.event.pull_request.head.repo.full_name == github.repository` ANDed onto it (fork PRs can't read secrets, so they must skip the job — note the fork-PR gating gap recorded in `epic.yaml` `merge.notes` and spec §7); (b) rewrite the review `prompt` for **this** stack — Rust workspace (core/backends/platform boundaries, `cargo mutants` teeth, no write-path leakage), strictly-declarative Angular (dumb components, no `invoke` outside the interface service), outside-in TDD evidence, Conventional Commits without AI trailers.
- Branch protection on `main` (via `gh api`/ruleset): required checks = `claude-review`, `lint`, `test`, `build (ubuntu-latest)`, `build (macos-latest)`, `build (windows-latest)`, `e2e`; `required_review_thread_resolution: true`; `required_approvals: 0`; squash only.

**TDD order:** Workflow YAML is validated by execution, not unit tests. The C2 PR itself is the test: its matrix must go green on all three OSes and `claude-review` must run and post a verdict on the C2 branch (the workflow is present on the PR head). **Enable branch protection as the LAST step**, only after the checks are proven on the PR.

**Verification:** C2 PR shows green `lint`/`test`/`build (*)` on 3 OSes + the Linux `e2e` job + a `claude-review` check; after enabling protection, `gh api repos/getvoicify/Queue-Boss/branches/main/protection` lists the required checks (incl. `e2e`).

**Gate notes:** `xplat-build-smoke` — the matrix *is* the gate. Reconcile the actual job names into `epic.yaml` `merge.required_checks` before enabling protection (a name mismatch would block every future PR on a never-reported check).

**Done when:** every subsequent PR to `main` is gated by `claude-review` + the CI matrix with thread-resolution required.

---

## C3 — Rust core: `QueueBackend` trait + domain model  *(P0; blocked by: C1)*

**Intent:** Define the platform-agnostic contract in `crates/core` — the seam the whole product hangs off.

**Files/modules:** `crates/core/src/{lib.rs, model.rs, r#trait.rs, error.rs, page.rs, testing.rs}`. Package `qb-core` (lib `qb_core`); depends on **neither** `qb-backends` nor `qb-platform`.

**Contract (from spec §3.2):** `QueueBackend` (read-only: `test_connection`, `list_queues`, `list_jobs`, `get_job`, `capabilities`; NO `job_action`). `JobState` (7 variants incl. Retry, DeadLetter, Cancelled). `QueueSummary` with `oldest_waiting_age: Option<Seconds>` first-class. `JobSummary`, `JobDetail` (+ `timeline`, `retry` readout, `extensions` map). `Capabilities`. `Page<T>` cursor pagination. `JobFilter`. `BackendError` typed enum (no raw driver strings).

**TDD order (unit — a trait alone isn't executable, so drive the model):**
1. `JobState` serde round-trip + display — **assert the exact wire strings** (`created`, `active`, `completed`, `failed`, `cancelled`, `retry`, `deadLetter`), so the **enum-level `#[serde(rename_all = "camelCase")]`** is verified (not merely that the round-trip is lossless); red → implement enum → green.
2. `Page<T>` cursor encode/decode (base64url JSON `{createdAt,id}`) round-trips; boundary cases; red → green.
3. `QueueSummary::counts_by_state` sums to `total_depth`; `oldest_waiting_age` semantics; red → green.
4. `extensions` map (de)serialization preserves unknown backend keys; red → green.
5. Prove the trait is implementable **and hand C5 a reusable fake**: export `pub mod testing { pub struct FakeBackend … }` — a `impl QueueBackend` returning fixtures, **NOT `#[cfg(test)]`-gated**, so `src-tauri` tests can `use qb_core::testing::FakeBackend`. Assert it compiles and is object-safe (`Box<dyn QueueBackend>`).

**Verification:** `cargo test -p qb-core` · `cargo clippy -p qb-core -- -D warnings` · `cargo fmt --check` · `cargo mutants -p qb-core`.

**Gate notes:** `tdd-evidence`. `rust-mutation-coverage` — `cargo mutants -p qb-core` must kill survivors on cursor encode/decode, count summation, oldest-waiting logic; justify any survivor in the PR body.

**Done when:** the trait + model compile, are object-safe, and the model unit tests + mutants pass.

---

## C4 — SandboxBackend + shared conformance suite  *(P0; blocked by: C3)*

**Intent:** The in-memory simulator, and a **reusable conformance suite** every adapter runs (the transport-agnostic proof; E2's `PgBossBackend` will run the same suite).

**Files/modules:** `crates/core/src/conformance.rs` (a `pub` harness exported at **`qb_core::conformance`**: `async fn assert_backend_conforms(b: &impl QueueBackend, clock, seed)`), `crates/backends/src/{lib.rs, sandbox.rs, simulator.rs}` (package `qb-backends`), `crates/backends/tests/sandbox_conformance.rs` (imports `qb_core::conformance::assert_backend_conforms`).

**TDD order (outside-in — the conformance suite is the failing spec):**
1. Write `conformance.rs` asserting the trait contract: `list_queues` returns ≥1 queue; `list_jobs` paginates (cursor advances, `has_more` correct) and filters by state; `get_job` returns a detail whose `timeline` is ordered; over simulated time jobs move Created→Active→(Completed|Failed→Retry→…→DeadLetter); `counts_by_state` sums to `total_depth`; `oldest_waiting_age` is populated when jobs wait. This compiles but **fails** (no impl).
2. Implement `SandboxBackend` + `simulator` (synthetic producer/consumer) under an **injected clock + seed** for determinism; free-running clock in the app. Green.

**Verification:** `cargo test -p qb-backends` · `cargo mutants -p qb-backends` · clippy · fmt.

**Gate notes:** `tdd-evidence`, `rust-mutation-coverage` (kill survivors on the state-machine transitions + pagination).

**Done when:** `SandboxBackend` passes `assert_backend_conforms`; the suite is `pub` and adapter-agnostic.

---

## C5 — Tauri command + event/polling bridge  *(P0; blocked by: C3)*

**Intent:** Expose the read methods as commands and stream aggregate counts per connection; managed backend state. Built against the **trait** (C3) with a fake backend — does not need the sandbox to be testable.

**Files/modules:** `src-tauri/src/{lib.rs, commands.rs, poller.rs, state.rs}`. `state.rs`: `AppState { backends: Map<ConnectionId, Arc<dyn QueueBackend>> }` via `.manage()`. `commands.rs`: `#[tauri::command]` `test_connection`/`list_queues`/`list_jobs`/`get_job` (each takes `connectionId`, resolves the backend, delegates; unknown id → typed error). `poller.rs`: `subscribe_counts(connectionId, channel: tauri::ipc::Channel<QueueCounts>)` — spawns a per-connection Tokio interval task running grouped counts and pushing snapshots into the `Channel`. **Teardown:** the poll task's `AbortHandle` is retained in `AppState` and is **aborted on connection removal, or on send-error** — `Channel::send` errors once the frontend drops the `Channel`, which the task treats as its stop signal (no leaked task).

**TDD order:**
1. Command handlers over a **fake** `QueueBackend` (from C3's `qb_core::testing::FakeBackend`): `list_queues` delegates and maps; unknown `connectionId` → typed error, not a panic. Keep handlers thin over pure, testable functions. Red → green.
2. Poller: with an injected clock + fake backend + a test channel sink, it emits N snapshots on the interval and stops on drop (no leaked task). Red → green.

**Verification:** `cargo test --workspace` (the app crate is named after the app, not `src-tauri`, so scope with `--workspace`) · `cargo mutants --in-diff <(git diff main)` on the changed modules · clippy · fmt.

**Gate notes:** `tdd-evidence`, `rust-mutation-coverage`. Use `Channel<T>` (verified idiomatic), keyed by `connectionId`, torn down cleanly — the aggregate-counts-only contract from spec §3.3 is mandatory (no per-job events).

**Done when:** commands invoke against a managed backend; the poller streams count snapshots and tears down cleanly; tests + mutants pass.

---

## C6 — Angular interface + facade layer  *(P0; blocked by: C5)*

**Intent:** The **sole** Tauri touchpoint plus signal-based facades. No UI.

**Files/modules:** `src/app/core/tauri/queue-backend.service.ts` (wraps `invoke` + the counts `Channel`; the ONLY file importing `@tauri-apps/api`), `src/app/core/facades/{queues.facade.ts, connection.facade.ts}` (signals), `src/app/core/models/*.ts` (TS mirrors of the Rust domain — `JobState`, `QueueSummary`, and the `QueueCounts` poll payload; **mirror spec §3.2 + §3.3 directly — read the SPEC, not the sibling Rust source**).

**TDD order (unit — no UI yet):**
1. Interface service: mock `@tauri-apps/api` `invoke`/`Channel`; assert `listQueues(connectionId)` calls `invoke('list_queues', {connectionId})` and maps the payload; `subscribeCounts` wires a `Channel` and surfaces pushes as a signal/observable. Red → green.
2. Facades: given a mocked interface service, `QueuesFacade` exposes a read-only `queues` signal that updates as counts stream in; `ConnectionFacade` exposes a `status` signal. Red → green.

**Verification:** `ng test --no-watch --no-progress` · `npm run lint`.

**Gate notes:** `tdd-evidence`. `ng-declarative-purity` — facades expose read-only signals + intent methods; **only** `queue-backend.service.ts` imports `@tauri-apps/api` (the gate greps for stray `invoke`/`@tauri-apps/api` imports elsewhere).

**Done when:** services + facades are tested with `invoke`/`Channel` mocked; no other module touches Tauri.

---

## C7 — Angular presentational UI + chrome + dark theme  *(P0; blocked by: C6)*

**Intent:** Dumb components fed by facades, presentational logic in directives/pipes, the app shell, dark theme default.

**Files/modules:** `src/app/features/overview/*` (queue list: depth + per-state counts, oldest-waiting shown), `src/app/features/lifecycle/*` (state-count list — numeric, NOT animated), `src/app/shell/primary-nav/*` (the **primary-nav component** switching between Overview and Lifecycle, per spec §3.6) + its **route registration** in `src/app/app.routes.ts` (Overview + Lifecycle routes), `src/app/shared/pipes/age.pipe.ts`, `src/app/shared/directives/state-color.directive.ts`, `src/app/shell/*` (chrome + connection-status region), `src/styles/*` (dark theme tokens).

**TDD order:**
1. Component tests: given inputs (signals), Overview renders a row per queue with depth + counts + oldest-waiting; Lifecycle renders a node per `JobState` with its count; ConnectionStatus renders the status. **Assert pure rendering — zero logic.** Red → green.
2. Pipe/directive unit tests: `age` pipe formats a duration ("3m ago"); `stateColor` maps `JobState`→class. Red → green.
3. **a11y in-spec:** the Overview + Lifecycle + ConnectionStatus component specs run **`vitest-axe`** (`expect(...).toHaveNoViolations()`) against the rendered output — no serious/critical violations; controls keyboard-reachable + labelled. (Runs in **jsdom**, so this covers structure/labels/keyboard; **color-contrast is NOT asserted here** — jsdom has no real render — it is verified against the real webview in C8.) Red (violation) → fix → green.

**Verification:** `ng test --no-watch` (includes the **`vitest-axe`** a11y assertions wired into the Overview + Lifecycle + ConnectionStatus specs) · `npm run lint`.

**Gate notes:** `tdd-evidence`. `ng-declarative-purity` — components take inputs/emit outputs only, no injected data services, no `invoke`. `a11y-audit` — `vitest-axe` runs in **jsdom**, so it covers structure, labels, and keyboard-reachability (no serious/critical violations; controls keyboard-reachable + labelled); **color-contrast rules cannot run in jsdom** (no real render) and are deferred to C8's real-webview axe check.

**Done when:** components render sandbox-shaped data from inputs; pipes/directives unit-tested; axe clean.

---

## C8 — End-to-end sandbox wiring + e2e test  *(P0; blocked by: C2, C4, C7)*

**Intent:** Wire the default `SandboxBackend` into Tauri state, the "Enter Sandbox" entry action, and the live-counts pipeline end-to-end — and prove it with an e2e test. **This is the epic deliverable.**

**Files/modules:** `src-tauri` wiring (register `SandboxBackend` as the default connection id `"sandbox"` in `AppState`), a thin container component binding `QueuesFacade`/`ConnectionFacade` → the C7 presentational components, the "Enter Sandbox" action/route, and the live-update e2e assertion that **extends/replaces C1's `tests/e2e/launch-smoke.e2e.ts`** (e.g. `tests/e2e/sandbox.e2e.ts`, WebdriverIO + `@wdio/tauri-service`) — C8 does NOT author the harness, `wdio.conf`, or the launch-smoke spec (all shipped by C1). **The e2e SPEC + app wiring live here; the CI `e2e` job that runs it (Linux + xvfb + `tauri-driver`) is OWNED by C2** — C8 adds no `.github/workflows/*`.

**TDD order (outside-in — e2e first):**
1. **Extend C1's launch-smoke spec** into the live-update e2e (do NOT re-author the harness): launch app → click "Enter Sandbox" → assert queue rows appear → capture a count, wait past **one poll interval (1000ms default, per spec §3.3)**, assert a count **changed** (live update). Red (nothing wired).
2. Faster inner loop: an Angular container integration spec wiring real facades to a fake interface service, asserting counts flow to the rendered components. Red → green.
3. Wire the container + default sandbox + `subscribeCounts` subscription to green the e2e.

**Verification:** `npm run e2e` — canonical on the **C2-owned Linux `e2e` job** (headless via xvfb; `tauri-driver` is Windows+Linux only; `@wdio/tauri-service` adds macOS but Linux is the gate). Run **axe (`vitest-axe`)** against the wired "Enter Sandbox" screen — no serious/critical violations. `npm run test:ci` · `npm run tauri:build` on all three OSes.

**Gate notes:** `tdd-evidence`, `ng-declarative-purity` (the container binds facades→components; the container is not a "dumb" component but must still keep business logic in facades), `a11y-audit` (the "Enter Sandbox" entry affordance is user-facing — no serious/critical violations; keyboard-reachable + labelled; **and C8 is where color-contrast is actually verified** — jsdom `vitest-axe` in C7 can't run contrast rules, so C8 runs axe against the REAL webview via WDIO, or records a manual dark-theme contrast check in the PR body), `xplat-build-smoke` (build green on 3 OSes; the e2e runs on the C2-owned Linux job, mac/win covered by build + unit/integration).

**Done when:** clean checkout → launch → Enter Sandbox → a live-updating fake queue, **no DB**; the e2e passes on Linux CI; builds pass on all three OSes.

---

## Drive order

`C1 → C2` (gate on) → `C3 → {C4, C5}` → `C6 → C7` → `C8`. C2 can run in parallel with C3 once C1 lands. C4 and C5 are parallel after C3. C8 wires C4 + C7 (C5's bridge reaches it transitively via C6→C7) **and requires C2's Linux `e2e` CI job** (C8 blocked-by: C2, C4, C7 — C5 is transitive via C6→C7).
