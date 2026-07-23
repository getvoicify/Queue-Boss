# Epic 3 — Hero Animation & Teaching — Runbook

Companion to `docs/superpowers/specs/2026-07-23-hero-lifecycle-teaching-design.md`. One section per child. A driver session reads **only** its child issue + the spec + this runbook — each recipe is self-sufficient. Builds on the E1 skeleton and the E2 read path ([E2 design](../specs/2026-07-23-pgboss-read-path-design.md) / [E2 runbook](2026-07-23-pgboss-read-path-plan.md)), both merged to `main` @ `831bb92`; the facades, the aggregate-counts poller, the strictly-declarative Angular layers, `StateColorDirective`, and the sandbox/pg-boss backends already exist. E3 is **FE-only** (Angular markup/TS/CSS + self-hosted fonts) — no Rust, no new Tauri command, no CI-workflow edits.

## Global conventions (apply to every child)

- **Outside-in TDD is the gate, not a suggestion.** Write the failing test first, watch it fail for the right reason, minimal change to green, refactor green. Start at the outermost reachable layer (component/integration/e2e) and drop to a unit test only when the outer test can't reach the behavior. (`tdd-evidence` gate.)
- **Strictly declarative Angular.** Presentational components: `input()`/`output()` only, no injected data services, no `invoke`. All Tauri access stays in `src/app/core/tauri/queue-backend.service.ts`; state in signal facades; presentational logic in directives/pipes; containers assemble `computed` view-state and bind it to dumb children. (`ng-declarative-purity` gate.)
- **Read-only, always.** E3 issues no writes and adds no backend surface — it consumes only the existing aggregate-counts stream (`QueuesFacade.queues()`). No new Tauri command, no `invoke` added outside the interface service.
- **Dark-first.** The OKLCH ramp is authored dark-only; no light theme this epic.
- **Minimal code comments — tests are the documentation of record.**
- **Conventional Commits, NO AI-attribution trailers** (no `Co-Authored-By: Claude`, no "Generated with Claude Code"). Squash-merge.
- **context-mode routing** — no raw curl/wget; route large command output through the sandbox.
- **Toolchain** (from `.claude/epic.yaml`): `npm run test:ci` (Angular headless Vitest) · `npm run lint` (biome + prettier `--check`) · `npm run e2e` (WebdriverIO + tauri-driver). Angular unit tests: **`ng test --no-watch --no-progress` under Node 24 (`nvm use 24`)**.
- **Per-PR CI gates on Linux** (`.claude/epic.yaml` `merge.required_checks`): `claude-review`, `lint`, `test`, `build (ubuntu-latest)`, `e2e`. **No E3 child touches Rust or the CI workflow**, so `xplat-build-smoke` and `rust-mutation-coverage` do not apply to this epic — every child's gates are `tdd-evidence`, `ng-declarative-purity`, `a11y-audit`.

## Verified surface facts (main @ 831bb92)

Anchor every child to these; do not re-derive.

- **`JobState`** (`src/app/core/models/job-state.ts`) = `created | active | completed | failed | cancelled | retry | deadLetter` (seven). `deadLetter` is **camelCase**.
- **State tokens** — `src/styles/tokens.css` `:root` has **seven flat hex** `--qb-state-*`: created `#6e7781`, active `#2f81f7`, completed `#3fb950`, failed `#f85149`, cancelled `#8b949e`, retry `#d29922`, deadLetter `#a371f7`. A **separate** `--qb-status-*` family (connected `#3fb950` / connecting `#d29922` / error `#f85149`) drives connection dots — **leave it untouched**.
- **`StateColorDirective`** (`src/app/shared/directives/state-color.directive.ts`) — host bindings `[attr.data-state]="appStateColor()"` and `[style.--qb-state-color]="colorVar()"` where `colorVar() = \`var(--qb-state-${appStateColor()})\`` and `appStateColor = input.required<JobState>()`. It resolves `var(--qb-state-<JobState-literal>)`, so **the ramp must keep `--qb-state-<name>` resolvable** (alias it to the `solid` step) and the state segment must be the `JobState` literal (`deadLetter`, not `deadletter`). Consumers: lifecycle dots + job-list rows (E2-6).
- **Counts model** (`src/app/core/models/queue.ts`) — `QueueCountEntry { queue, totalDepth, countsByState, oldestWaitingAge }`; `countsByState` is `Partial<Record<JobState,number>>` (**sparse**). `QueueCounts { connectionId, queues: QueueCountEntry[], polledAt }`. **No aggregate exists.**
- **Facades** — `QueuesFacade.queues: Signal<QueueCountEntry[]>` + `connect(connectionId)` (opens the counts `Channel`), `src/app/core/facades/queues.facade.ts`. `ConnectionsFacade` (`connections.facade.ts`): `activeConnectionId: Signal<string>` seeded `"sandbox"` (`SANDBOX_CONNECTION_ID`), `connect`/`disconnect`, per-connection status map; the sandbox is seeded connected and can never be disconnected.
- **Container pattern** — `src/app/features/overview/overview-container.component.ts` is the template: `standalone`, `OnPush`, injects `QueuesFacade` + `ConnectionsFacade`, `computed` view-state, an `effect()` that calls `queues.connect(id)` when `activeConnectionId()` changes, binds `[queues]="queues.queues()"` to the dumb `OverviewComponent`. **Note:** `/overview` gates the sandbox behind an `enter-sandbox` button; the E3 home screen instead connects on entry so the hero flows immediately.
- **Routes** — `src/app/app.routes.ts`: `/overview → OverviewContainerComponent`, `/jobs → JobsContainerComponent`, `/lifecycle → LifecycleComponent` (direct, no container), `/connect → ConnectContainerComponent`, and `{ path: "", pathMatch: "full", redirectTo: "overview" }`.
- **Inert lifecycle** — `src/app/features/lifecycle/lifecycle.component.ts` is a dumb `<dl>` of the seven states, `counts = input<Partial<Record<JobState,number>>>({})`, currently wired to nothing (renders 0). Stays as the plain `/lifecycle` secondary read; the animated hero is a new component.
- **Fonts** — `src/styles.css` sets `font-family: Inter, Avenir, Helvetica, Arial, sans-serif`. **No IBM Plex, no `@font-face`** (greenfield). **No `prefers-reduced-motion` handling anywhere** (greenfield).
- **Purity boundary** — the only **production** importer of `@tauri-apps/api` is `queue-backend.service.ts` (test mocks/specs reference the `@tauri-apps/api` string too, so the `ng-declarative-purity` grep must scope to production `src` non-spec files). `vitest-axe` idiom: `expect(await axe(el)).toHaveNoViolations()`; setup `src/testing/setup-axe.ts`. `data-testid` is pervasive. `a11y-audit` = `vitest-axe` in jsdom (structure/labels/keyboard); **color-contrast is not checked in jsdom** — documented per §3.3 / real-webview.

