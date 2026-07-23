import { Injectable, type Signal, signal } from "@angular/core";
import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  BackendInfo,
  CommandError,
  CommandErrorKind,
  JobDetail,
  JobFilter,
  JobSummary,
  Page,
  PgConnectConfig,
  QueueCounts,
  QueueSummary,
} from "../models";

export interface CountsSubscription {
  counts: Signal<QueueCounts | null>;
  stop(): void;
}

// The sole Tauri touchpoint: wraps the `invoke` command contract + counts
// `Channel` stream, normalizing every rejection to a typed `CommandError`.
@Injectable({ providedIn: "root" })
export class QueueBackendService {
  testConnection(connectionId: string): Promise<BackendInfo> {
    return this.#invoke<BackendInfo>("test_connection", { connectionId });
  }

  connectPgboss(config: PgConnectConfig): Promise<string> {
    return this.#invoke<string>("connect_pgboss", { config });
  }

  disconnect(connectionId: string): Promise<void> {
    return this.#invoke<void>("disconnect", { connectionId });
  }

  listQueues(connectionId: string): Promise<QueueSummary[]> {
    return this.#invoke<QueueSummary[]>("list_queues", { connectionId });
  }

  listJobs(connectionId: string, filter: JobFilter): Promise<Page<JobSummary>> {
    return this.#invoke<Page<JobSummary>>("list_jobs", {
      connectionId,
      filter,
    });
  }

  getJob(connectionId: string, id: string): Promise<JobDetail> {
    return this.#invoke<JobDetail>("get_job", { connectionId, id });
  }

  subscribeCounts(connectionId: string): CountsSubscription {
    const snapshot = signal<QueueCounts | null>(null);
    const channel = new Channel<QueueCounts>();
    channel.onmessage = (counts) => snapshot.set(counts);
    // Fire-and-forget: a failed subscribe leaves the signal null.
    void this.#invoke<null>("subscribe_counts", {
      connectionId,
      channel,
    }).catch(() => undefined);
    return {
      counts: snapshot.asReadonly(),
      stop: () => {
        channel.onmessage = () => undefined;
      },
    };
  }

  async #invoke<T>(command: string, args: Record<string, unknown>): Promise<T> {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      throw toCommandError(error);
    }
  }
}

const ERROR_KINDS: readonly CommandErrorKind[] = [
  "connection",
  "unsupported",
  "notFound",
  "internal",
];

function toCommandError(error: unknown): CommandError {
  if (isCommandError(error)) {
    return error;
  }
  return { kind: "internal", message: String(error) };
}

function isCommandError(value: unknown): value is CommandError {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const { kind, message } = value as { kind?: unknown; message?: unknown };
  return (
    typeof message === "string" &&
    typeof kind === "string" &&
    (ERROR_KINDS as readonly string[]).includes(kind)
  );
}
