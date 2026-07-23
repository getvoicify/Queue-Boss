import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  linkedSignal,
  output,
  signal,
} from "@angular/core";
import type { PgConnectConfig } from "../../core/models";

export type ConnectMode = "connectionString" | "discrete";

const SSL_MODES = ["disable", "prefer", "require", "verify-full"] as const;

// Dumb connect form: inputs/outputs only, no service or Tauri access. Emits a
// `PgConnectConfig` for whichever entry mode is active.
@Component({
  selector: "app-connect-form",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <form
      class="connect-form"
      data-testid="connect-form"
      aria-label="Connect to a pg-boss database"
      (submit)="onSubmit($event)"
    >
      <button
        type="button"
        class="connect-form__toggle"
        data-testid="connect-mode-toggle"
        (click)="toggleMode()"
      >
        {{
          activeMode() === "connectionString"
            ? "Use discrete fields"
            : "Use connection string"
        }}
      </button>

      @if (activeMode() === "connectionString") {
        <div class="connect-form__field">
          <label for="cf-connection-string">Connection string</label>
          <input
            id="cf-connection-string"
            name="connectionString"
            type="text"
            autocomplete="off"
            [value]="connectionString()"
            (input)="connectionString.set(read($event))"
          />
        </div>
      } @else {
        <div class="connect-form__field">
          <label for="cf-host">Host</label>
          <input
            id="cf-host"
            name="host"
            type="text"
            [value]="host()"
            (input)="host.set(read($event))"
          />
        </div>
        <div class="connect-form__field">
          <label for="cf-port">Port</label>
          <input
            id="cf-port"
            name="port"
            type="number"
            [value]="port()"
            (input)="port.set(read($event))"
          />
        </div>
        <div class="connect-form__field">
          <label for="cf-database">Database</label>
          <input
            id="cf-database"
            name="database"
            type="text"
            [value]="database()"
            (input)="database.set(read($event))"
          />
        </div>
        <div class="connect-form__field">
          <label for="cf-user">User</label>
          <input
            id="cf-user"
            name="user"
            type="text"
            autocomplete="username"
            [value]="user()"
            (input)="user.set(read($event))"
          />
        </div>
        <div class="connect-form__field">
          <label for="cf-password">Password</label>
          <input
            id="cf-password"
            name="password"
            type="password"
            autocomplete="current-password"
            [value]="password()"
            (input)="password.set(read($event))"
          />
        </div>
        <div class="connect-form__field">
          <label for="cf-ssl-mode">SSL mode</label>
          <select
            id="cf-ssl-mode"
            name="sslMode"
            [value]="sslMode()"
            (change)="sslMode.set(read($event))"
          >
            @for (mode of sslModes; track mode) {
              <option [value]="mode">{{ mode }}</option>
            }
          </select>
        </div>
        <div class="connect-form__field">
          <label for="cf-schema">Schema (optional)</label>
          <input
            id="cf-schema"
            name="schema"
            type="text"
            [value]="schema()"
            (input)="schema.set(read($event))"
          />
        </div>
      }

      <button
        type="submit"
        class="connect-form__submit"
        data-testid="connect-submit"
        [disabled]="connecting() || !valid()"
      >
        {{ connecting() ? "Connecting…" : "Connect" }}
      </button>
    </form>
  `,
})
export class ConnectFormComponent {
  readonly mode = input<ConnectMode>("connectionString");
  readonly connecting = input(false);
  readonly submit = output<PgConnectConfig>();

  protected readonly activeMode = linkedSignal(() => this.mode());
  protected readonly sslModes = SSL_MODES;

  protected readonly connectionString = signal("");
  protected readonly host = signal("");
  protected readonly port = signal("5432");
  protected readonly database = signal("");
  protected readonly user = signal("");
  protected readonly password = signal("");
  protected readonly sslMode = signal<string>("prefer");
  protected readonly schema = signal("");

  protected readonly valid = computed(() => {
    if (this.activeMode() === "connectionString") {
      return this.connectionString().trim().length > 0;
    }
    const port = Number(this.port());
    return (
      this.host().trim().length > 0 &&
      Number.isInteger(port) &&
      port > 0 &&
      this.database().trim().length > 0 &&
      this.user().trim().length > 0 &&
      this.password().length > 0 &&
      this.sslMode().trim().length > 0
    );
  });

  protected toggleMode(): void {
    this.activeMode.update((mode) =>
      mode === "connectionString" ? "discrete" : "connectionString",
    );
  }

  protected read(event: Event): string {
    return (event.target as HTMLInputElement | HTMLSelectElement).value;
  }

  protected onSubmit(event: Event): void {
    event.preventDefault();
    if (this.connecting() || !this.valid()) {
      return;
    }
    this.submit.emit(this.buildConfig());
  }

  private buildConfig(): PgConnectConfig {
    if (this.activeMode() === "connectionString") {
      return { connectionString: this.connectionString().trim() };
    }
    const base = {
      host: this.host().trim(),
      port: Number(this.port()),
      database: this.database().trim(),
      user: this.user().trim(),
      password: this.password(),
      sslMode: this.sslMode(),
    };
    const schema = this.schema().trim();
    return schema ? { ...base, schema } : base;
  }
}
