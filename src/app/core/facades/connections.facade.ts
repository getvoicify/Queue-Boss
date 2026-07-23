import {
  computed,
  Injectable,
  inject,
  type Signal,
  signal,
} from "@angular/core";
import type {
  CommandError,
  ConnectionEntry,
  ConnectionStatus,
  PgConnectConfig,
} from "../models";
import { QueueBackendService } from "../tauri/queue-backend.service";

export const SANDBOX_CONNECTION_ID = "sandbox";
export const PENDING_CONNECTION_ID = "pgboss";

interface StatusValue {
  status: ConnectionStatus;
  message?: string;
}
type StatusMap = Record<string, StatusValue>;

// Owns per-connection status (keyed by `connectionId`) plus the
// `activeConnectionId` selection that the overview/lifecycle rekey off. The
// sandbox is seeded connected and can never be disconnected.
@Injectable({ providedIn: "root" })
export class ConnectionsFacade {
  readonly #backend = inject(QueueBackendService);
  readonly #status = signal<StatusMap>({
    [SANDBOX_CONNECTION_ID]: { status: "connected" },
  });
  readonly #active = signal<string>(SANDBOX_CONNECTION_ID);

  readonly activeConnectionId: Signal<string> = this.#active.asReadonly();
  readonly entries: Signal<ConnectionEntry[]> = computed(() =>
    Object.entries(this.#status()).map(([id, value]) => ({
      id,
      status: value.status,
      message: value.message,
    })),
  );

  statusFor(id: string): StatusValue | undefined {
    return this.#status()[id];
  }

  async connect(config: PgConnectConfig): Promise<void> {
    this.#setStatus(PENDING_CONNECTION_ID, { status: "connecting" });
    try {
      const id = await this.#backend.connectPgboss(config);
      this.#status.update((map) => {
        const next = { ...map };
        delete next[PENDING_CONNECTION_ID];
        next[id] = { status: "connected" };
        return next;
      });
      this.#active.set(id);
    } catch (error) {
      this.#setStatus(PENDING_CONNECTION_ID, {
        status: "error",
        message: messageOf(error),
      });
    }
  }

  async disconnect(id: string): Promise<void> {
    if (id === SANDBOX_CONNECTION_ID) {
      const error: CommandError = {
        kind: "unsupported",
        message: "The sandbox connection cannot be disconnected.",
      };
      throw error;
    }
    await this.#backend.disconnect(id);
    this.#status.update((map) => {
      const next = { ...map };
      delete next[id];
      return next;
    });
    this.#active.set(SANDBOX_CONNECTION_ID);
  }

  #setStatus(id: string, value: StatusValue): void {
    this.#status.update((map) => ({ ...map, [id]: value }));
  }
}

function messageOf(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return String(error);
}