---

## E3-1 — State-token OKLCH ramp + IBM Plex fonts  *(P0; blocked by: none)*

**Intent:** Migrate `--qb-state-*` from 7 flat hex to an **OKLCH ramp per state — 4 valued steps** (`--qb-state-<name>-{bg|border|solid|text}`) **plus a reserved `-on`**, keeping a primary `--qb-state-<name>` aliased to the `solid` step so `StateColorDirective` and existing consumers don't break, and exposing the ramp for the E3-2 hero. Self-host **IBM Plex Sans** (UI) + **IBM Plex Mono** (counts) as `@font-face` woff2 in assets. Migrate the existing consumers (lifecycle dots, job-list rows) onto the ramp where it adds clarity — **no visual regression**. Foundation child: blocks E3-2.

**Files/modules:** `src/styles/tokens.css` (replace the 7 flat `--qb-state-*` with the ramp below + the `--qb-state-<name>` = `solid` aliases; add the new connector token `--qb-border-strong: oklch(0.62 0.02 250)` (the hero panel reuses the existing `--qb-surface`/`--qb-border` — no new surface or edge token); **do not touch `--qb-status-*`**); `src/styles.css` (register the `@font-face`s, set the UI font to IBM Plex Sans, expose an IBM Plex Mono family var for counts); `src/assets/fonts/*` (self-hosted woff2 for Plex Sans + Plex Mono — **source via npm `@fontsource/ibm-plex-sans` + `@fontsource/ibm-plex-mono`** and copy their woff2 into `src/assets/fonts/`; curl/wget are blocked here, so name the package — subset to the weights used; apply **Plex Mono to the counts**); `src/app/shared/directives/state-color.directive.ts` (unchanged if the `--qb-state-<name>` alias holds, OR extend to also emit ramp vars — pick the **minimal** path that keeps dots/rows regression-free); job-list row + lifecycle-dot styles (migrate onto the ramp where clarity improves). Unit tests: a token/directive spec asserting `--qb-state-<name>` still resolves and `data-state` is unchanged.

**Contract — the exact OKLCH ramp — the 4 valued steps `bg`/`border`/`solid`/`text` (dark-first; map onto `--qb-state-<name>-*`; alias `--qb-state-<name>` = the `-solid` step; `-on` is a reserved name, see below):**

| state (token segment) | `-bg` | `-border` | `-solid` | `-text` |
|---|---|---|---|---|
| `created` (slate) | `oklch(0.290 0.018 250)` | `oklch(0.430 0.024 250)` | `oklch(0.620 0.030 250)` | `oklch(0.790 0.024 250)` |
| `active` (azure = brand) | `oklch(0.300 0.060 250)` | `oklch(0.470 0.110 250)` | `oklch(0.640 0.150 250)` | `oklch(0.810 0.100 250)` |
| `completed` (green 155) | `oklch(0.300 0.055 155)` | `oklch(0.470 0.095 155)` | `oklch(0.680 0.150 155)` | `oklch(0.830 0.110 155)` |
| `failed` (red 25) | `oklch(0.305 0.075 25)` | `oklch(0.480 0.140 25)` | `oklch(0.620 0.200 25)` | `oklch(0.805 0.135 25)` |
| `retry` (amber 75) | `oklch(0.325 0.055 75)` | `oklch(0.520 0.100 75)` | `oklch(0.770 0.150 75)` | `oklch(0.860 0.105 75)` |
| `cancelled` (grey) | `oklch(0.260 0.010 250)` | `oklch(0.380 0.012 250)` | `oklch(0.490 0.015 250)` | `oklch(0.620 0.014 250)` |
| `deadLetter` (violet 320) | `oklch(0.305 0.085 320)` | `oklch(0.470 0.140 320)` | `oklch(0.585 0.190 320)` | `oklch(0.820 0.120 320)` |

