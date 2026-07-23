import { ChangeDetectionStrategy, Component, input } from "@angular/core";
import { RouterOutlet } from "@angular/router";
import type { ConnectionEntry } from "../core/models";
import { ConnectionStatusComponent } from "./connection-status/connection-status.component";
import { PrimaryNavComponent } from "./primary-nav/primary-nav.component";

@Component({
  selector: "app-shell",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterOutlet, PrimaryNavComponent, ConnectionStatusComponent],
  styleUrl: "./shell.component.css",
  template: `
    <div class="app-shell" data-testid="app-shell">
      <header class="app-shell__header">
        <span class="app-shell__brand">Queue Boss</span>
        <app-primary-nav />
        <div class="app-shell__status">
          @for (connection of connections(); track connection.id) {
            <app-connection-status
              [connectionId]="connection.id"
              [status]="connection.status"
              [message]="connection.message"
            />
          }
        </div>
      </header>
      <main class="app-shell__main">
        <router-outlet />
      </main>
    </div>
  `,
})
export class ShellComponent {
  readonly connections = input<ConnectionEntry[]>([
    { id: "sandbox", status: "idle" },
  ]);
}
