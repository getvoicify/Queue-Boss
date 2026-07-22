import { computed, Directive, input } from "@angular/core";
import type { JobState } from "../../core/models";

@Directive({
  selector: "[appStateColor]",
  host: {
    "[class.qb-state]": "true",
    "[attr.data-state]": "appStateColor()",
    "[style.--qb-state-color]": "colorVar()",
  },
})
export class StateColorDirective {
  readonly appStateColor = input.required<JobState>();

  protected readonly colorVar = computed(
    () => `var(--qb-state-${this.appStateColor()})`,
  );
}