- **`-on`** (foreground for text/glyph placed *on* the `-solid` fill) is a **reserved** token name — **E3-1 ships only the 4 valued steps** (`bg`/`border`/`solid`/`text`); `-on` is **not required this epic** (the nodes/dots/glyphs use only `bg`/`border`/`solid`/`text`), so **do not author an `-on` value now**. When a text-on-solid surface first needs it, derive it then (near-black `oklch(0.150 0 0)` on light solids like retry/completed, near-white `oklch(0.980 0 0)` on dark solids) and document the choice. No E3 component consumes `-on` yet.
- **Surfaces & connectors:** the hero panel background reuses the existing **`--qb-surface`** (`#161b22`) and its 1px edge the existing **`--qb-border`** (`#30363d`) — **no new surface token**. E3-1 adds ONE new connector token **`--qb-border-strong: oklch(0.62 0.02 250)`** (a visible mid-slate on the dark `--qb-surface`) for the diagram connectors/arrowheads; the driver may a11y-tune it within the `a11y-audit` gate, but this is the concrete starting value.
- **Alias:** for each state, `--qb-state-<name>: var(--qb-state-<name>-solid);` so `StateColorDirective`'s `var(--qb-state-${state})` keeps resolving. **Token segment = the `JobState` literal** — camelCase **`deadLetter`** (`--qb-state-deadLetter-*` resolves), **NOT** the design prompt's lowercase `deadletter`.

