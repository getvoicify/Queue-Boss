import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from "@angular/core";

export type ConnectionStatus = "idle" | "connecting" | "connected" | "error";

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
      data-testid="connection-status"
      [attr.data-status]="status()"
    >
      <span class="connection-status__dot" aria-hidden="true"></span>
      <span class="connection-status__text" role="status">
        <span class="connection-status__prefix">Connection:</span>
        <span class="connection-status__label" data-testid="connection-label">{{
          label()
        }}</span>
      </span>
    </div>
  `,
})
export class ConnectionStatusComponent {
  readonly status = input.required<ConnectionStatus>();
  protected readonly label = computed(() => STATUS_LABELS[this.status()]);
}
