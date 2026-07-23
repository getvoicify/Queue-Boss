import { describe, expect, it } from "vitest";
import { JsonPreviewPipe } from "./json-preview.pipe";

describe("JsonPreviewPipe", () => {
  const pipe = new JsonPreviewPipe();

  it("stringifies an object compactly", () => {
    expect(pipe.transform({ a: 1, b: "x" })).toBe('{"a":1,"b":"x"}');
  });

  it("renders null and undefined as an em dash", () => {
    expect(pipe.transform(null)).toBe("—");
    expect(pipe.transform(undefined)).toBe("—");
  });

  it("truncates very long output with an ellipsis", () => {
    const out = pipe.transform({ data: "x".repeat(500) });
    expect(out.length).toBe(201);
    expect(out.endsWith("…")).toBe(true);
  });
});