**TDD order (outside-in — the migration must not regress the existing consumers):**
1. **Red:** a spec asserting the lifecycle-dot / job-list-row still renders the expected `data-state` and that `--qb-state-<name>` resolves (a computed-style read in jsdom, or the directive's `colorVar()` output). With the ramp in but the alias missing, the resolve assertion is **red**. Add the aliases → green (behavior preserved).
2. Add the `@font-face`s + point the UI font at IBM Plex Sans; a spec/asserts the counts family var is IBM Plex Mono. (Font *loading* is verified in the real webview; the unit level asserts the CSS var + family string.)
3. Migrate the dot/row styles onto the ramp (`solid` for dots, optional `bg`/`border` tint for rows) — the existing dot/row tests stay green; record before/after swatches for the operator.
4. `vitest-axe` unchanged-clean on the migrated dots/rows.

**Verification:** `ng test --no-watch --no-progress` (Node 24) · `npm run lint` · `npm run e2e` (dots/rows still render). **Document the measured contrast ratios** in the PR body: each `-text` on its `-bg` ≥ **AA 4.5:1**; each `-solid` on its `-bg` ≥ **3:1** (non-text). Record before/after color swatches for the flat-hex → OKLCH shift. Confirm the self-hosted **woff2 files land in the Tauri bundle's asset globs** — they sit under `src/assets/fonts/`, which the existing `build` check already bundles, so the fonts ship in the packaged app.

**Gate notes:** `tdd-evidence` — the failing "alias resolves / no regression" spec is the red. `ng-declarative-purity` — pure CSS + directive; no Tauri, no data service. `a11y-audit` — `vitest-axe` clean **and** the documented AA contrast ratios (contrast is not machine-checked in jsdom — record the computed ratios). No Rust, no CI edit → **no `rust-mutation-coverage` / `xplat-build-smoke`**.

**Done when:** `--qb-state-*` is the OKLCH ramp — the 4 valued steps (`bg`/`border`/`solid`/`text`) with `--qb-state-<name>` aliased to `solid` and `-on` left reserved; the new **`--qb-border-strong: oklch(0.62 0.02 250)`** connector token is added (the hero panel reuses the existing `--qb-surface`/`--qb-border` — no new surface token); `StateColorDirective` and the dots/rows render with no visual regression (swatches recorded); IBM Plex Sans/Mono are self-hosted and applied (Mono to counts); `--qb-status-*` is untouched; contrast ratios documented; axe clean.

---

## E3-2 — `LifecycleDiagramComponent` (the dumb animated hero)  *(P0; blocked by: E3-1)*

**Intent:** Build the standalone Angular 22 `LifecycleDiagramComponent` — the animated job-lifecycle hero — **exactly** per the embedded design prompt below. **Dumb** (signal `input()`s only, one `output()`); **pure Angular + inline SVG + CSS, NO libraries**; `<animateMotion>` for edge flow, a CSS keyframe for the pulse; `prefers-reduced-motion` gate; consumes the E3-1 ramp. Blocks E3-3 (wiring) and E3-4 (teaching).

**Files/modules:** `src/app/features/lifecycle/lifecycle-diagram.component.ts` (the standalone component — inline SVG template + inline styles carrying `@keyframes qb-node-pulse`); a co-located spec `lifecycle-diagram.component.spec.ts` (+ `vitest-axe`); **`src/testing/setup-axe.ts`** — extend the sole vitest setupFile with a **`window.matchMedia` stub** (see the BLOCKING note below). No facade, no service, no `invoke` — this component is inert until E3-3 feeds it.

**Inputs/outputs (dumb contract):**
- `counts = input<Record<JobState, number>>({...zeros})` — per-state totals (E3-3 supplies the aggregate fold).
- `animated = input<boolean>(true)`.
- `selected = input<JobState | null>(null)`.
- `annotation = input<{ state: JobState; text: string } | null>(null)`.
- `selectState = output<JobState>()` — emitted on node click / keyboard activation.

### EMBEDDED DESIGN PROMPT (build to this substance exactly)

**Nature.** Angular 22 standalone `LifecycleDiagramComponent` — a live animated SVG lifecycle diagram. Pure Angular + inline SVG + CSS, **NO libraries** (no D3/anim lib). SVG-native `<animateMotion>` for edge flow + CSS keyframes for the node pulse.

**Coordinate system.** Fixed internal canvas **760×480**, `viewBox="0 0 760 480"`, `width="100%"`. Nodes are **absolutely-positioned HTML boxes over the SVG**, placed by **percentages of 760/480** — that is what glues the boxes to the SVG anchors at any width. Node box **150×88**. Top-left (x,y): `created(6,176) active(248,110) retry(248,300) completed(560,18) failed(560,176) cancelled(560,334) deadletter(300,402)`. Per-node anchor helpers: `center`, `right-middle (= x+150, y+44)`, `left-middle (= x, y+44)`, `top-middle (= x+75, y)`, `bottom-middle (= x+75, y+88)`. **Keep `deadLetter` at `(300, 402)`:** its box bottom (`402 + 88 = 490`) intentionally sits ~10px **below** the 480 canvas, so it relies on the SVG's `overflow: visible` (gotcha 3) — do **not** "correct" the coordinate to fit it fully on-canvas.

**Edges — 7 directed cubic Béziers.** A standard curve a→b uses horizontal-tangent controls: `M a.x a.y C mx a.y, mx b.y, b.x b.y` where `mx = (a.x + b.x)/2`. The seven edges (from→to, color, flowing?), with the exact `d` strings computed once from the anchors:

1. `created.right → active.left`, color=**created**, **FLOW** — `M 156 220 C 202 220, 202 154, 248 154`
2. `active.right → completed.left`, color=**completed**, **FLOW** — `M 398 154 C 479 154, 479 62, 560 62`
3. `active.right → failed.left`, color=**failed**, **FLOW** — `M 398 154 C 479 154, 479 220, 560 220`
4. `active → cancelled`, color=**cancelled**, **NO flow**, custom `M {active.right.x} {active.right.y+18} C 470 220, 470 360, {cancelled.left.x} {cancelled.left.y}` — `M 398 172 C 470 220, 470 360, 560 378`
5. `failed.bottom → retry.right`, color=**retry**, **NO flow**, custom `M {failed.bottom.x} {failed.bottom.y} C {failed.bottom.x} 420, {retry.right.x+30} 344, {retry.right.x} {retry.right.y}` — `M 635 264 C 635 420, 428 344, 398 344`
6. `retry.left → active.bottom`, color=**active**, **FLOW**, **DASHED** (`stroke-dasharray: 5 5`), custom `M {retry.left.x} {retry.left.y} C 180 344, 180 154, {active.bottom.x} {active.bottom.y}` — `M 248 344 C 180 344, 180 154, 323 198` (the **"retry re-enters active"** loop)
7. `retry.bottom → deadletter.top`, color=**deadletter**, **NO flow** — `M 323 388 C 349 388, 349 402, 375 402`

Each edge = `<path fill="none" stroke="var(--qb-border-strong)" stroke-width="1.5" opacity="0.85">` with `marker-end` → one reusable `<marker id="qb-arrow">` (filled triangle in `--qb-border-strong`, `orient="auto-start-reverse"`); the dashed edge (6) adds the dash array.

**Animation (the important part).**
- **Edge flow.** For each **flowing** edge (1, 2, 3, 6) render `<circle r="4" fill="var(--qb-state-<name>-solid)">` with a child `<animateMotion path="<the edge's EXACT d string>" repeatCount="indefinite">`. **Stagger durations** `dur = 1.6 + (i % 3) * 0.4` s where **`i` is the 0-based index over the *filtered* flowing edges** (the 4 flowing edges → **1.6 / 2.0 / 2.4 / 1.6**), **not** the raw edge index — so tokens read as organic traffic, not a metronome.
- **Active-node pulse.** The active node gets a small solid dot (`--qb-state-active-solid`) with a CSS keyframe **`qb-node-pulse`** animating an expanding, fading `box-shadow` ring: `0% { box-shadow: 0 0 0 0 currentColor } 70% { box-shadow: 0 0 0 7px transparent } 100% { box-shadow: 0 0 0 0 transparent }`, `1.6s ease-out infinite`. **Only** the active node pulses, and **only** when animation is on.
- **Toggle.** `animated` input (default **true**): when **false**, don't render the flowing circles and don't apply the pulse — a static chart for reduced-motion + print/screenshot.

**Nodes (`LifecycleNode`).** Each an absolutely-positioned HTML box at `left: (x/760)*100%`, `top: (y/480)*100%`, `width: (150/760)*100%`. Contents top→bottom: a **header row** (state-colored dot + **sentence-case label** + **state glyph** pushed right), a **large monospace count** (IBM Plex Mono), a **lowercase caption** ("job"/"jobs"). Tinted from its state ramp: `background var(--qb-state-<s>-bg)`, `border 1.5px solid var(--qb-state-<s>-border)`, text/dot/glyph in `--qb-state-<s>-text` / `-solid`. **Selected:** border → `--qb-state-<s>-solid` + a **2px outer ring**; clicking emits `selectState`. Counts `toLocaleString()`-formatted.

**Glyphs (semantic, always paired with color + label — colorblind contract, never color alone):** created `◷`, active `▶`, completed `✓`, failed `✕`, retry `↻`, cancelled `⊘`, deadletter `☠`.

**Tokens.** Dark-first OKLCH, the E3-1 ramp — 4 valued steps `--qb-state-<s>-{bg|border|solid|text}` (plus a reserved `-on`, unused here) (re-stated here so this recipe is self-sufficient):

| state | `-bg` | `-border` | `-solid` | `-text` |
|---|---|---|---|---|
| created (slate) | `oklch(0.290 0.018 250)` | `oklch(0.430 0.024 250)` | `oklch(0.620 0.030 250)` | `oklch(0.790 0.024 250)` |
| active (azure=brand) | `oklch(0.300 0.060 250)` | `oklch(0.470 0.110 250)` | `oklch(0.640 0.150 250)` | `oklch(0.810 0.100 250)` |
| completed (green155) | `oklch(0.300 0.055 155)` | `oklch(0.470 0.095 155)` | `oklch(0.680 0.150 155)` | `oklch(0.830 0.110 155)` |
| failed (red25) | `oklch(0.305 0.075 25)` | `oklch(0.480 0.140 25)` | `oklch(0.620 0.200 25)` | `oklch(0.805 0.135 25)` |
| retry (amber75) | `oklch(0.325 0.055 75)` | `oklch(0.520 0.100 75)` | `oklch(0.770 0.150 75)` | `oklch(0.860 0.105 75)` |
| cancelled (grey) | `oklch(0.260 0.010 250)` | `oklch(0.380 0.012 250)` | `oklch(0.490 0.015 250)` | `oklch(0.620 0.014 250)` |
| deadletter (violet320) | `oklch(0.305 0.085 320)` | `oklch(0.470 0.140 320)` | `oklch(0.585 0.190 320)` | `oklch(0.820 0.120 320)` |

Surfaces & connectors: the hero panel reuses the existing **`--qb-surface`** (`#161b22`) as its background and the existing **`--qb-border`** (`#30363d`) as its 1px edge — **no new surface token**; the connectors use the new **`--qb-border-strong`** (`oklch(0.62 0.02 250)`, added by E3-1). **Panel wrapper:** `background var(--qb-surface)`; `border 1px solid var(--qb-border)`; `radius 7px`; `padding 8px`. **Fonts:** IBM Plex Sans (UI) + IBM Plex Mono (counts). (Token segment is the `JobState` literal — `--qb-state-deadLetter-*` in Queue Boss. **Reinforced:** the design prompt writes `deadletter` lowercase throughout, but in Queue Boss **both the state color-keys and the node `data-testid`s use the `JobState` LITERAL — camelCase `deadLetter`** — so `--qb-state-deadLetter*` resolves and the node testid is `lifecycle-node-deadLetter`, NOT `deadletter`.)

**Annotation callout.** A row **below** the diagram using the annotation state's ramp (`bg`/`border`/`text`) + an `ⓘ` glyph. (Content wiring is E3-4; the component renders whatever `annotation` input it's given.)

