import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from "@angular/core";
import { ConnectionFacade } from "../../core/facades/connection.facade";
import { QueuesFacade } from "../../core/facades/queues.facade";
import { OverviewComponent } from "./overview.component";

const SANDBOX_CONNECTION_ID = "sandbox";

// Overview route container: binds the connection/queues facades to the dumb
// `app-overview`. All logic stays in the facades — this only wires intents.
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
  readonly #connection = inject(ConnectionFacade);
  protected readonly showEnter = computed(() => {
    const status = this.#connection.status();
    return status === "idle" || status === "error";
  });

  enter(): void {
    void this.#connection.connect(SANDBOX_CONNECTION_ID);
    this.queues.connect(SANDBOX_CONNECTION_ID);
  }
}
