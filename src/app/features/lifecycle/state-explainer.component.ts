import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  input,
  output,
  viewChild,
} from "@angular/core";
import type { JobState } from "../../core/models";
import type { StateCopy } from "./lifecycle-teaching";

// Per-instance title id so two explainers never collide on aria-labelledby.
let explainerSeq = 0;

// Dumb, reusable per-state teaching popover. Data in (`state` + its `copy`),
// dismiss out. Focus/keyboard/outside-click handling is presentational a11y,
// hand-rolled because no @angular/cdk exists: focus moves in on open, Esc
// dismisses and returns focus to the originating element, an outside click
// dismisses. No data service, no invoke.
@Component({
  selector: "app-state-explainer",
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    "(keydown.escape)": "onEscape()",
    "(document:click)": "onDocumentClick($event)",
  },
  template: `
    <div
      class="state-explainer"
      data-testid="state-explainer"
      role="dialog"
      [attr.aria-labelledby]="titleId"
      [attr.data-state]="state()"
      [style.--qb-ex-bg]="ramp('bg')"
      [style.--qb-ex-border]="ramp('border')"
      [style.--qb-ex-text]="ramp('text')"
      [style.--qb-ex-solid]="ramp('solid')"
    >
      <div class="state-explainer__header">
        <p class="state-explainer__title" [id]="titleId">{{ copy().title }}</p>
        <button
          #dismissButton
          type="button"
          class="state-explainer__dismiss"
          data-testid="state-explainer-dismiss"
          aria-label="Close explanation"
          (click)="onDismiss()"
        >
          <span aria-hidden="true">✕</span>
        </button>
      </div>
      <p
        class="state-explainer__body"
        data-testid="state-annotation"
        [attr.data-state]="state()"
      >
        {{ copy().body }}
      </p>
    </div>
  `,
  styles: `
    .state-explainer {
      display: flex;
      flex-direction: column;
      gap: 8px;
      max-width: 360px;
      padding: 12px 14px;
      border-radius: 8px;
      background: var(--qb-ex-bg);
      border: 1px solid var(--qb-ex-border);
      color: var(--qb-ex-text);
      font-family: var(--qb-font-sans);
      box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
    }
    .state-explainer__header {
      display: flex;
      align-items: flex-start;
      gap: 8px;
    }
    .state-explainer__title {
      margin: 0;
      font-size: 14px;
      font-weight: 600;
    }
    .state-explainer__dismiss {
      margin-left: auto;
      flex: none;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 24px;
      height: 24px;
      padding: 0;
      border-radius: 5px;
      border: 1px solid var(--qb-ex-border);
      background: transparent;
      color: var(--qb-ex-text);
      cursor: pointer;
      font-size: 13px;
      line-height: 1;
    }
    .state-explainer__dismiss:focus-visible {
      outline: 2px solid var(--qb-ex-solid);
      outline-offset: 2px;
    }
    .state-explainer__body {
      margin: 0;
      font-size: 13px;
      line-height: 1.4;
    }
  `,
})
export class StateExplainerComponent {
  readonly state = input.required<JobState>();
  readonly copy = input.required<StateCopy>();
  readonly origin = input<HTMLElement | null>(null);
  readonly dismiss = output<void>();

  protected readonly titleId = `state-explainer-title-${++explainerSeq}`;
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly dismissButton =
    viewChild<ElementRef<HTMLButtonElement>>("dismissButton");

  constructor() {
    afterNextRender(() => this.dismissButton()?.nativeElement.focus());
  }

  protected ramp(step: "bg" | "border" | "text" | "solid"): string {
    return `var(--qb-state-${this.state()}-${step})`;
  }

  protected onDismiss(): void {
    this.dismissAndReturnFocus();
  }

  protected onEscape(): void {
    this.dismissAndReturnFocus();
  }

  // Both dismiss paths (Esc, ✕) return focus to the originator before emitting
  // so a keyboard user never loses their place.
  private dismissAndReturnFocus(): void {
    this.origin()?.focus();
    this.dismiss.emit();
  }

  protected onDocumentClick(event: MouseEvent): void {
    const target = event.target as Node | null;
    if (target && !this.host.nativeElement.contains(target)) {
      this.dismiss.emit();
    }
  }
}
