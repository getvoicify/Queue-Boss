import { formatDate } from "@angular/common";
import { TestBed } from "@angular/core/testing";
import { describe, expect, it } from "vitest";
import { TimestampPipe } from "./timestamp.pipe";

function pipe(): TimestampPipe {
  return TestBed.runInInjectionContext(() => new TimestampPipe());
}

describe("TimestampPipe", () => {
  it("renders an em dash for null", () => {
    expect(pipe().transform(null)).toBe("—");
  });

  it("formats an epoch-ms timestamp as a human-readable medium date", () => {
    const ts = 1_700_000_000_000;
    expect(pipe().transform(ts)).toBe(formatDate(ts, "medium", "en-US"));
    expect(pipe().transform(ts)).not.toContain(String(ts));
  });
});
