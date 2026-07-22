import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { describe, expect, it, vi } from "vitest";
import type { QueueCounts } from "../models";
import { QueueBackendService } from "../tauri/queue-backend.service";
import { QueuesFacade } from "./queues.facade";

const sampleCounts: QueueCounts = {
  connectionId: "sandbox",
  polledAt: 2000,
  queues: [
    {
      queue: "emails",
      totalDepth: 4,
      countsByState: { created: 4 },
      oldestWaitingAge: 3,
    },
  ],
};

describe("QueuesFacade", () => {
  it("derives the queues signal from counts streamed after connect", () => {
    const source = signal<QueueCounts | null>(null);
    const stop = vi.fn();
    const subscribeCounts = vi.fn(() => ({
      counts: source.asReadonly(),
      stop,
    }));
    TestBed.configureTestingModule({
      providers: [
        { provide: QueueBackendService, useValue: { subscribeCounts } },
      ],
    });
    const facade = TestBed.inject(QueuesFacade);

    expect(facade.queues()).toEqual([]);

    facade.connect("sandbox");
    expect(subscribeCounts).toHaveBeenCalledWith("sandbox");
    expect(facade.queues()).toEqual([]);

    source.set(sampleCounts);
    expect(facade.queues()).toEqual(sampleCounts.queues);
  });
});