**Angular-specific process.** `standalone: true`, no `NgModule`, new control flow (`@for`/`@if`). Inputs via `input()`: `counts` (record state→number), `animated` (bool default `true`), `selected` (state|null), `annotation` (`{state,text}`|null). Output via `output()`: `selectState`. Compute the **node-position map** + **edges array** as constants/computed; compute each edge's `d` string **ONCE**; flowing tokens = `edges.filter(e => e.flow)` each carrying `{ d, colorVar, dur }` — `dur` computed from the **0-based index into this filtered array** (`i = 0..3` → 1.6 / 2.0 / 2.4 / 1.6), **not** the raw edge number. **Template:** one `<svg viewBox="0 0 760 480">` with `<defs>` marker, `@for` edges → `<path>`, `@for` flowing edges → `<circle>` + `<animateMotion>` (bind `[attr.path]` / `[attr.dur]` / `[attr.d]` / `[style.fill]`); overlay node boxes as absolutely-positioned `<div>`s (`@for` the position map) inside a `position: relative` wrapper with `aspect-ratio: 760/480`. Respect `prefers-reduced-motion` (read via a signal / `matchMedia` **once**, **defensively** — guard `typeof window.matchMedia === "function"`, else reduced-motion = `false`; jsdom omits `matchMedia`, so `src/testing/setup-axe.ts` stubs it — see the BLOCKING note below) — gate **BOTH** `<animateMotion>` and the pulse on `animated() && !reducedMotion()`. Put `@keyframes qb-node-pulse` + the token vars in styles.

**GOTCHAS (all load-bearing):**
1. The flowing circle's `animateMotion` `path` **MUST be byte-identical** to its edge's `d` — compute once, share — regenerating separately makes tokens drift off the line.
2. `<animateMotion>` needs `path` on the element (or `<mpath>`); binding `[attr.path]` works but **confirm SMIL restarts cleanly** when `animated` toggles — you may need to **`@if`-remove + re-add** the circles (not hide) so SMIL re-initializes.
3. Keep SVG **`overflow: visible`** so edges 4–6 that bow outside the node band aren't clipped.
4. Node-placement percentages divide by **760/480 (canvas)**, NOT rendered px.
5. SMIL `<animateMotion>` works in Tauri's WebView (Chromium/WebKit) but is the **spottiest** SVG-anim feature; the fallback if ever needed is the **Web Animations API motion path**.

**ACCEPTANCE (of the component in isolation):** a dark panel of 7 tinted nodes wired by arrowed connectors; colored dots continuously stream `created→active`, `active→completed`, `active→failed`, and around the dashed `retry→active` loop at slightly different speeds; the active node pulses; clicking a node emits `selectState` + highlights it; `animated=false` (or reduced-motion) freezes to a clean static diagram; the annotation row recolors to the selected/annotated state.

### End embedded prompt

**Test-setup prerequisite — the `matchMedia` stub (BLOCKING; land it in the RED step).** jsdom does **not** implement `window.matchMedia`, so the hero's `matchMedia('(prefers-reduced-motion: reduce)')` field-initializer throws **`matchMedia is not a function`** the instant a spec mounts the component — making **§E3-2 TDD step 1 fail for the WRONG reason** (a crash, not a missing-behavior red) and cascading into every E3-3/E3-4 spec that mounts the hero. Two mitigations, **both required**:
1. **Add a `window.matchMedia` stub to `src/testing/setup-axe.ts`** (the sole vitest setupFile) — a function returning `{ matches: false, media: query, onchange: null, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {}, dispatchEvent() { return false } }`. This stub is a prerequisite for the **E3-2, E3-3, and E3-4** specs that mount the hero.
2. **Read `matchMedia` defensively in the hero** — guard `typeof window.matchMedia === "function"`; when absent, treat reduced-motion as `false`.

