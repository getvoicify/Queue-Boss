import type { JobState } from "../../core/models";

export interface StateCopy {
  readonly title: string;
  readonly body: string;
}

// The single source of teaching copy for each pg-boss job state. Kept as plain
// data so the explainer (and any future consumer) stays dumb. Semantics match
// the queue backends: `retry` waits out a backoff, `deadLetter` is a derived
// bucket for failed jobs routed to their configured dead-letter queue.
export const LIFECYCLE_COPY: Record<JobState, StateCopy> = {
  created: {
    title: "Created",
    body: "The job is enqueued and waiting for a free worker to pick it up.",
  },
  active: {
    title: "Active",
    body: "A worker has claimed the job and is running it right now.",
  },
  completed: {
    title: "Completed",
    body: "The job finished successfully and its result was recorded.",
  },
  failed: {
    title: "Failed",
    body: "The job errored and exhausted its retry attempts, so it failed with nowhere left to go.",
  },
  retry: {
    title: "Retry",
    body: "The job failed but still has attempts left, so it is scheduled to run again after a backoff delay.",
  },
  cancelled: {
    title: "Cancelled",
    body: "The job was cancelled before it could complete and will not run again.",
  },
  deadLetter: {
    title: "Dead letter",
    body: "A derived bucket: after exhausting its retries the failed job was routed to its configured dead-letter queue for triage.",
  },
};
