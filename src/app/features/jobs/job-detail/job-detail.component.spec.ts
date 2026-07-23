import { formatDate } from "@angular/common";
import { TestBed } from "@angular/core/testing";
import { describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { Capabilities, JobDetail } from "../../../core/models";
import { JobDetailComponent } from "./job-detail.component";

function detail(overrides: Partial<JobDetail> = {}): JobDetail {
  return {
    id: "job-1",
    queue: "emails",
    state: "retry",
    createdAt: 1,
    startedAt: 2,
    completedAt: null,
    attempts: 2,
    priority: 0,
    data: { to: "a@b.c" },
    output: null,
    timeline: [
      { at: 1, state: "created" },
      { at: 2, state: "active" },
    ],
    retry: { attempts: 2, maxAttempts: 5, nextRetryAt: 1_700_000_000_000 },
    extensions: {},
    ...overrides,
  };
}

function caps(extensions: string[] = []): Capabilities {
  return { priority: true, singleton: false, deadLetter: true, extensions };
}

function render(job: JobDetail, capabilities: Capabilities) {
  const fixture = TestBed.createComponent(JobDetailComponent);
  fixture.componentRef.setInput("job", job);
  fixture.componentRef.setInput("capabilities", capabilities);
  fixture.detectChanges();
  return fixture;
}

describe("JobDetailComponent", () => {
  it("renders the detail panel with a compact data/output preview and the 'N of M' retry readout, no 'backoff'", () => {
    const el = render(detail(), caps()).nativeElement;
    expect(el.querySelector('[data-testid="job-detail"]')).not.toBeNull();
    expect(el.querySelector('[data-testid="job-data"]').textContent).toContain(
      '"to":"a@b.c"',
    );
    expect(
      el.querySelector('[data-testid="job-output"]').textContent.trim(),
    ).toBe("—");
    expect(el.querySelector('[data-testid="job-retry"]').textContent).toContain(
      "2 of 5",
    );
    expect(el.querySelector('[data-testid="job-next-retry"]')).not.toBeNull();
    expect(el.textContent.toLowerCase()).not.toContain("backoff");
  });

  it("renders the timeline events", () => {
    const el = render(detail(), caps()).nativeElement;
    expect(el.querySelectorAll('[data-testid="timeline-event"]').length).toBe(
      2,
    );
  });

  it("renders next-retry and timeline timestamps as human-readable dates, not raw epoch ms", () => {
    const nextRetryAt = 1_700_000_000_000;
    const at = 1_699_999_000_000;
    const job = detail({
      retry: { attempts: 2, maxAttempts: 5, nextRetryAt },
      timeline: [{ at, state: "created" }],
    });
    const el = render(job, caps()).nativeElement;

    const nextRetry = el.querySelector('[data-testid="job-next-retry"]');
    expect(nextRetry.textContent.trim()).toBe(
      formatDate(nextRetryAt, "medium", "en-US"),
    );
    expect(nextRetry.textContent).not.toContain(String(nextRetryAt));

    const timelineTime = el.querySelector(
      '[data-testid="timeline-event"] time',
    );
    expect(timelineTime.textContent.trim()).toBe(
      formatDate(at, "medium", "en-US"),
    );
    expect(timelineTime.textContent).not.toContain(String(at));
  });

  it("renders an extension row only for keys in BOTH the extensions map and the advertised capabilities", () => {
    const job = detail({
      extensions: {
        policy: { singletonKey: "x" },
        deadLetter: { reason: "y" },
      },
    });
    const el = render(
      job,
      caps(["singletonKey", "policy", "priority"]),
    ).nativeElement;
    expect(
      el.querySelector('[data-testid="job-extension-policy"]'),
    ).not.toBeNull();
    expect(
      el.querySelector('[data-testid="job-extension-deadLetter"]'),
    ).toBeNull();
  });

  it("renders no extension rows when the backend advertises none (sandbox)", () => {
    const job = detail({ extensions: { deadLetter: { reason: "y" } } });
    const el = render(job, caps([])).nativeElement;
    expect(el.querySelectorAll('[data-testid^="job-extension-"]').length).toBe(
      0,
    );
  });

  it("has no accessibility violations", async () => {
    const job = detail({ extensions: { policy: { singletonKey: "x" } } });
    const el = render(job, caps(["policy"])).nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
