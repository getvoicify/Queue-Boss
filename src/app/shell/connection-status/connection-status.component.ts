import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from "@angular/core";
import type { ConnectionStatus } from "../../core/models";

const STATUS_LABELS: Record<ConnectionStatus, string> = {
  idle: "Idle",
  connecting: "Connecting…",
  connected: "Connected",
  error: "Connection error",
};

@Component({
  selector: "app-connection-status",
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: "./connection-status.component.css",
  template: `
    <div
      class="connection-status"
      [attr.data-testid]="'connection-status-' + connectionId()"
      [attr.data-status]="status()"
    >
      <span class="connection-status__dot" aria-hidden="true"></span>
      <span class="connection-status__text" role="status">
        <span class="connection-status__prefix">{{ connectionId() }}:</span>
        <span class="connection-status__label" data-testid="connection-label">{{
          label()
        }}</span>
        @if (message(); as detail) {
          <span
            class="connection-status__message"
            data-testid="connection-message"
            >{{ detail }}</span
          >
        }
      </span>
    </div>
  `,
})
export class ConnectionStatusComponent {
  readonly status = input.required<ConnectionStatus>();
  readonly connectionId = input.required<string>();
  readonly message = input<string>();
  protected readonly label = computed(() => STATUS_LABELS[this.status()]);
}
