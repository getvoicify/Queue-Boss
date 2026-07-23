# Epic 3 — Hero Animation & Teaching — Design

**Product:** Queue Boss (getvoicify/Queue-Boss) — teaching-first, read-only desktop inspector for background job queues (Tauri 2 + Angular 22, dark-first).
**Epic:** 3 of the MVP program (E1 Skeleton & Sandbox ✅ → E2 pg-boss v10 Read Path ✅ → **E3 Hero + Teaching** → E4 Cross-platform Release).
**Source:** [Queue Boss MVP PRD](../../queue-boss-mvp-prd.md) (Draft v0.2) — the product brief of record. This spec scopes **Milestone 3** ("Hero + Teaching: the animated job-lifecycle hero as the app's landing experience, plus a per-state teaching layer, fed by the live aggregate counts the read path already streams"). The PRD's **animated lifecycle hero is P0** — E2's FE children (E2-5 connect UI, E2-6 job explorer/detail) deliberately deferred it to E3. Features realized here: the **animated lifecycle hero** (PRD P0 hero), the **teaching-annotation + per-state explanation** layer, and the **unified OKLCH state-token** foundation both consume.
**Predecessor:** [E2 pg-boss Read Path Design](2026-07-23-pgboss-read-path-design.md) + [Runbook](../plans/2026-07-23-pgboss-read-path-plan.md), merged to `main` @ `831bb92`. E2 shipped the `PgBossBackend` v10 adapter, runtime connect/disconnect, per-connection status, active-connection rekeying, and the job explorer/detail — all read-only, all fed by the E1 aggregate-counts poller. E3 turns the counts E2 already streams into the product's front door.
**Board:** org Project #3. **Planning:** self-hosted in this repo.

---

## 1. Problem this epic solves

