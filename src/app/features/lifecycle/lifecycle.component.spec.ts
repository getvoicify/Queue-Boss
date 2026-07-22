import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { JobState } from "../../core/models";
import { LifecycleComponent } from "./lifecycle.component";

const counts: Partial<Record<JobState, number>> = { active: 2, failed: 1 };

function render(input: Partial<Record<JobState, number>>) {
  const fixture = TestBed.createComponent(LifecycleComponent);
  fixture.componentRef.setInput("counts", input);
  fixture.detectChanges();
  return fixture;
}

describe("LifecycleComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LifecycleComponent],
    }).compileComponents();
  });

  it("renders one node per JobState", () => {
    const el = render(counts).nativeElement;
    expect(el.querySelectorAll('[data-testid="lifecycle-item"]').length).toBe(
      7,
    );
  });

  it("shows each state's count, defaulting absent states to 0", () => {
    const el = render(counts).nativeElement;
    expect(
      el
        .querySelector('[data-testid="lifecycle-value-active"]')
        .textContent.trim(),
    ).toBe("2");
    expect(
      el
        .querySelector('[data-testid="lifecycle-value-failed"]')
        .textContent.trim(),
    ).toBe("1");
    expect(
      el
        .querySelector('[data-testid="lifecycle-value-created"]')
        .textContent.trim(),
    ).toBe("0");
    expect(
      el
        .querySelector('[data-testid="lifecycle-value-deadLetter"]')
        .textContent.trim(),
    ).toBe("0");
  });

  it("has no accessibility violations", async () => {
    const el = render(counts).nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
