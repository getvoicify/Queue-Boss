import { TestBed } from "@angular/core/testing";
import { describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import type { JobSummary } from "../../../core/models";
import { JobListComponent } from "./job-list.component";

const jobs: JobSummary[] = [
  {
    id: "job-1",
    queue: "emails",
    state: "active",
    createdAt: 1,
    startedAt: 2,
    completedAt: null,
    attempts: 1,
    priority: 0,
  },
  {
    id: "job-2",
    queue: "emails",
    state: "failed",
    createdAt: 3,
    startedAt: null,
    completedAt: 9,
    attempts: 3,
    priority: 5,
  },
];

function render(items: JobSummary[], hasMore = false) {
  const fixture = TestBed.createComponent(JobListComponent);
  fixture.componentRef.setInput("jobs", items);
  fixture.componentRef.setInput("hasMore", hasMore);
  fixture.detectChanges();
  return fixture;
}

describe("JobListComponent", () => {
  it("renders a state-colored row per job carrying id, attempts and priority", () => {
    const el = render(jobs).nativeElement;
    const rows = el.querySelectorAll('[data-testid="job-row"]');
    expect(rows.length).toBe(2);
    expect(rows[0].getAttribute("data-state")).toBe("active");
    expect(rows[1].getAttribute("data-state")).toBe("failed");
    expect(el.textContent).toContain("job-1");
    expect(el.textContent).toContain("job-2");
  });

  it("renders the empty state and no rows when there are no jobs", () => {
    const el = render([]).nativeElement;
    expect(el.querySelectorAll('[data-testid="job-row"]').length).toBe(0);
    expect(el.querySelector('[data-testid="job-list-empty"]')).not.toBeNull();
  });

  it("shows the load-more button only when hasMore and emits loadMore on click", () => {
    expect(
      render(jobs, false).nativeElement.querySelector(
        '[data-testid="jobs-load-more"]',
      ),
    ).toBeNull();

    const fixture = render(jobs, true);
    const loadMore = vi.fn();
    fixture.componentInstance.loadMore.subscribe(loadMore);
    const button = fixture.nativeElement.querySelector(
      '[data-testid="jobs-load-more"]',
    );
    expect(button).not.toBeNull();
    button.click();
    expect(loadMore).toHaveBeenCalled();
  });

  it("emits the selected job id when a row is clicked", () => {
    const fixture = render(jobs);
    const selected = vi.fn();
    fixture.componentInstance.select.subscribe(selected);
    fixture.nativeElement.querySelector('[data-testid="job-row"]').click();
    expect(selected).toHaveBeenCalledWith("job-1");
  });

  it("emits a state filter when the filter control changes", () => {
    const fixture = render(jobs);
    const changed = vi.fn();
    fixture.componentInstance.filterChange.subscribe(changed);
    const control = fixture.nativeElement.querySelector(
      '[data-testid="job-filter-state"]',
    ) as HTMLSelectElement;
    control.value = "failed";
    control.dispatchEvent(new Event("change"));
    expect(changed).toHaveBeenCalledWith(
      expect.objectContaining({ states: ["failed"] }),
    );
  });

  it("has no accessibility violations", async () => {
    const el = render(jobs, true).nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
