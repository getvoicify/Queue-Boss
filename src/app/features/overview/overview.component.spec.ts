import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { QueueCountEntry } from "../../core/models";
import { OverviewComponent } from "./overview.component";

const sample: QueueCountEntry[] = [
  {
    queue: "emails",
    totalDepth: 5,
    countsByState: { active: 2, completed: 3 },
    oldestWaitingAge: 180,
  },
  {
    queue: "images",
    totalDepth: 0,
    countsByState: {},
    oldestWaitingAge: null,
  },
];

function render(queues: QueueCountEntry[]) {
  const fixture = TestBed.createComponent(OverviewComponent);
  fixture.componentRef.setInput("queues", queues);
  fixture.detectChanges();
  return fixture;
}

describe("OverviewComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [OverviewComponent],
    }).compileComponents();
  });

  it("renders one row per queue", () => {
    const el = render(sample).nativeElement;
    expect(el.querySelectorAll('[data-testid="queue-row"]').length).toBe(2);
  });

  it("renders the empty state and no data rows when there are no queues", () => {
    const el = render([]).nativeElement;
    expect(el.querySelectorAll('[data-testid="queue-row"]').length).toBe(0);
    const empty = el.querySelector('[data-testid="overview-empty"]');
    expect(empty).not.toBeNull();
    expect(empty.textContent.trim()).toBe("No queues to display.");
  });

  it("renders depth, per-state counts (0 when sparse) and oldest-waiting age", () => {
    const el = render(sample).nativeElement;
    expect(
      el.querySelector('[data-testid="depth-emails"]').textContent.trim(),
    ).toBe("5");
    expect(
      el
        .querySelector('[data-testid="count-emails-active"]')
        .textContent.trim(),
    ).toBe("2");
    expect(
      el
        .querySelector('[data-testid="count-emails-completed"]')
        .textContent.trim(),
    ).toBe("3");
    expect(
      el
        .querySelector('[data-testid="count-emails-created"]')
        .textContent.trim(),
    ).toBe("0");
    expect(
      el.querySelector('[data-testid="oldest-emails"]').textContent.trim(),
    ).toBe("3m");
    expect(
      el.querySelector('[data-testid="oldest-images"]').textContent.trim(),
    ).toBe("—");
  });

  it("has no accessibility violations", async () => {
    const el = render(sample).nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
