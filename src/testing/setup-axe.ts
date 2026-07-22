import { expect } from "vitest";
import * as matchers from "vitest-axe/matchers";

expect.extend(matchers);

declare module "vitest" {
  interface Assertion<T> {
    toHaveNoViolations(): T;
  }
}
