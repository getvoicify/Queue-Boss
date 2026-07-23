import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
} from "@angular/core";
import { ConnectionsFacade } from "../../core/facades/connections.facade";
import { QueuesFacade } from "../../core/facades/queues.facade";
import type { JobState } from "../../core/models";
import { LifecycleDiagramComponent } from "./lifecycle-diagram.component";

// Iterated explicitly rather than importing the unexported JOB_STATES so the
// fold owns its own dense key set.
const STATES: readonly JobState[] = [
  "created",
  "active",
  "completed",
  "failed",
  "cancelled",
  "retry",
  "deadLetter",
];

// Baseline teaching copy owned by the home container. E3-4's richer
// LIFECYCLE_COPY is swapped into this teaching slot when it lands; the home
// container must not hard-depend on it.
export const LOCAL_COPY: Record<JobState, string> = {
  created: "Enqueued and waiting for a worker to pick it up.",
  active: "A worker has claimed this job and is running it now.",
  completed: "Finished successfully and left the pipeline.",
  failed: "Threw and exhausted its retries.",
  cancelled: "Cancelled before it finished running.",
  retry: "Failed and waiting out a backoff before another attempt.",
  deadLetter: "Failed terminally and parked in the dead-letter queue.",
};

// Home / front-door container: connects the queues stream on entry (no
// enter-sandbox gate), folds every queue's per-state counts into the dense
// aggregate the hero renders, and owns the select -> teaching wiring.
@Component({
  selector: "app-lifecycle-home-container",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LifecycleDiagramComponent],
  styleUrl: "./lifecycle-home-container.component.css",
  template: `
    @if (hasCounts()) {
      <app-lifecycle-diagram
        data-testid="home-hero"
        [counts]="aggregate()"
        [selected]="selected()"
        [annotation]="annotation()"
        (selectState)="onSelect($event)"
      />
    } @else {
      <p class="home-waiting" data-testid="home-waiting" role="status">
        Waiting for queue activity…
      </p>
    }
  `,
})
export class LifecycleHomeContainerComponent {
  readonly #queues = inject(QueuesFacade);
  readonly #connections = inject(ConnectionsFacade);
  protected readonly selected = signal<JobState | null>(null);

  protected readonly hasCounts = computed(
    () => this.#queues.queues().length > 0,
  );

  protected readonly aggregate = computed<Record<JobState, number>>(() => {
    const totals: Record<JobState, number> = {
      created: 0,
      active: 0,
      completed: 0,
      failed: 0,
      cancelled: 0,
      retry: 0,
      deadLetter: 0,
    };
    for (const entry of this.#queues.queues()) {
      for (const state of STATES) {
        totals[state] += entry.countsByState[state] ?? 0;
      }
    }
    return totals;
  });

  protected readonly annotation = computed<{
    state: JobState;
    text: string;
  } | null>(() => {
    const state = this.selected();
    return state ? { state, text: LOCAL_COPY[state] } : null;
  });

  constructor() {
    effect(() => {
      this.#queues.connect(this.#connections.activeConnectionId());
    });
  }

  protected onSelect(state: JobState): void {
    this.selected.set(state);
  }
}
