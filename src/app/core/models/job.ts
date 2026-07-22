import type { JobState } from "./job-state";

export interface JobSummary {
  id: string;
  queue: string;
  state: JobState;
  createdAt: number;
  startedAt: number | null;
  completedAt: number | null;
  attempts: number;
  priority: number;
}

export interface TimelineEvent {
  at: number;
  state: JobState;
}

export interface RetryReadout {
  attempts: number;
  maxAttempts: number | null;
  nextRetryAt: number | null;
}

export interface JobDetail extends JobSummary {
  data: unknown;
  output: unknown;
  timeline: TimelineEvent[];
  retry: RetryReadout;
  extensions: Record<string, unknown>;
}

export interface TimeWindow {
  from: number;
  to: number;
}

export interface Page<T> {
  items: T[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface JobFilter {
  queue?: string;
  states?: JobState[];
  timeWindow?: TimeWindow;
  search?: string;
  cursor?: string;
  limit: number;
}
