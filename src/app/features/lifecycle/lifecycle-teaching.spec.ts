import { describe, expect, it } from "vitest";
import type { JobState } from "../../core/models";
import { LIFECYCLE_COPY } from "./lifecycle-teaching";

const STATES: JobState[] = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
];

describe("LIFECYCLE_COPY", () => {
  it("has a non-empty title and body for every JobState", () => {
    for (const state of STATES) {
      const entry = LIFECYCLE_COPY[state];
      expect(entry, `copy for ${state} exists`).toBeTruthy();
      expect(
        entry.title.trim().length,
        `title for ${state} is non-empty`,
      ).toBeGreaterThan(0);
      expect(
        entry.body.trim().length,
        `body for ${state} is non-empty`,
      ).toBeGreaterThan(0);
    }
  });

  it("covers exactly the seven job states and nothing else", () => {
    expect(Object.keys(LIFECYCLE_COPY).sort()).toEqual([...STATES].sort());
  });

  it("encodes the distinguishing pg-boss semantics per state", () => {
    expect(LIFECYCLE_COPY.created.body.toLowerCase()).toContain("waiting");
    expect(LIFECYCLE_COPY.active.body.toLowerCase()).toContain("worker");
    expect(LIFECYCLE_COPY.completed.body.toLowerCase()).toContain("success");
    expect(LIFECYCLE_COPY.failed.body.toLowerCase()).toContain("fail");
    expect(LIFECYCLE_COPY.retry.body.toLowerCase()).toContain("backoff");
    expect(LIFECYCLE_COPY.cancelled.body.toLowerCase()).toContain("cancel");
    const deadLetter = LIFECYCLE_COPY.deadLetter.body.toLowerCase();
    expect(deadLetter).toContain("dead-letter");
    expect(deadLetter).toContain("routed");
  });
});
