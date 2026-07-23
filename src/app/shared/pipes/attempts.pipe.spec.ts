import { describe, expect, it } from "vitest";
import type { RetryReadout } from "../../core/models";
import { AttemptsPipe } from "./attempts.pipe";

function readout(attempts: number, maxAttempts: number | null): RetryReadout {
  return { attempts, maxAttempts, nextRetryAt: null };
}

describe("AttemptsPipe", () => {
  const pipe = new AttemptsPipe();

  it("renders 'N of M' when the max attempts are known", () => {
    expect(pipe.transform(readout(2, 5))).toBe("2 of 5");
  });

  it("renders just the attempt count when maxAttempts is null", () => {
    expect(pipe.transform(readout(3, null))).toBe("3");
  });
});
