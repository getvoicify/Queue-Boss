import type { JobState } from "./job-state";

export interface QueueSummary {
  name: string;
  totalDepth: number;
  countsByState: Partial<Record<JobState, number>>;
  oldestWaitingAge: number | null;
}

export interface QueueCountEntry {
  queue: string;
  totalDepth: number;
  countsByState: Partial<Record<JobState, number>>;
  oldestWaitingAge: number | null;
}

export interface QueueCounts {
  connectionId: string;
  queues: QueueCountEntry[];
  polledAt: number;
}
