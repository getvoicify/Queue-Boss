import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { ConnectionEntry } from "../core/models";
import { ShellComponent } from "./shell.component";

describe("ShellComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ShellComponent],
      providers: [provideRouter([])],
    }).compileComponents();
  });

  function render(connections?: ConnectionEntry[]) {
    const fixture = TestBed.createComponent(ShellComponent);
    if (connections) {
      fixture.componentRef.setInput("connections", connections);
    }
    fixture.detectChanges();
    return fixture;
  }

  it("renders the chrome: shell region, primary nav and the sandbox status chip", () => {
    const el = render().nativeElement;
    expect(el.querySelector('[data-testid="app-shell"]')).not.toBeNull();
    expect(el.querySelector("app-primary-nav")).not.toBeNull();
    expect(
      el.querySelector('[data-testid="connection-status-sandbox"]'),
    ).not.toBeNull();
  });

  it("renders one status chip per connection entry", () => {
    const el = render([
      { id: "sandbox", status: "connected" },
      { id: "pgboss", status: "connecting" },
    ]).nativeElement;
    expect(
      el
        .querySelector('[data-testid="connection-status-sandbox"]')
        .getAttribute("data-status"),
    ).toBe("connected");
    expect(
      el
        .querySelector('[data-testid="connection-status-pgboss"]')
        .getAttribute("data-status"),
    ).toBe("connecting");
  });

  it("defaults the sandbox chip to idle", () => {
    const el = render().nativeElement;
    expect(
      el
        .querySelector('[data-testid="connection-status-sandbox"]')
        .getAttribute("data-status"),
    ).toBe("idle");
  });

  it("has no accessibility violations", async () => {
    const el = render([
      { id: "sandbox", status: "connected" },
      { id: "pgboss", status: "error", message: "database is not reachable" },
    ]).nativeElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
