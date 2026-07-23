import { Injectable, inject, type Signal, signal } from "@angular/core";
import type { Capabilities, JobDetail, JobFilter, JobSummary } from "../models";
import { QueueBackendService } from "../tauri/queue-backend.service";
import { ConnectionsFacade } from "./connections.facade";

const DEFAULT_FILTER: JobFilter = { limit: 20 };

// Read-path facade for the job-list drill-down: owns the accumulated job page,
// the active filter, and the selected job detail plus its connection's
// capabilities. Every fetch is keyed on the facade's `activeConnectionId`.
@Injectable({ providedIn: "root" })
export class JobsFacade {
  readonly #backend = inject(QueueBackendService);
  readonly #connections = inject(ConnectionsFacade);

  readonly #jobs = signal<JobSummary[]>([]);
  readonly #filter = signal<JobFilter>(DEFAULT_FILTER);
  readonly #hasMore = signal(false);
  readonly #nextCursor = signal<string | null>(null);
  readonly #selected = signal<JobDetail | null>(null);
  readonly #capabilities = signal<Capabilities | null>(null);

  readonly jobs: Signal<JobSummary[]> = this.#jobs.asReadonly();
  readonly filter: Signal<JobFilter> = this.#filter.asReadonly();
  readonly hasMore: Signal<boolean> = this.#hasMore.asReadonly();
  readonly nextCursor: Signal<string | null> = this.#nextCursor.asReadonly();
  readonly selected: Signal<JobDetail | null> = this.#selected.asReadonly();
  readonly capabilities: Signal<Capabilities | null> =
    this.#capabilities.asReadonly();

  async loadPage(): Promise<void> {
    const connectionId = this.#connections.activeConnectionId();
    const cursor = this.#nextCursor() ?? undefined;
    const page = await this.#backend.listJobs(connectionId, {
      ...this.#filter(),
      cursor,
    });
    this.#jobs.update((prev) => [...prev, ...page.items]);
    this.#hasMore.set(page.hasMore);
    this.#nextCursor.set(page.nextCursor);
  }

  async setFilter(filter: JobFilter): Promise<void> {
    this.#filter.set(filter);
    this.#jobs.set([]);
    this.#nextCursor.set(null);
    this.#hasMore.set(false);
    this.#capabilities.set(null);
    this.#selected.set(null);
    await this.loadPage();
  }

  async select(id: string): Promise<void> {
    const connectionId = this.#connections.activeConnectionId();
    this.#selected.set(await this.#backend.getJob(connectionId, id));
    if (this.#capabilities() === null) {
      this.#capabilities.set(await this.#backend.capabilities(connectionId));
    }
  }
}
