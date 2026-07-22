export type JobState =
  | "created"
  | "active"
  | "completed"
  | "failed"
  | "cancelled"
  | "retry"
  | "deadLetter";
