import { Component } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { describe, expect, it } from "vitest";
import type { JobState } from "../../core/models";
import { StateColorDirective } from "./state-color.directive";

@Component({
  template: `<span class="marker" data-testid="dot" [appStateColor]="state"></span>`,
  imports: [StateColorDirective],
})
class HostComponent {
  state: JobState = "created";
}

const STATES: JobState[] = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
];

describe("StateColorDirective", () => {
  it("marks the host and maps each JobState to its data attribute and color property", () => {
    for (const state of STATES) {
      const fixture = TestBed.createComponent(HostComponent);
      fixture.componentInstance.state = state;
      fixture.detectChanges();
      const dot = fixture.nativeElement.querySelector('[data-testid="dot"]');
      expect(dot.classList.contains("qb-state")).toBe(true);
      expect(dot.getAttribute("data-state")).toBe(state);
      expect(dot.style.getPropertyValue("--qb-state-color")).toBe(
        `var(--qb-state-${state})`,
      );
    }
  });

  it("preserves static classes on the host element", () => {
    const fixture = TestBed.createComponent(HostComponent);
    fixture.detectChanges();
    const dot = fixture.nativeElement.querySelector('[data-testid="dot"]');
    expect(dot.classList.contains("marker")).toBe(true);
    expect(dot.classList.contains("qb-state")).toBe(true);
  });
});
