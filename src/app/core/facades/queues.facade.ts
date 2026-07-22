import {
  computed,
  Injectable,
  inject,
  type Signal,
  signal,
} from "@angular/core";
import type { QueueCountEntry, QueueCounts } from "../models";
import { QueueBackendService } from "../tauri/queue-backend.service";

// Exposes the derived, read-only `queues` signal plus a `connect` intent;
// no component touches the interface service directly.
@Injectable({ providedIn: "root" })
export class QueuesFacade {
  readonly #backend = inject(QueueBackendService);
  readonly #counts = signal<Signal<QueueCounts | null>>(
    signal<QueueCounts | null>(null).asReadonly(),
  );
  readonly queues: Signal<QueueCountEntry[]> = computed(
    () => this.#counts()()?.queues ?? [],
  );
  #stop: (() => void) | undefined;

  connect(connectionId: string): void {
    this.#stop?.();
    const { counts, stop } = this.#backend.subscribeCounts(connectionId);
    this.#stop = stop;
    this.#counts.set(counts);
  }
}
