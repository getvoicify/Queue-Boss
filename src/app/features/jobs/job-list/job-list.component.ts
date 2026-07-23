import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from "@angular/core";
import type { JobFilter, JobState, JobSummary } from "../../../core/models";
import { StateColorDirective } from "../../../shared/directives/state-color.directive";
import { TimestampPipe } from "../../../shared/pipes/timestamp.pipe";

const DEFAULT_LIMIT = 20;
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

// Dumb job-list table: renders state-colored rows and emits selection,
// pagination and filter intents. No service or `invoke` — a container wires it.
@Component({
  selector: "app-job-list",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [StateColorDirective, TimestampPipe],
  template: `
    <section class="job-list" aria-label="Jobs">
      <div class="job-list__filter">
        <label for="job-filter-state">Filter by state</label>
        <select
          id="job-filter-state"
          data-testid="job-filter-state"
          (change)="onFilterChange($event)"
        >
          <option value="">All states</option>
          @for (state of states; track state) {
            <option [value]="state">{{ labels[state] }}</option>
          }
        </select>
      </div>
      <table class="job-list__table">
        <caption>
          Jobs
        </caption>
        <thead>
          <tr>
            <th scope="col">ID</th>
            <th scope="col">State</th>
            <th scope="col">Created</th>
            <th scope="col">Started / Completed</th>
            <th scope="col">Attempts</th>
            <th scope="col">Priority</th>
          </tr>
        </thead>
        <tbody>
          @for (job of jobs(); track job.id) {
            <tr
              data-testid="job-row"
              [appStateColor]="job.state"
              tabindex="0"
              [attr.aria-label]="'View job ' + job.id"
              (click)="select.emit(job.id)"
              (keydown.enter)="select.emit(job.id)"
              (keydown.space)="select.emit(job.id)"
            >
              <td [attr.data-testid]="'job-id-' + job.id">{{ job.id }}</td>
              <td>{{ labels[job.state] }}</td>
              <td>{{ job.createdAt | timestamp }}</td>
              <td>{{ startedOrCompleted(job) | timestamp }}</td>
              <td class="job-list__num">{{ job.attempts }}</td>
              <td class="job-list__num">{{ job.priority }}</td>
            </tr>
          } @empty {
            <tr>
              <td data-testid="job-list-empty" [attr.colspan]="6">
                No jobs to display.
              </td>
            </tr>
          }
        </tbody>
      </table>
      @if (hasMore()) {
        <button
          type="button"
          data-testid="jobs-load-more"
          (click)="loadMore.emit()"
        >
          Load more
        </button>
      }
    </section>
  `,
})
export class JobListComponent {
  readonly jobs = input<JobSummary[]>([]);
  readonly hasMore = input(false);
  readonly filterChange = output<JobFilter>();
  readonly loadMore = output<void>();
  readonly select = output<string>();

  protected readonly states = JOB_STATES;
  protected readonly labels = JOB_STATE_LABELS;

  protected startedOrCompleted(job: JobSummary): number | null {
    return job.startedAt ?? job.completedAt;
  }

  protected onFilterChange(event: Event): void {
    const value = (event.target as HTMLSelectElement).value as JobState | "";
    this.filterChange.emit(
      value
        ? { states: [value], limit: DEFAULT_LIMIT }
        : { limit: DEFAULT_LIMIT },
    );
  }
}
