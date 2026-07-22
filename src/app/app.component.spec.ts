import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { beforeEach, describe, expect, it } from "vitest";
import { AppComponent } from "./app.component";
import { ConnectionFacade } from "./core/facades/connection.facade";

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

  it("binds the connection facade status into the shell", () => {
    TestBed.overrideProvider(ConnectionFacade, {
      useValue: { status: signal("connecting").asReadonly() },
    });
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const label = fixture.nativeElement.querySelector(
      '[data-testid="connection-label"]',
    );
    expect(label.textContent.trim()).toBe("Connecting…");
  });
});
