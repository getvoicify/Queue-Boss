import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  output,
} from "@angular/core";
import type { JobState } from "../../core/models";

const CANVAS_W = 760;
const CANVAS_H = 480;
const NODE_W = 150;
const NODE_H = 88;

const ZERO_COUNTS: Record<JobState, number> = {
  created: 0,
  active: 0,
  completed: 0,
  failed: 0,
  cancelled: 0,
  retry: 0,
  deadLetter: 0,
};

const LABELS: Record<JobState, string> = {
  created: "Created",
  active: "Active",
  completed: "Completed",
  failed: "Failed",
  cancelled: "Cancelled",
  retry: "Retry",
  deadLetter: "Dead letter",
};

const GLYPHS: Record<JobState, string> = {
  created: "◷",
  active: "▶",
  completed: "✓",
  failed: "✕",
  retry: "↻",
  cancelled: "⊘",
  deadLetter: "☠",
};

interface Edge {
  readonly d: string;
  readonly color: JobState;
  readonly flow: boolean;
  readonly dashed: boolean;
}

// Geometry copied verbatim from runbook §E3-2 / spec §3.1. Each `d` is
// authored ONCE here and shared with the flowing token so its animateMotion
// path stays byte-identical to the edge (gotcha 1).
const EDGES: readonly Edge[] = [
  {
    d: "M 156 220 C 202 220, 202 154, 248 154",
    color: "created",
    flow: true,
    dashed: false,
  },
  {
    d: "M 398 154 C 479 154, 479 62, 560 62",
    color: "completed",
    flow: true,
    dashed: false,
  },
  {
    d: "M 398 154 C 479 154, 479 220, 560 220",
    color: "failed",
    flow: true,
    dashed: false,
  },
  {
    d: "M 398 172 C 470 220, 470 360, 560 378",
    color: "cancelled",
    flow: false,
    dashed: false,
  },
  {
    d: "M 635 264 C 635 420, 428 344, 398 344",
    color: "retry",
    flow: false,
    dashed: false,
  },
  {
    d: "M 248 344 C 180 344, 180 154, 323 198",
    color: "active",
    flow: true,
    dashed: true,
  },
  {
    d: "M 323 388 C 349 388, 349 402, 375 402",
    color: "deadLetter",
    flow: false,
    dashed: false,
  },
];

// Filtered flowing edges 1/2/3/6; dur staggers over THIS 0-based index →
// 1.6 / 2.0 / 2.4 / 1.6 (not the raw edge number).
const FLOWING_TOKENS = EDGES.filter((e) => e.flow).map((e, i) => ({
  d: e.d,
  colorVar: `var(--qb-state-${e.color}-solid)`,
  dur: `${(1.6 + (i % 3) * 0.4).toFixed(1)}s`,
}));

const NODE_COORDS: ReadonlyArray<{ state: JobState; x: number; y: number }> = [
  { state: "created", x: 6, y: 176 },
  { state: "active", x: 248, y: 110 },
  { state: "completed", x: 560, y: 18 },
  { state: "failed", x: 560, y: 176 },
  { state: "cancelled", x: 560, y: 334 },
  { state: "retry", x: 248, y: 300 },
  { state: "deadLetter", x: 300, y: 402 },
];

const NODES = NODE_COORDS.map((n) => ({
  state: n.state,
  left: (n.x / CANVAS_W) * 100,
  top: (n.y / CANVAS_H) * 100,
  bg: `var(--qb-state-${n.state}-bg)`,
  border: `var(--qb-state-${n.state}-border)`,
  solid: `var(--qb-state-${n.state}-solid)`,
  text: `var(--qb-state-${n.state}-text)`,
}));

const WIDTH_PCT = (NODE_W / CANVAS_W) * 100;
const HEIGHT_PCT = (NODE_H / CANVAS_H) * 100;

// Per-instance marker id: two diagrams on one page must not collide on a
// document-global `qb-arrow` id (invalid DOM, both edges resolve to the first).
let markerSeq = 0;

