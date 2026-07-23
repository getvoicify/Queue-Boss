import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesCss = readFileSync(
  resolve(process.cwd(), "src/styles.css"),
  "utf-8",
);

const lifecycleCss = readFileSync(
  resolve(process.cwd(), "src/app/features/lifecycle/lifecycle.component.css"),
  "utf-8",
);

const overviewCss = readFileSync(
  resolve(process.cwd(), "src/app/features/overview/overview.component.css"),
  "utf-8",
);

describe("self-hosted IBM Plex fonts", () => {
  it("registers @font-face for both IBM Plex Sans and IBM Plex Mono", () => {
    expect(stylesCss).toMatch(/@font-face/);
    expect(stylesCss).toMatch(/font-family:\s*["']IBM Plex Sans["']/);
    expect(stylesCss).toMatch(/font-family:\s*["']IBM Plex Mono["']/);
  });

  it("registers the faces with font-display: swap", () => {
    expect(stylesCss).toMatch(/font-display:\s*swap/);
  });

  it("drives the global UI font off the sans family token", () => {
    expect(stylesCss).toMatch(/font-family:\s*var\(--qb-font-sans\)/);
  });

  it("no longer hard-codes Inter as the body font", () => {
    expect(stylesCss).not.toMatch(/font-family:\s*Inter/);
  });
});

describe("mono font applied to numeric count displays", () => {
  it("applies the mono family to the lifecycle count value", () => {
    expect(lifecycleCss).toMatch(
      /\.lifecycle__value\s*\{[^}]*font-family:\s*var\(--qb-font-mono\)/s,
    );
  });

  it("applies the mono family to the overview numeric cells", () => {
    expect(overviewCss).toMatch(
      /\.overview__num\s*\{[^}]*font-family:\s*var\(--qb-font-mono\)/s,
    );
  });
});