The `matchMedia`-backed `reducedMotion` signal is **owned by E3-2** and is **purity-EXEMPT**: it is a presentation media query, not a Tauri/data service — `ng-declarative-purity` only targets `@tauri-apps/api` / `invoke`, so reading `matchMedia` in the dumb hero does **not** violate the purity gate.

**TDD order (outside-in — component spec first):**
1. **Red:** `lifecycle-diagram.component.spec.ts` renders the component with fixed `counts`; assert seven nodes exist with the right labels/glyphs and `toLocaleString()`-formatted counts, seven `<path>`s with the **exact `d` strings** above, and one `<marker id="qb-arrow">`. Red (no component) → build the static structure → green. **Lock the seven `d` strings in the test** so gotcha 1 (byte-identical paths) is regression-guarded. (Land the `matchMedia` stub in `setup-axe.ts` **first** — otherwise this very step throws `matchMedia is not a function`, a wrong-reason red.)
2. **Flow circles:** with `animated=true` and `reducedMotion=false`, assert one `<circle><animateMotion>` per **flowing** edge (4 of them), each `path` **equal to its edge `d`**, and `dur` following `1.6 + (i%3)*0.4` with **`i` the 0-based index over the filtered flowing edges** → durations **1.6 / 2.0 / 2.4 / 1.6**. Red → green.
3. **Toggle + reduced-motion:** with `animated=false` (or a stubbed `matchMedia` returning reduce), assert **no** flowing circles and **no** pulse class. Red → green. Cover the `@if`-remove-and-re-add re-init path (gotcha 2).
4. **Selection:** clicking / Enter-activating a node emits `selectState(state)`; `selected` input renders the 2px ring on that node (not color-only). Red → green.
5. **Annotation:** an `annotation` input renders the callout row recolored to that state's ramp with the `ⓘ` glyph. Red → green.
6. `vitest-axe` clean (structure/labels/keyboard); nodes keyboard-focusable + labelled; glyph+label present on every node.

