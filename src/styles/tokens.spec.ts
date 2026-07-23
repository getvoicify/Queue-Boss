import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const tokensCss = readFileSync(
  resolve(process.cwd(), "src/styles/tokens.css"),
  "utf-8",
);

const STATES = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
] as const;

const STEPS = ["bg", "border", "solid", "text"] as const;

describe("state OKLCH ramp tokens", () => {
  it("defines the four valued ramp steps for every JobState", () => {
    for (const state of STATES) {
      for (const step of STEPS) {
        expect(tokensCss, `--qb-state-${state}-${step}`).toMatch(
          new RegExp(`--qb-state-${state}-${step}:\\s*oklch\\(`),
        );
      }
    }
  });

  it("aliases each state to its solid step so consumers keep resolving", () => {
    for (const state of STATES) {
      expect(tokensCss, `${state} alias`).toMatch(
        new RegExp(
          `--qb-state-${state}:\\s*var\\(--qb-state-${state}-solid\\)`,
        ),
      );
    }
  });

  it("leaves the -on foreground slot reserved with no value", () => {
    for (const state of STATES) {
      expect(tokensCss, `${state}-on reserved`).not.toMatch(
        new RegExp(`--qb-state-${state}-on\\s*:`),
      );
    }
  });

  it("adds the strong connector token", () => {
    expect(tokensCss).toMatch(/--qb-border-strong:\s*oklch\(0\.62 0\.02 250\)/);
  });

  it("keeps the connection-status family untouched", () => {
    for (const status of ["connected", "connecting", "error"]) {
      expect(tokensCss, `--qb-status-${status}`).toMatch(
        new RegExp(`--qb-status-${status}:`),
      );
    }
  });
});

describe("font-family tokens", () => {
  it("defines the sans family token pointing at IBM Plex Sans", () => {
    expect(tokensCss).toMatch(/--qb-font-sans:\s*["']?IBM Plex Sans/);
  });

  it("exposes a mono family token pointing at IBM Plex Mono", () => {
    expect(tokensCss).toMatch(
      /--qb-font-mono:\s*["']?IBM Plex Mono["']?[^;]*monospace/,
    );
  });
});