E2 made Queue Boss *correct* — point it at a real pg-boss v10 database and read it safely — but it still opens on a bare `/overview` table. The PRD's headline promise is a **teaching-first** inspector: the thing an OSS newcomer sees first should *explain the pg-boss job lifecycle by animating it*, not present a spreadsheet. That hero is the PRD's P0 differentiator, and it is the one deliverable E2 explicitly punted (E2-6's note: "the animated hero lifecycle stays in E3").

Three things must become true, and each is load-bearing:

1. **An animated lifecycle hero that greets on launch.** A standalone `LifecycleDiagramComponent` renders the seven pg-boss job states as a dark panel of tinted nodes wired by arrowed connectors, with colored tokens continuously streaming along the edges (created→active, active→completed, active→failed, and around the dashed retry→active loop) and the active node pulsing. It is **pure Angular + inline SVG + CSS, no libraries** — SVG-native `<animateMotion>` for edge flow, CSS keyframes for the pulse. It becomes the app's **default landing screen**.

2. **The hero shows *live* aggregate counts, not a canned loop.** The seven nodes carry the real per-state totals **aggregated across every queue of the active connection** — a `computed` that folds `QueuesFacade.queues()` into per-state sums. Connecting the sandbox (E1's live fake) makes the tokens flow over real numbers; connecting a pg-boss database (E2) shows that database's lifecycle. No aggregate exists today — `countsByState` is a sparse `Partial<Record<JobState,number>>` per queue — so E3 introduces the fold.

3. **A teaching layer that answers "why is this job here".** Clicking a node selects it and surfaces a per-state explanation — what the state *means* in pg-boss terms, plus the retry / dead-letter / cancelled semantics — as a keyboard-accessible popover, alongside an annotation callout the hero can point at a specific state. Content is authored as **data** (a state→copy map), not markup.

Underneath all three sits a **token debt**: the existing `--qb-state-*` family is seven flat hex values sufficient for a colored dot, but a tinted-node hero needs a *ramp* per state (surface, border, solid, text). E3 unifies the app on an **OKLCH ramp per state — the 4 valued steps `bg`/`border`/`solid`/`text`, plus a *reserved* `-on`** — and routes the existing consumers (lifecycle dots, job-list rows) onto it without visual regression.

Everything E3 ships stays **read-only** and consumes only data the E1/E2 backends already stream (aggregate counts). No new backend, no new Tauri command, no write path.

## 2. Success criteria (epic is "done" when…)

- **On launch the app opens on the animated hero.** The default route (`""`) resolves to a new home/landing screen, not `/overview`; `/overview`, `/jobs`, `/lifecycle`, `/connect` remain reachable as secondary nav.
- **Connecting the sandbox makes the diagram come alive:** colored tokens stream along the flowing edges at slightly different speeds (organic, not metronomic), and the **active** node pulses. Connecting a pg-boss database shows *that* connection's lifecycle.
- **The seven nodes show live aggregate per-state counts** — the sum of `countsByState[state]` across all of the active connection's queues, `toLocaleString()`-formatted — updating as the poller streams.
- **Clicking a node selects it and surfaces its teaching explanation** (per-state popover: the pg-boss meaning + retry/dead-letter/cancelled semantics); the annotation callout recolors to the selected/annotated state.
- **`animated=false` or `prefers-reduced-motion` freezes to a clean static diagram** — no flowing circles, no pulse — suitable for reduced-motion users and for print/screenshot; the diagram is fully legible and correct when frozen.
- **State tokens are unified** on the OKLCH ramp (the 4 valued steps `bg`/`border`/`solid`/`text`, plus a reserved `-on`); the lifecycle dots and job-list rows render on it with **no visual regression**, and `StateColorDirective` still resolves `--qb-state-color`.
- **Only the interface service touches Tauri** (`ng-declarative-purity`); the hero, home container, and teaching layer are dumb components + a facade `computed`. Every child ships via outside-in TDD (`tdd-evidence`) and is **axe-clean** in jsdom (`a11y-audit`).

## 3. Chosen architecture

### 3.1 The animated hero — native SVG `<animateMotion>` + CSS, no libraries

The hero is a standalone Angular 22 `LifecycleDiagramComponent` built from **pure Angular + inline SVG + CSS** — no D3, no animation library. Two motion mechanisms only: SVG-native `<animateMotion>` moves a token circle along each *flowing* edge; a CSS `@keyframes` ring pulses the active node. Both are gated on `animated() && !reducedMotion` so the whole thing freezes cleanly.

**Coordinate system.** A fixed internal canvas **760×480**, `viewBox="0 0 760 480"`, `width="100%"` (scales to any container). Nodes are **absolutely-positioned HTML boxes overlaid on the SVG**, placed by **percentages of 760/480** — dividing by the *canvas* dimensions (not rendered px) is what glues the boxes to their SVG anchors at every width. Node box is **150×88**. The wrapper is `position: relative` with `aspect-ratio: 760/480`; the SVG keeps **`overflow: visible`** so the edges that bow outside the node band (edges 4–6) are not clipped. The **deadLetter** node stays at top-left `(300, 402)`: its box bottom (`402 + 88 = 490`) sits ~10px **below** the 480 canvas by design, so it too relies on `overflow: visible` — do **not** "correct" the coordinate to fit it fully on-canvas.

**Node top-left (x,y) and anchor helpers** (`right-middle = x+150, y+44`; `center = x+75, y+44`; `left-middle = x, y+44`; `top-middle = x+75, y`; `bottom-middle = x+75, y+88`):

| Node | top-left (x,y) | left-mid | right-mid | top-mid | bottom-mid |
|---|---|---|---|---|---|
| created | (6, 176) | (6, 220) | (156, 220) | (81, 176) | (81, 264) |
| active | (248, 110) | (248, 154) | (398, 154) | (323, 110) | (323, 198) |
| retry | (248, 300) | (248, 344) | (398, 344) | (323, 300) | (323, 388) |
| completed | (560, 18) | (560, 62) | (710, 62) | (635, 18) | (635, 106) |
| failed | (560, 176) | (560, 220) | (710, 220) | (635, 176) | (635, 264) |
| cancelled | (560, 334) | (560, 378) | (710, 378) | (635, 334) | (635, 422) |
| deadLetter | (300, 402) | (300, 446) | (450, 446) | (375, 402) | (375, 490) |

**Edges — 7 directed cubic Béziers.** A standard curve a→b uses horizontal-tangent controls `M a.x a.y C mx a.y, mx b.y, b.x b.y` with `mx = (a.x + b.x)/2`. The exact `d` strings (computed once, from the anchors above):

| # | from → to | color token | flow? | `d` |
|---|---|---|---|---|
| 1 | created.right → active.left | created | **FLOW** | `M 156 220 C 202 220, 202 154, 248 154` |
| 2 | active.right → completed.left | completed | **FLOW** | `M 398 154 C 479 154, 479 62, 560 62` |
| 3 | active.right → failed.left | failed | **FLOW** | `M 398 154 C 479 154, 479 220, 560 220` |
| 4 | active → cancelled (custom) | cancelled | no | `M 398 172 C 470 220, 470 360, 560 378` |
| 5 | failed.bottom → retry.right (custom) | retry | no | `M 635 264 C 635 420, 428 344, 398 344` |
| 6 | retry.left → active.bottom (custom, **DASHED**) | active | **FLOW** | `M 248 344 C 180 344, 180 154, 323 198` |
| 7 | retry.bottom → deadLetter.top | deadLetter | no | `M 323 388 C 349 388, 349 402, 375 402` |

Edge 4 is the custom "active cancels" bow (`active.right.y + 18`); edge 5 the "failed retries" hook; edge 6 the **"retry re-enters active"** loop, rendered `stroke-dasharray: 5 5`. Every edge is a `<path fill="none" stroke="var(--border-strong)" stroke-width="1.5" opacity="0.85">` with `marker-end` pointing at one reusable `<marker id="qb-arrow">` (filled triangle in `--border-strong`, `orient="auto-start-reverse"`); the dashed edge adds the dash array.

**Animation — the important part.**
- **Edge flow.** For each *flowing* edge (1, 2, 3, 6) render a `<circle r="4" fill="var(--qb-state-<name>-solid)">` containing `<animateMotion path="<the edge's EXACT d>" repeatCount="indefinite">`. Durations stagger: **`dur = 1.6 + (i % 3) * 0.4` s** where **`i` is the 0-based index over the *filtered* flowing edges** (the 4 flowing edges → durations **1.6 / 2.0 / 2.4 / 1.6**), **not** the raw edge index — so the tokens read as organic traffic, not a metronome.
- **Active-node pulse.** The active node carries a small solid dot (`--qb-state-active-solid`) with a CSS keyframe **`qb-node-pulse`** animating an expanding, fading `box-shadow` ring: `0% { box-shadow: 0 0 0 0 currentColor } 70% { box-shadow: 0 0 0 7px transparent } 100% { box-shadow: 0 0 0 0 transparent }`, `1.6s ease-out infinite`. **Only** the active node pulses, and **only** when animation is on.
- **Toggle.** `animated` input (default `true`): when `false`, the flowing circles are **not rendered** and the pulse is **not applied** — a static chart for reduced-motion and print/screenshot.

**Glyphs (colorblind contract — never color alone).** Each node pairs its color+label with a semantic glyph: created `◷`, active `▶`, completed `✓`, failed `✕`, retry `↻`, cancelled `⊘`, deadLetter `☠`.

**Load-bearing gotchas** (carried verbatim into runbook §E3-2):
1. **The flowing circle's `animateMotion` path must be byte-identical to its edge's `d`** — compute the `d` once and share it; regenerating it separately makes the token drift off the line.
2. **SMIL must re-initialize on toggle** — binding `[attr.path]`/`[attr.d]` works, but `<animateMotion>` may not restart cleanly when `animated` flips. `@if`-**remove-and-re-add** the circles (not hide them) so SMIL re-inits.
3. **`overflow: visible`** on the SVG so edges 4–6 aren't clipped.
4. **Node placement divides by 760/480** (the canvas), not rendered px.
5. SMIL `<animateMotion>` works in Tauri's WebView but is the spottiest SVG-anim feature; the documented fallback (if ever needed) is the Web Animations API motion path.

### 3.2 The aggregate-counts data path — a `computed` fold over `QueuesFacade.queues()`

The hero's `counts` input is a per-state total across **all queues of the active connection**. E2 exposes `QueuesFacade.queues: Signal<QueueCountEntry[]>` (`src/app/core/facades/queues.facade.ts`), each entry carrying a **sparse** `countsByState: Partial<Record<JobState,number>>` (`src/app/core/models/queue.ts`). No aggregate exists. E3 introduces a `computed` in the home container that folds:

```
for each JobState s:  total[s] = Σ over entries of (entry.countsByState[s] ?? 0)
```

yielding a dense `Record<JobState, number>` fed straight to `LifecycleDiagramComponent.counts`. The fold lives in the **container** (not the facade or the dumb component), mirroring E2's `overview-container` pattern (`computed` view-state assembled from injected facades, bound `[counts]="aggregate()"` to a dumb child). Because it reads only the existing `queues()` signal, it updates live as the poller streams and rekeys automatically when `ConnectionsFacade.activeConnectionId` changes — no new backend surface. Until counts arrive (empty `queues()`), every state folds to `0` and the hero renders its zero/empty state (a legible static diagram of zeros, still teaching the lifecycle shape).

### 3.3 The OKLCH token migration — one ramp, app-wide

Today `src/styles/tokens.css` `:root` defines seven **flat hex** `--qb-state-*` values (created `#6e7781`, active `#2f81f7`, completed `#3fb950`, failed `#f85149`, cancelled `#8b949e`, retry `#d29922`, deadLetter `#a371f7`), consumed by `StateColorDirective` (`src/app/shared/directives/state-color.directive.ts`), which emits `--qb-state-color: var(--qb-state-<state>)` + a `data-state` attribute on the lifecycle dots and job-list rows. A flat hex is enough for a dot but not for a tinted node (which needs a background, a border, a solid fill, and readable text). E3-1 migrates `--qb-state-*` to an **OKLCH ramp per state — the 4 valued steps `--qb-state-<name>-{bg | border | solid | text}` plus a *reserved* `--qb-state-<name>-on`** (a named slot with no value shipped this epic — see §7) — and **keeps a primary `--qb-state-<name>` aliased to the `solid` step** so `StateColorDirective` and every existing consumer keep resolving. The exact OKLCH values (the four valued steps) are in runbook **§E3-1** (and re-embedded in **§E3-2**).

Two constraints anchor the migration:
- **The state segment stays the `JobState` literal.** The directive resolves `var(--qb-state-${appStateColor()})`, and `appStateColor()` returns the `JobState` string — so the dead-letter tokens are `--qb-state-deadLetter-*` (**camelCase**, matching `JobState.deadLetter`), not `deadletter`. The design-prompt's lowercase "deadletter" is informal; the Queue Boss token segment is the literal.
- **`--qb-status-*` is a different family** (connection-status dots — connected/connecting/error) and is **left untouched**. The ramp touches only `--qb-state-*`.

Existing consumers (lifecycle dots, job-list rows) migrate onto the ramp where it adds clarity — a dot can become `solid`, a row can gain a `bg`/`border` tint — with **no visual regression** as the acceptance bar. `a11y-audit` for E3-1 documents the measured contrast ratios: each `-text` on its `-bg` must clear **AA 4.5:1**; the `solid` fill used for glyph/dot against `bg` must clear the 3:1 non-text bar.

### 3.4 The teaching layer — annotations + keyboard-accessible per-state explanations

Teaching is two coordinated affordances, both driven by the hero's `selectState` output and authored as **data**:
- **The annotation callout** ("why is this job here") — a row below the diagram that the hero can point at a specific state, recoloring to that state's ramp (`bg`/`border`/`text`) with an `ⓘ` glyph. Fed by the hero's `annotation` input (`{ state, text } | null`).
- **Per-state explanations** — clicking (or keyboard-activating) a node selects it and surfaces a popover/tooltip explaining that pg-boss state plus the **retry / dead-letter / cancelled semantics** (e.g. "Failed = terminally failed, nowhere to go" vs "Dead-letter = failed and routed to another queue for triage"; "Retry = waiting for a backoff timer before re-entering active"; "Cancelled = terminated before completion").

Content is a **state→copy map** (a typed constant), not markup — one source of truth the popover and the annotation share. The popover is **keyboard-accessible**: focus moves into it on open, `Esc` dismisses and returns focus to the node, it is `aria`-labelled and dismiss-on-outside-click. There is **no full guided-walkthrough mode** (deferred — see §5). This is the annotations + per-state popovers scope only.

### 3.5 Home-landing placement & routing — a new default screen

The hero ships as a **new home/landing screen**, made the app's **default route**, rather than by replacing `/lifecycle`. `app.routes.ts` today maps `""` → `redirectTo: "overview"` and mounts `/lifecycle → LifecycleComponent` directly. E3-3:
- adds a new `LifecycleHomeContainerComponent` (a container mirroring `overview-container.component.ts` — OnPush, injects `ConnectionsFacade` + `QueuesFacade`, assembles the §3.2 aggregate `computed`, binds it to the dumb `LifecycleDiagramComponent`);
- changes the default route to `"" → home` (the hero greets on launch);
- keeps `/overview`, `/jobs`, `/lifecycle`, `/connect` as **secondary nav** (a nav affordance to the home screen is added);
- **connects the active connection on entry** so the hero flows live — for the sandbox that means calling `QueuesFacade.connect(activeConnectionId())` on init (the home screen greets and animates immediately, unlike `/overview` which gates the sandbox behind an "Enter Sandbox" click), rendering the zero/empty state until counts arrive.

The old inert `LifecycleComponent` (a dumb `<dl>` of the seven states, currently wired to nothing → renders 0) stays mounted at `/lifecycle` as the plain secondary read; the animated hero is the new front door.

### 3.6 Accessibility & reduced-motion — greenfield, built in from the start

There is **no `prefers-reduced-motion` handling anywhere today** (greenfield). E3 reads the preference **once** (a signal over `matchMedia('(prefers-reduced-motion: reduce)')`, read **defensively** — guard `typeof window.matchMedia === "function"`, else treat reduced-motion as `false`) and gates **both** `<animateMotion>` and the pulse on `animated() && !reducedMotion()`. Because jsdom omits `matchMedia`, the sole vitest setupFile `src/testing/setup-axe.ts` must **stub `window.matchMedia`** (see runbook §E3-2) or the field-initializer throws `matchMedia is not a function` and the E3-2/E3-3/E3-4 specs that mount the hero fail for the wrong reason. This `matchMedia`-backed `reducedMotion` signal is **owned by E3-2** and is **purity-EXEMPT** — a presentation media query, not a Tauri/data service (`ng-declarative-purity` only targets `@tauri-apps/api`/`invoke`). The static frozen diagram is the reduced-motion experience and is fully legible. Beyond motion:
- **Colorblind contract** — every node pairs color with a **glyph and a label**; state is never conveyed by color alone (§3.1 glyphs).
- **Node interaction** — nodes are keyboard-focusable and activate `selectState` on Enter/Space; the selected node gets a 2px outer ring (not color-only).
- **Teaching popover** — focus management + `Esc`-dismiss + labelled (§3.4).
- **axe** — `vitest-axe` in jsdom guards structure/labels/keyboard on the hero, the home container, and the teaching layer (`expect(await axe(el)).toHaveNoViolations()`, setup `src/testing/setup-axe.ts`); color-contrast is documented per §3.3 and follows E2's real-webview/manual-record pattern for the runtime check.

## 4. Rejected alternatives

- **A JS animation library / D3 for the hero** — rejected. The motion is a handful of tokens gliding along fixed Béziers plus one pulsing ring; SVG-native `<animateMotion>` + a CSS keyframe deliver it **declaratively and GPU-cheaply** with zero dependencies, keeping the bundle small and the component pure Angular. D3's selection/transition model would fight Angular's rendering and add weight for no gain. Chosen: native SVG + CSS.
- **Replacing the existing `/lifecycle` route with the hero** — rejected. The PRD wants the hero to **greet on launch**; bolting it onto a secondary route buries it. A new **default-landing home screen** puts it first while `/lifecycle` (and `/overview`, `/jobs`) survive as secondary reads. Chosen: new home, made default.
- **Per-queue node counts (a node per queue, or a queue selector)** — rejected for the hero. The teaching story is the *lifecycle*, which is per-state, not per-queue; the honest headline number is the **all-queues aggregate** per state. Per-queue drill-down is a future enhancement, not the hero. Chosen: the all-queues aggregate fold (§3.2).
- **A full guided-walkthrough / "learn mode"** — deferred. A step-through tour is real design + build; E3's teaching scope is the **annotation callout + keyboard-accessible per-state popovers**, which already answer "why is this job here" and explain every state. The walkthrough is post-E3. Chosen: annotations + per-state popovers.
- **Keeping the flat-hex `--qb-state-*` and hand-tinting the hero locally** — rejected. The hero needs a bg/border/solid/text ramp; encoding it inline in the component forks the color system and guarantees drift from the dots/rows. A single **app-wide OKLCH ramp** (with `--qb-state-<name>` aliased to `solid` for back-compat) unifies every consumer. Chosen: the OKLCH ramp migration — 4 valued steps (`bg`/`border`/`solid`/`text`) plus a reserved `-on` (§3.3).
- **Web Animations API (WAAPI) motion path instead of SMIL** — held as the documented fallback, not the default. `<animateMotion>` is declarative, lives in the template, and needs no per-frame JS; it works in Tauri's WebView. WAAPI's `MotionPath` is the escape hatch if a WebView ever regresses (§3.1 gotcha 5), not the primary.

## 5. Out of scope (this epic)

The **full guided-walkthrough / "learn mode"** (deferred; E3 ships annotations + per-state popovers only); **per-queue hero drill-down** / a per-queue node view / a queue selector on the hero (the hero is the all-queues aggregate); a **light theme** (dark-first stays; the OKLCH ramp is authored dark-only for now); **any write path** (the app is read-only — unchanged from E1/E2); any **new backend/Tauri command** (E3 consumes only the existing aggregate-counts stream); throughput sparklines / historical charts; **cross-platform packaging, signing, README, demo assets** (E4). The `on` step of the ramp is authored but not yet consumed by a component in E3 (reserved for text-on-solid surfaces — see §7).

**Feature-flag policy:** N/A for this epic (as E1/E2). The org Flagsmith gating policy targets the gangan product; Queue Boss is a standalone desktop MVP with no flag infrastructure and no live release to gate.

## 6. Children & dependency graph

Contract-first: the tokens precede the component that consumes them; the component precedes its live wiring and its teaching layer.

| # | Child | Blocked by | Prio | Gates |
|---|-------|-----------|------|-------|
| E3-1 | State-token OKLCH ramp + IBM Plex fonts (foundation) | — | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |
| E3-2 | `LifecycleDiagramComponent` (dumb animated hero) | E3-1 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |
| E3-3 | Home container + live wiring + landing route + baseline select→teaching wiring | E3-2 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |
| E3-4 | Reusable per-state explainer + copy map (richer teaching layer) | E3-2 | P0 | tdd-evidence, ng-declarative-purity, a11y-audit |

**DAG:** `E3-1 → E3-2 → { E3-3, E3-4 }`, with **E3-3 and E3-4 genuinely parallel**. Four children. E3-1 is the foundation (tokens + fonts) and blocks the component; E3-2 is the dumb hero. **E3-3 owns the integrated select→teaching wiring** — it binds the hero's `selectState` to `selected`/`annotation` and uses the hero's **built-in annotation callout (from E3-2)** as the baseline teaching surface, so it does **not** hard-depend on E3-4. **E3-4 owns a reusable richer per-state explainer + copy map**, proven against a **stub host** (not E3-3's container). Neither blocks the other's core — no cross-child DAG edge. Gate catalog (from `.claude/epic.yaml`): tdd-evidence, ng-declarative-purity, a11y-audit, xplat-build-smoke, rust-mutation-coverage — E3 is FE-only markup/TS/CSS, so **rust-mutation-coverage and xplat-build-smoke do not apply** to any child (no Rust, no CI-workflow edits).

Per-child recipes (files, contract, TDD order, verification commands, gate notes) are in the companion runbook.

## 7. Risks / open questions

- **The `-on` step is *reserved*, not shipped — the ramp is 4 valued steps.** E3-1 ships the four valued steps `bg`/`border`/`solid`/`text` per state (the values the design prompt supplies); `-on` (foreground for text placed *on* the `solid` fill) is a **named-but-unvalued reserved slot**, not required by any E3 component (the nodes/dots/glyphs use only `bg`/`border`/`solid`/`text` — §5). So there is **no** "five names, four values" gap: E3-1 does **not** author an `-on` value now. When a text-on-solid surface first needs it, derive it then (near-black `oklch(0.15 0 0)` on light solids, near-white on dark solids) and document the choice — low-risk and revisable.
- **SMIL `<animateMotion>` is the spottiest SVG-anim feature in WebViews.** It works in Tauri's Chromium/WebKit today, but the toggle-reinit path (gotcha 2) is fragile — a hidden-but-not-removed circle can freeze mid-path. Mitigated by `@if`-remove-and-re-add on toggle and by the documented WAAPI motion-path fallback (§3.1 gotcha 5). The e2e (E3-3) asserts the hero renders and a node is selectable; SMIL-frame correctness is verified visually / in the real webview, not in jsdom.
- **jsdom cannot render SVG motion or compute color-contrast.** `vitest-axe` guards structure/labels/keyboard, and the `animated=false` static path is unit-testable, but *that tokens actually flow* and *the ramp actually meets AA at runtime* are verified via the e2e / real-webview / manual-record pattern (as E2 did for contrast), documented in each PR body — not asserted in the unit suite.
- **No-visual-regression on the migrated dots/rows is a judgment call.** E3-1 changes the color source under `StateColorDirective`; "no regression" is asserted by keeping `--qb-state-<name>` aliased to `solid` and by the existing dot/row tests staying green, but a subtle hue shift (flat hex → OKLCH `solid`) is possible. The driver records before/after swatches in the PR body so the operator can sign off the shift as intentional.
- **The aggregate fold assumes `countsByState` keys are exactly `JobState`.** The fold reads `entry.countsByState[s] ?? 0` for each of the seven states; a backend emitting an unexpected key would be silently dropped from the total (never double-counted). Acceptable — the E2 adapter's `CASE` bucketing guarantees each job lands in exactly one `JobState` bucket — but noted so a future backend's key drift surfaces as an undercount, not a crash.
