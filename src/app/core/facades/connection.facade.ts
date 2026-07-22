import { Injectable, inject, type Signal, signal } from "@angular/core";
import { QueueBackendService } from "../tauri/queue-backend.service";

export type ConnectionStatus = "idle" | "connecting" | "connected" | "error";

// Owns connection status as a read-only signal, driven by a `connect` intent
// that probes the backend via `testConnection`.
@Injectable({ providedIn: "root" })
export class ConnectionFacade {
  readonly #backend = inject(QueueBackendService);
  readonly #status = signal<ConnectionStatus>("idle");
  readonly status: Signal<ConnectionStatus> = this.#status.asReadonly();

  async connect(connectionId: string): Promise<void> {
    this.#status.set("connecting");
    try {
      await this.#backend.testConnection(connectionId);
      this.#status.set("connected");
    } catch {
      this.#status.set("error");
    }
  }
}
