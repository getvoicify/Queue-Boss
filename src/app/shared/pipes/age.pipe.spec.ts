import { describe, expect, it } from "vitest";
import { AgePipe } from "./age.pipe";

describe("AgePipe", () => {
  const pipe = new AgePipe();

  it("renders an em dash when the age is null", () => {
    expect(pipe.transform(null)).toBe("—");
  });

  it("renders 'just now' for durations under a minute", () => {
    expect(pipe.transform(0)).toBe("just now");
    expect(pipe.transform(45)).toBe("just now");
  });

  it("renders whole minutes", () => {
    expect(pipe.transform(180)).toBe("3m");
  });

  it("renders whole hours", () => {
    expect(pipe.transform(7200)).toBe("2h");
  });

  it("renders whole days", () => {
    expect(pipe.transform(172800)).toBe("2d");
  });
});
