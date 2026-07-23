import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { beforeEach, describe, expect, it } from "vitest";
import { AppComponent } from "./app.component";
import { ConnectionsFacade } from "./core/facades/connections.facade";

describe("AppComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [provideRouter([])],
    }).compileComponents();
  });

  it("renders the dark app shell", () => {
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const shell = fixture.nativeElement.querySelector(
      '[data-testid="app-shell"]',
    );
    expect(shell).not.toBeNull();
  });

  it("renders a per-connection status chip from the facade entries", () => {
    TestBed.overrideProvider(ConnectionsFacade, {
      useValue: {
        entries: signal([
          { id: "sandbox", status: "connecting" as const },
        ]).asReadonly(),
      },
    });
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const chip = fixture.nativeElement.querySelector(
      '[data-testid="connection-status-sandbox"]',
    );
    expect(chip).not.toBeNull();
    expect(chip.getAttribute("data-status")).toBe("connecting");
    expect(
      fixture.nativeElement
        .querySelector('[data-testid="connection-label"]')
        .textContent.trim(),
    ).toBe("Connecting…");
  });
});
