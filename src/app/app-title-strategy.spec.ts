import { TestBed } from "@angular/core/testing";
import { Title } from "@angular/platform-browser";
import type { RouterStateSnapshot } from "@angular/router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppTitleStrategy } from "./app-title-strategy";

describe("AppTitleStrategy", () => {
  let strategy: AppTitleStrategy;
  let title: Title;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [AppTitleStrategy, Title] });
    strategy = TestBed.inject(AppTitleStrategy);
    title = TestBed.inject(Title);
  });

  it("appends the app name to a route title", () => {
    vi.spyOn(strategy, "buildTitle").mockReturnValue("Overview");
    strategy.updateTitle({} as RouterStateSnapshot);
    expect(title.getTitle()).toBe("Overview · Queue Boss");
  });

  it("falls back to the app name when the route has no title", () => {
    vi.spyOn(strategy, "buildTitle").mockReturnValue(undefined);
    strategy.updateTitle({} as RouterStateSnapshot);
    expect(title.getTitle()).toBe("Queue Boss");
  });
});
