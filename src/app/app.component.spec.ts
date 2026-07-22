import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { beforeEach, describe, expect, it } from "vitest";
import { AppComponent } from "./app.component";

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
});
