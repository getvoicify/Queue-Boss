import { expect } from "vitest";
import * as matchers from "vitest-axe/matchers";

expect.extend(matchers);

declare module "vitest" {
  interface Assertion<T> {
    toHaveNoViolations(): T;
  }
}

const noop = (): void => undefined;

// jsdom omits window.matchMedia; components that read prefers-reduced-motion
// throw "matchMedia is not a function" without this. Tests may reassign
// window.matchMedia to force a reduced-motion match.
if (typeof window !== "undefined" && typeof window.matchMedia !== "function") {
  window.matchMedia = ((query: string): MediaQueryList => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: noop,
    removeEventListener: noop,
    addListener: noop,
    removeListener: noop,
    dispatchEvent: (): boolean => false,
  })) as typeof window.matchMedia;
}
