import { ChangeDetectionStrategy, Component, input } from "@angular/core";
import type { JobState } from "../../core/models";
import { StateColorDirective } from "../../shared/directives/state-color.directive";

const JOB_STATES: readonly JobState[] = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
];
const JOB_STATE_LABELS: Record<JobState, string> = {
  created: "Created",
  active: "Active",
  completed: "Completed",
  failed: "Failed",
  cancelled: "Cancelled",
  retry: "Retry",
  deadLetter: "Dead letter",
};

@Component({
  selector: "app-lifecycle",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [StateColorDirective],
  styleUrl: "./lifecycle.component.css",
  template: `
    <section class="lifecycle" aria-label="Job lifecycle counts">
      <dl class="lifecycle__list">
        @for (state of states; track state) {
          <div class="lifecycle__item" data-testid="lifecycle-item">
            <dt class="lifecycle__label">
              <span class="lifecycle__dot" aria-hidden="true" [appStateColor]="state"></span>
              {{ labels[state] }}
            </dt>
            <dd
              class="lifecycle__value"
              [attr.data-testid]="'lifecycle-value-' + state"
            >
              {{ counts()[state] ?? 0 }}
            </dd>
          </div>
        }
      </dl>
    </section>
  `,
})
export class LifecycleComponent {
  readonly counts = input<Partial<Record<JobState, number>>>({});
  protected readonly states = JOB_STATES;
  protected readonly labels = JOB_STATE_LABELS;
}
