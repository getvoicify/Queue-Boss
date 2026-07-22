import { ChangeDetectionStrategy, Component, input } from "@angular/core";
import type { JobState, QueueCountEntry } from "../../core/models";
import { AgePipe } from "../../shared/pipes/age.pipe";

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
  selector: "app-overview",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [AgePipe],
  styleUrl: "./overview.component.css",
  template: `
    <section class="overview" aria-label="Queue overview">
      <table class="overview__table">
        <caption>
          Queues
        </caption>
        <thead>
          <tr>
            <th scope="col">Queue</th>
            <th scope="col">Depth</th>
            @for (state of states; track state) {
              <th scope="col">{{ labels[state] }}</th>
            }
            <th scope="col">Oldest waiting</th>
          </tr>
        </thead>
        <tbody>
          @for (entry of queues(); track entry.queue) {
            <tr data-testid="queue-row">
              <th scope="row" [attr.data-testid]="'queue-' + entry.queue">
                {{ entry.queue }}
              </th>
              <td class="overview__num" [attr.data-testid]="'depth-' + entry.queue">
                {{ entry.totalDepth }}
              </td>
              @for (state of states; track state) {
                <td
                  class="overview__num"
                  [attr.data-testid]="'count-' + entry.queue + '-' + state"
                >
                  {{ entry.countsByState[state] ?? 0 }}
                </td>
              }
              <td [attr.data-testid]="'oldest-' + entry.queue">
                {{ entry.oldestWaitingAge | age }}
              </td>
            </tr>
          } @empty {
            <tr>
              <td
                data-testid="overview-empty"
                [attr.colspan]="states.length + 3"
              >
                No queues to display.
              </td>
            </tr>
          }
        </tbody>
      </table>
    </section>
  `,
})
export class OverviewComponent {
  readonly queues = input<QueueCountEntry[]>([]);
  protected readonly states = JOB_STATES;
  protected readonly labels = JOB_STATE_LABELS;
}
