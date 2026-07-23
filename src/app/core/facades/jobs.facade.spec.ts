import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { describe, expect, it, vi } from "vitest";
import type { Capabilities, JobDetail, JobSummary, Page } from "../models";
import { QueueBackendService } from "../tauri/queue-backend.service";
import { ConnectionsFacade } from "./connections.facade";
import { JobsFacade } from "./jobs.facade";

function summary(id: string): JobSummary {
  return {
    id,
    queue: "default",
    state: "active",
    createdAt: 1,
    startedAt: null,
    completedAt: null,
    attempts: 1,
    priority: 0,
  };
}

function detail(id: string): JobDetail {
  return {
    ...summary(id),
    data: null,
    output: null,
    timeline: [],
    retry: { attempts: 1, maxAttempts: 3, nextRetryAt: null },
    extensions: {},
  };
}

const caps: Capabilities = {
  priority: true,
  singleton: false,
  deadLetter: true,
  extensions: [],
};

function configure(
  backend: Partial<QueueBackendService>,
  activeId = "sandbox",
): JobsFacade {
  TestBed.configureTestingModule({
    providers: [
      { provide: QueueBackendService, useValue: backend },
      {
        provide: ConnectionsFacade,
        useValue: { activeConnectionId: signal(activeId).asReadonly() },
      },
    ],
  });
  return TestBed.inject(JobsFacade);
}

describe("JobsFacade", () => {
  it("loadPage fetches a page keyed on the active connection, appends items and tracks hasMore/cursor", async () => {
    const page1: Page<JobSummary> = {
      items: [summary("a"), summary("b")],
      nextCursor: "c1",
      hasMore: true,
    };
    const page2: Page<JobSummary> = {
      items: [summary("c")],
      nextCursor: null,
      hasMore: false,
    };
    const listJobs = vi
      .fn()
      .mockResolvedValueOnce(page1)
      .mockResolvedValueOnce(page2);
    const facade = configure({ listJobs });

    await facade.loadPage();
    expect(listJobs).toHaveBeenLastCalledWith(
      "sandbox",
      expect.objectContaining({ cursor: undefined }),
    );
    expect(facade.jobs().map((j) => j.id)).toEqual(["a", "b"]);
    expect(facade.hasMore()).toBe(true);
    expect(facade.nextCursor()).toBe("c1");

    await facade.loadPage();
    expect(listJobs).toHaveBeenLastCalledWith(
      "sandbox",
      expect.objectContaining({ cursor: "c1" }),
    );
    expect(facade.jobs().map((j) => j.id)).toEqual(["a", "b", "c"]);
    expect(facade.hasMore()).toBe(false);
    expect(facade.nextCursor()).toBeNull();
  });

  it("setFilter replaces the filter, resets accumulated jobs and cursor, then reloads the first page", async () => {
    const page1: Page<JobSummary> = {
      items: [summary("a")],
      nextCursor: "c1",
      hasMore: true,
    };
    const filtered: Page<JobSummary> = {
      items: [summary("z")],
      nextCursor: null,
      hasMore: false,
    };
    const listJobs = vi
      .fn()
      .mockResolvedValueOnce(page1)
      .mockResolvedValueOnce(filtered);
    const facade = configure({ listJobs });

    await facade.loadPage();
    expect(facade.jobs().map((j) => j.id)).toEqual(["a"]);

    await facade.setFilter({ states: ["failed"], limit: 20 });
    expect(facade.filter()).toEqual({ states: ["failed"], limit: 20 });
    expect(listJobs).toHaveBeenLastCalledWith(
      "sandbox",
      expect.objectContaining({ states: ["failed"], cursor: undefined }),
    );
    expect(facade.jobs().map((j) => j.id)).toEqual(["z"]);
    expect(facade.hasMore()).toBe(false);
    expect(facade.nextCursor()).toBeNull();
  });

  it("select loads the job detail and ensures capabilities are loaded once for the active connection", async () => {
    const getJob = vi.fn().mockResolvedValue(detail("job-1"));
    const capabilities = vi.fn().mockResolvedValue(caps);
    const facade = configure({ getJob, capabilities });

    await facade.select("job-1");
    expect(getJob).toHaveBeenCalledWith("sandbox", "job-1");
    expect(facade.selected()?.id).toBe("job-1");
    expect(capabilities).toHaveBeenCalledWith("sandbox");
    expect(facade.capabilities()).toEqual(caps);

    await facade.select("job-1");
    expect(capabilities).toHaveBeenCalledTimes(1);
  });
});
