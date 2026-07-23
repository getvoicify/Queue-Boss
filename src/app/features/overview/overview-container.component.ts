import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
} from "@angular/core";
import {
  ConnectionsFacade,
  SANDBOX_CONNECTION_ID,
} from "../../core/facades/connections.facade";
import { QueuesFacade } from "../../core/facades/queues.facade";
import { OverviewComponent } from "./overview.component";

// Overview route container: rekeys the queues facade off the facade's
// `activeConnectionId`. The sandbox still requires an explicit Enter click; a
// real connection auto-subscribes when it becomes active.
@Component({
  selector: "app-overview-container",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [OverviewComponent],
  styleUrl: "./overview-container.component.css",
  template: `
    @if (showEnter()) {
      <div class="overview-entry">
        <button
          type="button"
          class="overview-entry__button"
          data-testid="enter-sandbox"
          (click)="enter()"
        >
          Enter Sandbox
        </button>
      </div>
    } @else {
      <app-overview [queues]="queues.queues()" />
    }
  `,
})
export class OverviewContainerComponent {
  protected readonly queues = inject(QueuesFacade);
  readonly #connections = inject(ConnectionsFacade);
  readonly #active = this.#connections.activeConnectionId;
  readonly #entered = signal(false);
  protected readonly showEnter = computed(
    () => this.#active() === SANDBOX_CONNECTION_ID && !this.#entered(),
  );

  constructor() {
    effect(() => {
      const id = this.#active();
      if (id !== SANDBOX_CONNECTION_ID) {
        this.queues.connect(id);
      }
    });
  }

  enter(): void {
    this.queues.connect(SANDBOX_CONNECTION_ID);
    this.#entered.set(true);
  }
}
