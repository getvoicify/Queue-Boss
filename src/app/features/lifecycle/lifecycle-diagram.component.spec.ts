import { TestBed } from "@angular/core/testing";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { JobState } from "../../core/models";
import { LifecycleDiagramComponent } from "./lifecycle-diagram.component";

const STATES: JobState[] = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
];

const ZEROS: Record<JobState, number> = {
  created: 0,
  active: 0,
  completed: 0,
  failed: 0,
  cancelled: 0,
  retry: 0,
  deadLetter: 0,
};

// Regression lock — the seven edge `d` strings, byte-exact, in edge order 1..7.
const EDGE_DS = [
  "M 156 220 C 202 220, 202 154, 248 154",
  "M 398 154 C 479 154, 479 62, 560 62",
  "M 398 154 C 479 154, 479 220, 560 220",
  "M 398 172 C 470 220, 470 360, 560 378",
  "M 635 264 C 635 420, 428 344, 398 344",
  "M 248 344 C 180 344, 180 154, 323 198",
  "M 323 388 C 349 388, 349 402, 375 402",
];

// Flowing edges 1/2/3/6 with staggered durations 1.6/2.0/2.4/1.6.
const FLOW = [
  { d: EDGE_DS[0], dur: "1.6s" },
  { d: EDGE_DS[1], dur: "2.0s" },
  { d: EDGE_DS[2], dur: "2.4s" },
  { d: EDGE_DS[5], dur: "1.6s" },
];

const GLYPHS: Record<JobState, string> = {
  created: "◷",
  active: "▶",
  completed: "✓",
  failed: "✕",
  retry: "↻",
  cancelled: "⊘",
  deadLetter: "☠",
};

interface Inputs {
  counts?: Record<JobState, number>;
  animated?: boolean;
  selected?: JobState | null;
  annotation?: { state: JobState; text: string } | null;
}

function render(inputs: Inputs = {}) {
  const fixture = TestBed.createComponent(LifecycleDiagramComponent);
  const ref = fixture.componentRef;
  ref.setInput("counts", inputs.counts ?? ZEROS);
  if (inputs.animated !== undefined) ref.setInput("animated", inputs.animated);
  if (inputs.selected !== undefined) ref.setInput("selected", inputs.selected);
  if (inputs.annotation !== undefined)
    ref.setInput("annotation", inputs.annotation);
  fixture.detectChanges();
  return fixture;
}

function node(el: HTMLElement, state: JobState): HTMLElement {
  return el.querySelector(
    `[data-testid="lifecycle-node-${state}"]`,
  ) as HTMLElement;
}