**Verification:** `ng test --no-watch --no-progress` (Node 24, incl. `vitest-axe`) · `npm run lint`. (No e2e here — the hero is inert until E3-3 mounts it; the *live* flow/selection e2e lives in E3-3.) Note in the PR body that SMIL frame-flow is verified in the real webview (jsdom can't animate SVG).

**Gate notes:** `tdd-evidence` — the exact-`d`-string lock + the flowing-circle-count + the toggle-freeze cases are the evidence. `ng-declarative-purity` — inputs/outputs only, no service, no `invoke`, no facade; the gate greps for stray **production** `@tauri-apps/api` imports. The `reducedMotion` signal reads `matchMedia` (a presentation media query) and is **purity-EXEMPT** — the gate only targets `@tauri-apps/api` / `invoke`. `a11y-audit` — `vitest-axe` for structure/labels/keyboard + the colorblind glyph contract; contrast comes from the E3-1 ramp (documented there). No Rust/CI → no `rust-mutation-coverage`/`xplat-build-smoke`.

**Done when:** `LifecycleDiagramComponent` is a standalone dumb component matching the embedded prompt — 7 tinted nodes, 7 exact-`d` arrowed edges, flowing tokens on edges 1/2/3/6 with the staggered `dur`, the active-node pulse, `animated=false`/reduced-motion freeze, node selection emitting `selectState` + 2px ring, the annotation callout recolor — pure Angular + inline SVG + CSS, no libraries, axe clean, the seven paths regression-locked.

---

## E3-3 — Home container + live wiring + landing route  *(P0; blocked by: E3-2)*

**Intent:** Wire the hero to live data and make it the app's front door. A new `LifecycleHomeContainerComponent` (mirroring `overview-container`) injects `ConnectionsFacade` + `QueuesFacade`, folds `queues()` into the aggregate per-state `computed`, and binds it to `LifecycleDiagramComponent`; the new home screen becomes the **default route** (`"" → home`), with `/overview`, `/jobs`, `/lifecycle`, `/connect` kept as secondary nav; the sandbox is connected on entry so the hero flows live; a zero/empty state renders until counts arrive. **E3-3 also OWNS the integrated select→teaching wiring:** it binds the hero's `selectState` to a `selected` signal and feeds the hero's `selected` + `annotation` inputs so a selected node surfaces its explanation via the hero's **built-in annotation callout (from E3-2)** — the baseline teaching surface — so E3-3 does **not** hard-depend on E3-4. When E3-4's richer explainer lands, the container renders it in that teaching slot.

**Files/modules:** `src/app/features/lifecycle/lifecycle-home-container.component.ts` (new container: `standalone`, `OnPush`, injects both facades; the **aggregate `computed`**; an `effect()` calling `queues.connect(activeConnectionId())` on entry / when the active connection changes — like `overview-container` but **without** the `enter-sandbox` gate so the hero animates immediately; binds `[counts]` / `[animated]` / `[selected]` / `[annotation]` to the hero and handles `(selectState)` — **wiring `selectState` → a `selected` signal and feeding `selected` + `annotation` so a selected node surfaces its explanation via the hero's built-in annotation callout**); `src/app/app.routes.ts` (change `{ path: "", … redirectTo: "overview" }` → `{ path: "", pathMatch: "full", redirectTo: "home" }` + `{ path: "home", component: LifecycleHomeContainerComponent, title: "Lifecycle" }`; keep the other four routes); nav affordance (`src/app/shell/*` — a link to `home`); update `tests/e2e/sandbox.e2e.ts` — (a) the **new** home-hero specs (launch → hero renders → counts flow → clicking a node selects it **and its annotation/explanation becomes visible**), and (b) **because `"" → home` is now the default landing, any existing `sandbox.e2e.ts` specs that assumed launch lands on `/overview` must first navigate there via the `nav-overview` link** — they can no longer assume the launch route is `/overview`. **`data-testid`s:** `home-hero`, `nav-home`, `nav-overview`, `lifecycle-node-<state>` (reuse the hero's node testids if it already emits them; `<state>` is the `JobState` literal, e.g. `lifecycle-node-deadLetter`).

**Contract — the aggregate fold (spec §3.2):**
```
// iterate the seven JobState keys EXPLICITLY — do NOT import JOB_STATES
// from lifecycle.component.ts (it is unexported there).
const STATES: JobState[] = ["created","active","completed","failed","cancelled","retry","deadLetter"];
aggregate = computed(() => {
  const totals = { created:0, active:0, completed:0, failed:0, cancelled:0, retry:0, deadLetter:0 };
  for (const entry of queues.queues())
    for (const s of STATES) totals[s] += entry.countsByState[s] ?? 0;
  return totals;                       // dense Record<JobState, number>
});
```
- The fold lives in the **container**, not the facade or the dumb hero.
- Empty `queues()` ⇒ all zeros ⇒ the hero renders its legible zero/empty state (still teaching the shape).
- On entry, `queues.connect(activeConnectionId())` opens the counts channel so the sandbox (or connected pg-boss) streams; the fold updates live and rekeys when `activeConnectionId()` changes.
- **Iterate the seven `JobState` keys explicitly** — use a local `const STATES: JobState[]` (or `Object.keys(totals)`), **NOT** the unexported `JOB_STATES` in `lifecycle.component.ts` (it is not exported; don't import it).
- **Select→teaching wiring (E3-3 OWNS it).** The container binds the hero's `selectState` output to a `selected` signal and feeds the hero's `selected` + `annotation` inputs, so a selected node surfaces its explanation using the hero's **built-in `annotation` callout (from E3-2)** as the baseline teaching surface — E3-3 does **not** hard-depend on E3-4 landing first. **E3-3's baseline teaching text is a MINIMAL LOCAL per-state string owned by the home container** (e.g. a short inline `Record<JobState, string>` or the existing state label) — E3-3 must **NOT** import E3-4's `LIFECYCLE_COPY`/explainer to green its "annotation visible on select" assertion; E3-4's richer copy/explainer is swapped into the teaching slot when it lands. When E3-4's richer explainer exists, the container renders it in that teaching slot.

**TDD order (outside-in — e2e is the outer red, then the container unit):**
1. **e2e red:** extend `sandbox.e2e.ts` — launch the app, assert it lands on `home-hero` (not `/overview`), that node counts become non-zero as the sandbox streams, and that clicking a `lifecycle-node-<state>` selects it (ring visible) **AND its annotation/explanation becomes visible** (the baseline teaching surface). Also **update the existing overview-facing specs to reach `/overview` via `nav-overview`** now that launch lands on the hero. Red (no home route/container).
2. **Container unit:** over mocked facades, assert the aggregate `computed` sums `countsByState` across multiple `QueueCountEntry`s (incl. sparse/missing keys → treated as 0) into the dense record, that it binds to the hero's `counts`, and that on entry it calls `queues.connect(activeConnectionId())`. Feed a `(selectState)` and assert the container tracks `selected`. Red → green.
3. **Routing:** a route spec asserts `""` resolves to `home` and the other four routes still resolve. Red → green.
4. Green the e2e.

**Verification:** `ng test --no-watch --no-progress` (Node 24) · `npm run lint` · `npm run e2e` (launch → hero → live counts → selectable node). `vitest-axe` on the container.

**Gate notes:** `tdd-evidence` — the launch-lands-on-hero + counts-flow + node-selectable e2e is the outer red; the aggregate-fold unit test is the inner red. `ng-declarative-purity` — the container injects facades and assembles a `computed`/`effect` (allowed for containers); the hero stays dumb; **no `invoke`** added (it reuses `QueuesFacade.connect`); the gate greps for stray `@tauri-apps/api`. `a11y-audit` — `vitest-axe` on the home container; keyboard reach to the hero + nav. No Rust/CI → no `rust-mutation-coverage`/`xplat-build-smoke`.

**Done when:** the app launches on the animated hero (default route `"" → home`); the seven nodes show the live all-queues aggregate per-state counts and the tokens flow once the sandbox streams; **clicking a node selects it AND surfaces its explanation via the hero's built-in annotation callout** (baseline teaching, no dependency on E3-4); `/overview`/`/jobs`/`/lifecycle`/`/connect` remain reachable via nav **(the existing `sandbox.e2e.ts` specs reach `/overview` via `nav-overview`)**; a zero/empty state shows until counts arrive; only the interface service touches Tauri; axe clean.

---

## E3-4 — Teaching layer (annotation + per-state explanations)  *(P0; blocked by: E3-2)*

**Intent:** Deliver the **reusable richer teaching surface**: a **dumb, reusable per-state explainer/popover component** (input = the selected `JobState` + its copy; keyboard-operable, `Esc`-dismiss, focus-managed, `aria`-labelled, `vitest-axe`-clean) **plus** the `state → explanatory copy` content map (data — explaining each pg-boss state + the retry / dead-letter / cancelled semantics). **Genuinely parallel with E3-3** (both hang off E3-2's dumb contract): E3-3 already ships the baseline select→teaching wiring via the hero's built-in annotation callout, so **E3-4 does not need E3-3's container** — its integration test drives the explainer against a **STUB host**, not `LifecycleHomeContainerComponent`. When E3-4 lands, E3-3's container renders this explainer in its teaching slot.

**Files/modules:** `src/app/features/lifecycle/lifecycle-teaching.ts` (the typed **state→copy map**: for each `JobState`, `{ title, body }` explaining the pg-boss meaning + retry/dead-letter/cancelled semantics); `src/app/features/lifecycle/state-explainer.component.ts` (a **dumb, reusable** popover component: `input()` the selected `JobState` + its copy, renders the explanation; `output()` a dismiss event; focus-managed, `Esc`-dismiss, `aria`-labelled); a **spec-only STUB host** that mounts the explainer and drives `selected` — E3-4's integration test runs against this **stub**, **NOT** E3-3's `LifecycleHomeContainerComponent` (E3-3 already owns the container wiring; E3-4 stays independently landable). **The stub host provides a FOCUSABLE originating-node stand-in** (a `<button>` / `tabindex=0` element) so the explainer's focus-return-on-`Esc` (focus returns to the originating node) is testable in the stub context. Co-located specs + `vitest-axe`. **`data-testid`s:** `state-explainer`, `state-explainer-dismiss`, `state-annotation`.

**Contract:**
- **Content is data.** `LIFECYCLE_COPY: Record<JobState, { title: string; body: string }>` — the single source. Example semantics to encode: **created** = "queued, waiting to run"; **active** = "in flight, a worker holds it"; **completed** = "finished successfully, output recorded"; **failed** = "terminally failed, nowhere to go"; **cancelled** = "terminated before completion"; **retry** = "failed but retries remain — waiting on a backoff timer before re-entering active"; **deadLetter** = "failed and routed to another queue for triage (pg-boss has no deadLetter state — it's derived)".
- **Per-state popover** — opening (a node's `selectState`) shows `state-explainer` for that state; it is **keyboard-accessible**: focus moves into the popover on open, `Esc` dismisses and returns focus to the originating node, it is `aria`-labelled (`role`/`aria-label` naming the state), and dismiss-on-outside-click. Recolored to the state's ramp.
- **Annotation callout** — the hero's `annotation` input (`{ state, text }`), sourced from the same copy map, renders the below-diagram callout recolored to that state (**E3-2** renders the callout; **E3-3** wires it to selection as the baseline; **E3-4** supplies the shared copy map both the callout and the explainer draw from).
- **No guided-walkthrough** — annotations + per-state popovers only (spec §4/§5).

**TDD order (outside-in — the explainer component + a11y first):**
1. **Red:** `state-explainer.component.spec.ts` — given a state + its copy, renders the `title`/`body`; `vitest-axe` clean; it is `aria`-labelled. Red → green.
2. **Focus management:** on open, focus is inside the popover; **`Esc`** emits dismiss and (asserted at the container/integration level) returns focus to the node; outside-click dismisses. Red → green. (Focus-return is the a11y crux — test it explicitly.)
3. **Content map:** a spec asserts every `JobState` has a `title` + non-empty `body` and that retry/dead-letter/cancelled bodies carry their distinguishing semantics (e.g. deadLetter body mentions "routed"/"derived"; retry mentions "backoff"). Red → green.
4. **Wiring (against a STUB host):** a spec-only stub host mounts the explainer and drives `selected`; an integration test drives `select → explainer visible → Esc → dismissed + focus back`. Red → green. **Do NOT reach into E3-3's `LifecycleHomeContainerComponent`** — E3-4 is proven independently with the stub.

**Verification:** `ng test --no-watch --no-progress` (Node 24) — the explainer + copy map + stub-host integration + `vitest-axe` — · `npm run lint`. **No `npm run e2e` here** — the in-app "select a node → explanation appears" e2e belongs to **E3-3** (runnable only in E3-3's host); E3-4 is proven against a STUB host.

**Gate notes:** `tdd-evidence` — the content-map completeness + the focus-management (open-focus / Esc-dismiss-and-return / outside-click) cases are the evidence. `ng-declarative-purity` — explainer is dumb (inputs/outputs); content is a plain typed constant; no service, no `invoke`. `a11y-audit` — this is the a11y-heavy child: `vitest-axe` **plus** explicit assertions on **focus management + Esc-dismiss + labelling** (jsdom can drive focus/keyboard). No Rust/CI → no `rust-mutation-coverage`/`xplat-build-smoke`.

**Done when:** the **reusable** per-state explainer component **and** the typed `state→copy` map **exist**, are **keyboard-accessible** (focus moves in on open, `Esc` dismisses and returns focus to the originating node, `aria`-labelled) **and axe-clean**, cover each pg-boss state + the retry/dead-letter/cancelled semantics, and **honor the selection contract — proven against a STUB host** (not E3-3's container); all content lives in one typed state→copy map; components dumb.

---

## Drive order

`E3-1 → E3-2 → { E3-3, E3-4 }`, with **E3-3 and E3-4 genuinely parallel**. **E3-1** (tokens + fonts) is the foundation — land it first so the OKLCH ramp and IBM Plex are available; **E3-2** (the dumb animated hero) consumes the ramp and blocks everything downstream. Once E3-2 is green, **E3-3** (home container + live wiring + landing route **+ the integrated select→teaching wiring**) and **E3-4** (the **reusable** richer per-state explainer + copy map) run **in parallel** — both only depend on the hero's dumb contract (`counts`/`animated`/`selected`/`annotation`/`selectState`). **E3-3 owns the integrated wiring** and uses the hero's **built-in annotation callout (from E3-2)** as the baseline teaching surface, so it does **not** wait on E3-4; **E3-4 proves its explainer against a STUB host** (not E3-3's container), so it does **not** wait on E3-3. **Neither blocks the other's core — no cross-child DAG edge.** Four children total; FE-only, so no `xplat-build-smoke`/`rust-mutation-coverage` on any of them — the gates are `tdd-evidence`, `ng-declarative-purity`, `a11y-audit` throughout.
