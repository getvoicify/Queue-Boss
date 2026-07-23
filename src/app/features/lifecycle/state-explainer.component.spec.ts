import { Component, computed, signal } from "@angular/core";
import { type ComponentFixture, TestBed } from "@angular/core/testing";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { JobState } from "../../core/models";
import { LIFECYCLE_COPY, type StateCopy } from "./lifecycle-teaching";
import { StateExplainerComponent } from "./state-explainer.component";

// Spec-only stub host (NOT LifecycleHomeContainerComponent, per #48 pin): mounts
// the explainer next to a REAL focusable originating <button> so focus-move-in
// and Esc-focus-return can be asserted against document.activeElement.
@Component({
  imports: [StateExplainerComponent],
  template: `
    <button type="button" data-testid="origin">Open explainer</button>
    @if (open()) {
      <app-state-explainer
        [state]="state()"
        [copy]="copy()"
        [origin]="originEl()"
        (dismiss)="onDismiss()"
      />
    }
  `,
})
class StubHostComponent {
  readonly open = signal(true);
  readonly state = signal<JobState>("retry");
  readonly copy = computed<StateCopy>(() => LIFECYCLE_COPY[this.state()]);
  readonly originEl = signal<HTMLElement | null>(null);
  readonly dismissed = signal(0);

  onDismiss(): void {
    this.dismissed.update((n) => n + 1);
    this.open.set(false);
  }
}

let mounted: ComponentFixture<StubHostComponent> | null = null;

// Amendment (#48 pin): attach to document.body so jsdom reflects .focus() in
// document.activeElement, and remove it in afterEach.
function mount(state: JobState = "retry") {
  const fixture = TestBed.createComponent(StubHostComponent);
  mounted = fixture;
  const host = fixture.componentInstance;
  host.state.set(state);
  document.body.appendChild(fixture.nativeElement);
  fixture.detectChanges();
  const el = fixture.nativeElement as HTMLElement;
  const origin = el.querySelector(
    '[data-testid="origin"]',
  ) as HTMLButtonElement;
  host.originEl.set(origin);
  fixture.detectChanges();
  return { fixture, host, el, origin };
}

function explainerEl(el: HTMLElement): HTMLElement {
  return el.querySelector('[data-testid="state-explainer"]') as HTMLElement;
}

describe("StateExplainerComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [StubHostComponent],
    }).compileComponents();
  });

  afterEach(() => {
    if (mounted) {
      mounted.nativeElement.remove();
      mounted.destroy();
      mounted = null;
    }
    TestBed.resetTestingModule();
  });

  it("renders the given state's title and body", () => {
    const { el } = mount("retry");
    const explainer = explainerEl(el);
    expect(explainer).not.toBeNull();
    expect(explainer.textContent).toContain(LIFECYCLE_COPY.retry.title);
    expect(explainer.textContent).toContain(LIFECYCLE_COPY.retry.body);

    const annotation = el.querySelector(
      '[data-testid="state-annotation"]',
    ) as HTMLElement;
    expect(annotation).not.toBeNull();
    expect(annotation.textContent).toContain(LIFECYCLE_COPY.retry.body);
  });

  it("moves focus into the popover on open", async () => {
    const { fixture, el } = mount();
    await fixture.whenStable();
    expect(explainerEl(el).contains(document.activeElement)).toBe(true);
  });

  it("dismisses and returns focus to the origin when Esc is pressed", async () => {
    const { fixture, host, el, origin } = mount();
    await fixture.whenStable();

    const dismissBtn = el.querySelector(
      '[data-testid="state-explainer-dismiss"]',
    ) as HTMLButtonElement;
    dismissBtn.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
    );
    fixture.detectChanges();

    expect(host.dismissed()).toBe(1);
    expect(document.activeElement).toBe(origin);
  });

  it("emits dismiss when the dismiss control is clicked", () => {
    const { fixture, host, el } = mount();
    const dismissBtn = el.querySelector(
      '[data-testid="state-explainer-dismiss"]',
    ) as HTMLButtonElement;
    expect(dismissBtn.tagName).toBe("BUTTON");

    dismissBtn.click();
    fixture.detectChanges();

    expect(host.dismissed()).toBe(1);
  });

  it("dismisses and returns focus to the origin when the dismiss control is clicked", async () => {
    const { fixture, host, el, origin } = mount();
    await fixture.whenStable();

    const dismissBtn = el.querySelector(
      '[data-testid="state-explainer-dismiss"]',
    ) as HTMLButtonElement;
    dismissBtn.click();
    fixture.detectChanges();

    expect(host.dismissed()).toBe(1);
    expect(document.activeElement).toBe(origin);
  });

  it("dismisses on an outside click", () => {
    const { fixture, host } = mount();
    document.body.click();
    fixture.detectChanges();
    expect(host.dismissed()).toBe(1);
  });

  it("is an aria-labelled dialog naming the state", () => {
    const { el } = mount("deadLetter");
    const explainer = explainerEl(el);
    expect(explainer.getAttribute("role")).toBe("dialog");

    const labelledby = explainer.getAttribute("aria-labelledby");
    expect(labelledby).toBeTruthy();
    const label = el.querySelector(`#${labelledby}`);
    expect(label).not.toBeNull();
    expect(label?.textContent).toContain(LIFECYCLE_COPY.deadLetter.title);
  });

  it("has no accessibility violations", async () => {
    const { fixture, el } = mount();
    await fixture.whenStable();
    expect(await axe(explainerEl(el))).toHaveNoViolations();
  });
});