describe("LifecycleDiagramComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LifecycleDiagramComponent],
    }).compileComponents();
  });

  it("renders one node per JobState with label, glyph and locale-formatted count", () => {
    const counts = { ...ZEROS, created: 1234, active: 1 };
    const el = render({ counts }).nativeElement as HTMLElement;

    expect(el.querySelectorAll('[data-testid^="lifecycle-node-"]').length).toBe(
      7,
    );
    for (const state of STATES) {
      const n = node(el, state);
      expect(n, `node ${state} exists`).not.toBeNull();
      expect(n.textContent).toContain(GLYPHS[state]);
    }
    expect(node(el, "created").textContent).toContain((1234).toLocaleString());
    expect(node(el, "created").textContent?.toLowerCase()).toContain("created");
  });

  it("renders seven edges with the exact d strings and one arrowhead marker", () => {
    const el = render().nativeElement as HTMLElement;
    const edges = Array.from(el.querySelectorAll(".lifecycle-diagram__edge"));
    expect(edges.length).toBe(7);
    expect(edges.map((e) => e.getAttribute("d"))).toEqual(EDGE_DS);
    expect(el.querySelectorAll("marker").length).toBe(1);
    const markerId = el.querySelector("marker")?.getAttribute("id");
    expect(markerId).toBeTruthy();
    expect(el.querySelector(`#${markerId}`)).not.toBeNull();
    expect(el.querySelector('[stroke-dasharray="5 5"]')).not.toBeNull();
  });

  it("gives each instance a unique marker id and wires its edges to it", () => {
    const a = render().nativeElement as HTMLElement;
    const b = render().nativeElement as HTMLElement;

    const idA = a.querySelector("marker")?.getAttribute("id");
    const idB = b.querySelector("marker")?.getAttribute("id");
    expect(idA).toBeTruthy();
    expect(idB).toBeTruthy();
    expect(idA).not.toBe(idB);

    for (const el of [a, b]) {
      const id = el.querySelector("marker")?.getAttribute("id");
      const edges = Array.from(el.querySelectorAll(".lifecycle-diagram__edge"));
      expect(edges.length).toBe(7);
      for (const edge of edges) {
        expect(edge.getAttribute("marker-end")).toBe(`url(#${id})`);
      }
    }
  });

  it("renders four flowing tokens whose animateMotion path === edge d, staggered", () => {
    const el = render({ animated: true }).nativeElement as HTMLElement;
    const circles = Array.from(el.querySelectorAll("circle"));
    expect(circles.length).toBe(4);
    circles.forEach((c, i) => {
      const am = c.querySelector("animateMotion");
      expect(am, `circle ${i} has animateMotion`).not.toBeNull();
      expect(am?.getAttribute("path")).toBe(FLOW[i].d);
      expect(am?.getAttribute("dur")).toBe(FLOW[i].dur);
    });
  });

  it("pulses the active node only, never the others, when animated", () => {
    const el = render({ animated: true, counts: { ...ZEROS, active: 1 } })
      .nativeElement as HTMLElement;
    expect(
      node(el, "active").classList.contains("lifecycle-diagram__node--pulsing"),
    ).toBe(true);
    for (const state of STATES.filter((s) => s !== "active")) {
      expect(
        node(el, state).classList.contains("lifecycle-diagram__node--pulsing"),
        `node ${state} must not pulse`,
      ).toBe(false);
    }
  });

  it("freezes when animated=false: no flowing tokens, no pulse", () => {
    const el = render({ animated: false }).nativeElement as HTMLElement;
    expect(el.querySelectorAll("circle").length).toBe(0);
    expect(
      node(el, "active").classList.contains("lifecycle-diagram__node--pulsing"),
    ).toBe(false);
  });

  it("freezes under prefers-reduced-motion even when animated=true", () => {
    const original = window.matchMedia;
    window.matchMedia = ((query: string) => ({
      matches: true,
      media: query,
      onchange: null,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => false,
    })) as typeof window.matchMedia;
    try {
      const el = render({ animated: true }).nativeElement as HTMLElement;
      expect(el.querySelectorAll("circle").length).toBe(0);
      expect(
        node(el, "active").classList.contains(
          "lifecycle-diagram__node--pulsing",
        ),
      ).toBe(false);
    } finally {
      window.matchMedia = original;
    }
  });

  it("emits selectState on node click and on Enter/Space activation", () => {
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;
    const emitted: JobState[] = [];
    fixture.componentInstance.selectState.subscribe((s) => emitted.push(s));

    node(el, "active").click();
    node(el, "failed").dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    node(el, "retry").dispatchEvent(
      new KeyboardEvent("keydown", { key: " ", bubbles: true }),
    );

    expect(emitted).toEqual(["active", "failed", "retry"]);
  });

  it("draws a selection ring on the selected node only", () => {
    const el = render({ selected: "failed" }).nativeElement as HTMLElement;
    expect(
      node(el, "failed").classList.contains(
        "lifecycle-diagram__node--selected",
      ),
    ).toBe(true);
    expect(
      node(el, "active").classList.contains(
        "lifecycle-diagram__node--selected",
      ),
    ).toBe(false);
  });

  it("renders no annotation callout when annotation is null", () => {
    const el = render({ annotation: null }).nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="lifecycle-annotation"]')).toBeNull();
  });

  it("renders the annotation callout recolored to its state", () => {
    const el = render({
      annotation: { state: "retry", text: "waiting on a backoff timer" },
    }).nativeElement as HTMLElement;
    const callout = el.querySelector(
      '[data-testid="lifecycle-annotation"]',
    ) as HTMLElement;
    expect(callout).not.toBeNull();
    expect(callout.textContent).toContain("waiting on a backoff timer");
    expect(callout.getAttribute("data-state")).toBe("retry");
    expect(callout.style.getPropertyValue("--qb-note-bg").trim()).toBe(
      "var(--qb-state-retry-bg)",
    );
    expect(callout.style.getPropertyValue("--qb-note-border").trim()).toBe(
      "var(--qb-state-retry-border)",
    );
    expect(callout.style.getPropertyValue("--qb-note-text").trim()).toBe(
      "var(--qb-state-retry-text)",
    );
  });

  it("has no accessibility violations", async () => {
    const el = render({ counts: { ...ZEROS, active: 3 } })
      .nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});

afterEach(() => {
  TestBed.resetTestingModule();
});
