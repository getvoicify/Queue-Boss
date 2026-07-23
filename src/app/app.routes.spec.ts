import type { Route } from "@angular/router";
import { describe, expect, it } from "vitest";
import { routes } from "./app.routes";
import { LifecycleHomeContainerComponent } from "./features/lifecycle/lifecycle-home-container.component";

function routeFor(path: string): Route | undefined {
  return routes.find((r) => r.path === path);
}

describe("app routes", () => {
  it("lands the default route on home", () => {
    const empty = routeFor("");
    expect(empty).toBeDefined();
    expect(empty?.pathMatch).toBe("full");
    expect(empty?.redirectTo).toBe("home");
  });

  it("serves the home route from the lifecycle home container with a distinct title", () => {
    const home = routeFor("home");
    expect(home?.component).toBe(LifecycleHomeContainerComponent);
    const title = home?.title;
    expect(typeof title).toBe("string");
    expect((title as string).length).toBeGreaterThan(0);
    expect(title).not.toBe("Lifecycle");
  });

  it("keeps the four secondary routes reachable", () => {
    for (const path of ["overview", "jobs", "lifecycle", "connect"]) {
      const route = routeFor(path);
      expect(route, `route ${path} exists`).toBeDefined();
      expect(route?.component, `route ${path} has a component`).toBeDefined();
    }
  });
});
