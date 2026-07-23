import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from "@angular/core";
import {
  ConnectionsFacade,
  PENDING_CONNECTION_ID,
} from "../../core/facades/connections.facade";
import { ConnectFormComponent } from "./connect-form.component";

// Connect route container: wires the dumb connect form to the facade's
// `connect` intent and surfaces the pending connection's status/error.
@Component({
  selector: "app-connect-container",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ConnectFormComponent],
  template: `
    <section class="connect" aria-label="Connect to a database">
      <app-connect-form
        [connecting]="connecting()"
        (submit)="facade.connect($event)"
      />
      @if (pending(); as status) {
        <p
          class="connect__status"
          data-testid="connect-status"
          [attr.data-status]="status.status"
        >
          @if (status.message) {
            <span data-testid="connect-error">{{ status.message }}</span>
          }
        </p>
      }
    </section>
  `,
})
export class ConnectContainerComponent {
  protected readonly facade = inject(ConnectionsFacade);
  protected readonly pending = computed(() =>
    this.facade.statusFor(PENDING_CONNECTION_ID),
  );
  protected readonly connecting = computed(
    () => this.pending()?.status === "connecting",
  );
}
