import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { By } from "@angular/platform-browser";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { ConnectionsFacade } from "../../core/facades/connections.facade";
import { QueuesFacade } from "../../core/facades/queues.facade";
import type { JobState, QueueCountEntry } from "../../core/models";
import { LifecycleDiagramComponent } from "./lifecycle-diagram.component";
import {
  LifecycleHomeContainerComponent,
  LOCAL_COPY,
} from "./lifecycle-home-container.component";

const STATES: JobState[] = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
];

const sparseQueues: QueueCountEntry[] = [
  {
    queue: "emails",
    totalDepth: 6,
    countsByState: { created: 2, active: 1, completed: 3 },
    oldestWaitingAge: 5,
  },
  {
    queue: "webhooks",
    totalDepth: 7,
    countsByState: { active: 2, failed: 1, deadLetter: 4 },
    oldestWaitingAge: 2,
  },
];

describe("LifecycleHomeContainerComponent", () => {
  const queues = signal<QueueCountEntry[]>([]);
  const connect = vi.fn();
  const active = signal("sandbox");

  beforeEach(async () => {
    queues.set([]);
    active.set("sandbox");
    connect.mockClear();
    await TestBed.configureTestingModule({
      imports: [LifecycleHomeContainerComponent],
      providers: [
        {
          provide: QueuesFacade,
          useValue: { queues: queues.asReadonly(), connect },
        },
        {
          provide: ConnectionsFacade,
          useValue: { activeConnectionId: active.asReadonly() },
        },
      ],
    }).compileComponents();
  });

  function render() {
    const fixture = TestBed.createComponent(LifecycleHomeContainerComponent);
    fixture.detectChanges();
    return fixture;
  }

  function diagramOf(fixture: ReturnType<typeof render>) {
    return fixture.debugElement.query(By.directive(LifecycleDiagramComponent))
      .componentInstance as LifecycleDiagramComponent;
  }

  function node(el: HTMLElement, state: JobState): HTMLElement {
    return el.querySelector(
      `[data-testid="lifecycle-node-${state}"]`,
    ) as HTMLElement;
  }

  // The callout's own text span (the glyph span is excluded) so equality can be
  // asserted against the copy SOURCE, not the component's own annotation output.
  function annotationText(el: HTMLElement): string | undefined {
    return el
      .querySelector(
        '[data-testid="lifecycle-annotation"] span:not(.lifecycle-diagram__glyph)',
      )
      ?.textContent?.trim();
  }

  it("folds every queue's sparse countsByState into a dense 7-key aggregate", () => {
    queues.set(sparseQueues);
    const fixture = render();

    expect(diagramOf(fixture).counts()).toEqual({
      created: 2,
      active: 3,
      completed: 3,
      failed: 1,
      cancelled: 0,
      retry: 0,
      deadLetter: 4,
    });
  });

  it("connects the queues stream to the active connection on entry (no gate)", () => {
    render();
    expect(connect).toHaveBeenCalledWith("sandbox");
  });

  it("rekeys the stream when the active connection changes", () => {
    const fixture = render();
    active.set("pgboss");
    fixture.detectChanges();
    expect(connect).toHaveBeenCalledWith("pgboss");
  });

  it("shows a waiting affordance and no hero until counts arrive", () => {
    const fixture = render();
    let el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="home-hero"]')).toBeNull();
    expect(el.querySelector('[data-testid="home-waiting"]')).not.toBeNull();

    queues.set(sparseQueues);
    fixture.detectChanges();
    el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="home-hero"]')).not.toBeNull();
    expect(el.querySelector('[data-testid="home-waiting"]')).toBeNull();
    expect(el.querySelectorAll('[data-testid^="lifecycle-node-"]').length).toBe(
      7,
    );
  });

  it("surfaces no teaching annotation until a node is selected", () => {
    queues.set(sparseQueues);
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;

    expect(diagramOf(fixture).selected()).toBeNull();
    expect(diagramOf(fixture).annotation()).toBeNull();
    expect(el.querySelector('[data-testid="lifecycle-annotation"]')).toBeNull();
  });

  it("selects a clicked node and surfaces its local teaching copy in the hero callout", () => {
    queues.set(sparseQueues);
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;

    node(el, "active").click();
    fixture.detectChanges();

    const activeNote = diagramOf(fixture).annotation();
    expect(diagramOf(fixture).selected()).toBe("active");
    expect(activeNote?.state).toBe("active");
    expect(activeNote?.text.length ?? 0).toBeGreaterThan(0);
    const callout = el.querySelector('[data-testid="lifecycle-annotation"]');
    expect(callout?.getAttribute("data-state")).toBe("active");
    expect(annotationText(el)).toBe(LOCAL_COPY.active);

    node(el, "failed").click();
    fixture.detectChanges();

    const failedNote = diagramOf(fixture).annotation();
    expect(diagramOf(fixture).selected()).toBe("failed");
    expect(failedNote?.state).toBe("failed");
    expect(failedNote?.text).not.toBe(activeNote?.text);
    expect(
      el
        .querySelector('[data-testid="lifecycle-annotation"]')
        ?.getAttribute("data-state"),
    ).toBe("failed");
    expect(annotationText(el)).toBe(LOCAL_COPY.failed);
  });

  it("gives distinct teaching copy to every job state", () => {
    queues.set(sparseQueues);
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;

    const texts = new Set<string>();
    for (const state of STATES) {
      node(el, state).click();
      fixture.detectChanges();
      const note = diagramOf(fixture).annotation();
      expect(note?.state).toBe(state);
      expect(note?.text.length ?? 0).toBeGreaterThan(0);
      texts.add(note?.text ?? "");
    }
    expect(texts.size).toBe(STATES.length);
  });

  it("has no accessibility violations while waiting for counts", async () => {
    const fixture = render();
    expect(await axe(fixture.nativeElement)).toHaveNoViolations();
  });

  it("has no accessibility violations once the hero streams with a selection", async () => {
    queues.set([
      {
        queue: "emails",
        totalDepth: 6,
        countsByState: { active: 3, completed: 2 },
        oldestWaitingAge: 5,
      },
    ]);
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;
    node(el, "active").click();
    fixture.detectChanges();
    expect(await axe(fixture.nativeElement)).toHaveNoViolations();
  });
});
