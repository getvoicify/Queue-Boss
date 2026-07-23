import {
  ChangeDetectionStrategy,
  Component,
  effect,
  inject,
  untracked,
} from "@angular/core";
import { ConnectionsFacade } from "../../core/facades/connections.facade";
import { JobsFacade } from "../../core/facades/jobs.facade";
import { JobDetailComponent } from "./job-detail/job-detail.component";
import { JobListComponent } from "./job-list/job-list.component";

// Jobs route container: wires the dumb list + detail panel to the JobsFacade
// and reloads the first page whenever the active connection changes. Touches
// facades only — no service or `invoke`.
@Component({
  selector: "app-jobs-container",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [JobListComponent, JobDetailComponent],
  template: `
    <app-job-list
      [jobs]="facade.jobs()"
      [hasMore]="facade.hasMore()"
      (filterChange)="facade.setFilter($event)"
      (loadMore)="facade.loadPage()"
      (select)="facade.select($event)"
    />
    @if (facade.selected(); as job) {
      @if (facade.capabilities(); as capabilities) {
        <app-job-detail [job]="job" [capabilities]="capabilities" />
      }
    }
  `,
})
export class JobsContainerComponent {
  protected readonly facade = inject(JobsFacade);
  readonly #active = inject(ConnectionsFacade).activeConnectionId;

  constructor() {
    effect(() => {
      this.#active();
      untracked(() => void this.facade.setFilter(this.facade.filter()));
    });
  }
}
