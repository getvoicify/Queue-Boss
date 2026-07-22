import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import { ShellComponent } from "./shell.component";

describe("ShellComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ShellComponent],
      providers: [provideRouter([])],
    }).compileComponents();
  });

  it("renders the chrome: shell region, primary nav and connection status", () => {
    const fixture = TestBed.createComponent(ShellComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement;
    expect(el.querySelector('[data-testid="app-shell"]')).not.toBeNull();
    expect(el.querySelector("app-primary-nav")).not.toBeNull();
    expect(
      el.querySelector('[data-testid="connection-status"]'),
    ).not.toBeNull();
  });

  it("defaults the connection status to idle", () => {
    const fixture = TestBed.createComponent(ShellComponent);
    fixture.detectChanges();
    expect(
      fixture.nativeElement
        .querySelector('[data-testid="connection-status"]')
        .getAttribute("data-status"),
    ).toBe("idle");
  });

  it("has no accessibility violations", async () => {
    const fixture = TestBed.createComponent(ShellComponent);
    fixture.detectChanges();
    expect(await axe(fixture.nativeElement)).toHaveNoViolations();
  });
});