@Component({
  selector: "app-lifecycle-diagram",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="lifecycle-diagram"
      role="group"
      aria-label="Job lifecycle diagram"
    >
      <div class="lifecycle-diagram__canvas">
        <svg
          class="lifecycle-diagram__svg"
          viewBox="0 0 760 480"
          width="100%"
          aria-hidden="true"
          focusable="false"
        >
          <defs>
            <marker
              [attr.id]="markerId"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="7"
              markerHeight="7"
              orient="auto-start-reverse"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--qb-border-strong)" />
            </marker>
          </defs>

          @for (edge of edges; track edge.d) {
            <path
              class="lifecycle-diagram__edge"
              [attr.d]="edge.d"
              fill="none"
              stroke="var(--qb-border-strong)"
              stroke-width="1.5"
              opacity="0.85"
              [attr.marker-end]="'url(#' + markerId + ')'"
              [attr.stroke-dasharray]="edge.dashed ? '5 5' : null"
            />
          }

          @for (token of flowingTokens(); track token.d) {
            <circle r="4" [style.fill]="token.colorVar">
              <animateMotion
                [attr.path]="token.d"
                [attr.dur]="token.dur"
                repeatCount="indefinite"
                rotate="auto"
              />
            </circle>
          }
        </svg>

        @for (n of nodes; track n.state) {
          <div
            class="lifecycle-diagram__node"
            [class.lifecycle-diagram__node--selected]="selected() === n.state"
            [class.lifecycle-diagram__node--pulsing]="
              n.state === 'active' && motionOn()
            "
            [attr.data-testid]="'lifecycle-node-' + n.state"
            [attr.data-state]="n.state"
            role="button"
            tabindex="0"
            [attr.aria-label]="ariaLabel(n.state)"
            [attr.aria-pressed]="selected() === n.state"
            [style.left.%]="n.left"
            [style.top.%]="n.top"
            [style.width.%]="widthPct"
            [style.height.%]="heightPct"
            [style.--qb-node-bg]="n.bg"
            [style.--qb-node-border]="n.border"
            [style.--qb-node-solid]="n.solid"
            [style.--qb-node-text]="n.text"
            (click)="selectState.emit(n.state)"
            (keydown.enter)="activate($event, n.state)"
            (keydown.space)="activate($event, n.state)"
          >
            <span class="lifecycle-diagram__header">
              <span class="lifecycle-diagram__dot" aria-hidden="true"></span>
              <span class="lifecycle-diagram__label">{{ labels[n.state] }}</span>
              <span class="lifecycle-diagram__glyph" aria-hidden="true">{{
                glyphs[n.state]
              }}</span>
            </span>
            <span class="lifecycle-diagram__count">{{
              formatCount(counts()[n.state])
            }}</span>
            <span class="lifecycle-diagram__caption">{{
              caption(counts()[n.state])
            }}</span>
          </div>
        }
      </div>

      @if (annotation(); as note) {
        <div
          class="lifecycle-diagram__annotation"
          data-testid="lifecycle-annotation"
          role="note"
          [attr.data-state]="note.state"
          [style.--qb-note-bg]="ramp(note.state, 'bg')"
          [style.--qb-note-border]="ramp(note.state, 'border')"
          [style.--qb-note-text]="ramp(note.state, 'text')"
        >
          <span class="lifecycle-diagram__glyph" aria-hidden="true">ⓘ</span>
          <span>{{ note.text }}</span>
        </div>
      }
    </div>
  `,
  styles: `
    .lifecycle-diagram {
      background: var(--qb-surface);
      border: 1px solid var(--qb-border);
      border-radius: 7px;
      padding: 8px;
      font-family: var(--qb-font-sans);
    }
    .lifecycle-diagram__canvas {
      position: relative;
      width: 100%;
      aspect-ratio: 760 / 480;
    }
    .lifecycle-diagram__svg {
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      overflow: visible;
    }
    .lifecycle-diagram__node {
      position: absolute;
      box-sizing: border-box;
      display: flex;
      flex-direction: column;
      gap: 2px;
      padding: 6px 8px;
      border-radius: 6px;
      background: var(--qb-node-bg);
      border: 1.5px solid var(--qb-node-border);
      color: var(--qb-node-text);
      cursor: pointer;
      text-align: left;
    }
    .lifecycle-diagram__node:focus-visible {
      outline: 2px solid var(--qb-node-solid);
      outline-offset: 2px;
    }
    .lifecycle-diagram__node--selected {
      border-color: var(--qb-node-solid);
      box-shadow: 0 0 0 2px var(--qb-node-solid);
    }
    .lifecycle-diagram__header {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 12px;
    }
    .lifecycle-diagram__dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--qb-node-solid);
      flex: none;
    }
    .lifecycle-diagram__label {
      font-weight: 600;
    }
    .lifecycle-diagram__glyph {
      margin-left: auto;
      color: var(--qb-node-solid);
    }
    .lifecycle-diagram__count {
      font-family: var(--qb-font-mono);
      font-size: 22px;
      line-height: 1.1;
    }
    .lifecycle-diagram__caption {
      font-size: 11px;
      color: var(--qb-fg-muted);
    }
    .lifecycle-diagram__node--pulsing .lifecycle-diagram__dot {
      color: var(--qb-node-solid);
      animation: qb-node-pulse 1.6s ease-out infinite;
    }
    @keyframes qb-node-pulse {
      0% {
        box-shadow: 0 0 0 0 currentColor;
      }
      70% {
        box-shadow: 0 0 0 7px transparent;
      }
      100% {
        box-shadow: 0 0 0 0 transparent;
      }
    }
    .lifecycle-diagram__annotation {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-top: 8px;
      padding: 8px 10px;
      border-radius: 6px;
      background: var(--qb-note-bg);
      border: 1px solid var(--qb-note-border);
      color: var(--qb-note-text);
      font-size: 13px;
    }
    @media (prefers-reduced-motion: reduce) {
      .lifecycle-diagram__node--pulsing .lifecycle-diagram__dot {
        animation: none;
      }
    }
  `,
})
export class LifecycleDiagramComponent {
  readonly counts = input<Record<JobState, number>>(ZERO_COUNTS);
  readonly animated = input<boolean>(true);
  readonly selected = input<JobState | null>(null);
  readonly annotation = input<{ state: JobState; text: string } | null>(null);
  readonly selectState = output<JobState>();

  protected readonly markerId = `qb-arrow-${++markerSeq}`;
  protected readonly nodes = NODES;
  protected readonly edges = EDGES;
  protected readonly labels = LABELS;
  protected readonly glyphs = GLYPHS;
  protected readonly widthPct = WIDTH_PCT;
  protected readonly heightPct = HEIGHT_PCT;

  // Read once, defensively — jsdom omits matchMedia (stubbed in setup-axe.ts).
  // This presentation media query is purity-exempt (§E3-2).
  private readonly reducedMotion =
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
      : false;

  protected readonly motionOn = computed(
    () => this.animated() && !this.reducedMotion,
  );
  protected readonly flowingTokens = computed(() =>
    this.motionOn() ? FLOWING_TOKENS : [],
  );

  protected formatCount(value: number): string {
    return value.toLocaleString();
  }

  protected caption(value: number): string {
    return value === 1 ? "job" : "jobs";
  }

  protected ariaLabel(state: JobState): string {
    const value = this.counts()[state];
    return `${LABELS[state]}: ${value.toLocaleString()} ${this.caption(value)}`;
  }

  protected ramp(state: JobState, step: "bg" | "border" | "text"): string {
    return `var(--qb-state-${state}-${step})`;
  }

  protected activate(event: Event, state: JobState): void {
    event.preventDefault();
    this.selectState.emit(state);
  }
}
