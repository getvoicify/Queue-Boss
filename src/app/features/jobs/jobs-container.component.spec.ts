import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ConnectionsFacade } from "../../core/facades/connections.facade";
import { JobsFacade } from "../../core/facades/jobs.facade";
import type {
  Capabilities,
  JobDetail,
  JobFilter,
  JobSummary,
} from "../../core/models";
import { JobsContainerComponent } from "./jobs-container.component";

function summary(id: string): JobSummary {
  return {
    id,
    queue: "emails",
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

const jobs = signal<JobSummary[]>([]);
const hasMore = signal(false);
const selected = signal<JobDetail | null>(null);
const capabilities = signal<Capabilities | null>(null);
const filter = signal<JobFilter>({ limit: 20 });

function facadeStub() {
  return {
    jobs: jobs.asReadonly(),
    hasMore: hasMore.asReadonly(),
    selected: selected.asReadonly(),
    capabilities: capabilities.asReadonly(),
    filter: filter.asReadonly(),
    setFilter: vi.fn(),
    loadPage: vi.fn(),
    select: vi.fn(),
  };
}

let stub: ReturnType<typeof facadeStub>;

function create() {
  const fixture = TestBed.createComponent(JobsContainerComponent);
  fixture.detectChanges();
  return fixture;
}

describe("JobsContainerComponent", () => {
  beforeEach(() => {
    jobs.set([]);
    hasMore.set(false);
    selected.set(null);
    capabilities.set(null);
    filter.set({ limit: 20 });
    stub = facadeStub();
    TestBed.configureTestingModule({
      providers: [
        { provide: JobsFacade, useValue: stub },
        {
          provide: ConnectionsFacade,
          useValue: { activeConnectionId: signal("sandbox").asReadonly() },
        },
      ],
    });
  });

  it("loads the first page for the active connection on init", () => {
    create();
    expect(stub.setFilter).toHaveBeenCalledWith({ limit: 20 });
  });

  it("forwards a row selection to the facade", () => {
    jobs.set([summary("job-1")]);
    const el = create().nativeElement;
    el.querySelector('[data-testid="job-open"]').click();
    expect(stub.select).toHaveBeenCalledWith("job-1");
  });

  it("does not render the detail panel until both a job and its capabilities are present", () => {
    selected.set(detail("job-1"));
    const el = create().nativeElement;
    expect(el.querySelector('[data-testid="job-detail"]')).toBeNull();

    capabilities.set(caps);
    const el2 = create().nativeElement;
    expect(el2.querySelector('[data-testid="job-detail"]')).not.toBeNull();
  });
});
