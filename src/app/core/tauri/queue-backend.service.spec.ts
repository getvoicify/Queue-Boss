import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { QueueCounts } from "../models";
import { QueueBackendService } from "./queue-backend.service";

interface RawChannelMessage {
  message: QueueCounts;
  index: number;
}

interface TauriTestGlobal {
  __TAURI_INTERNALS__?: {
    invoke: ReturnType<typeof vi.fn>;
    transformCallback: (
      cb: (raw: RawChannelMessage) => void,
      once?: boolean,
    ) => number;
  };
}

const sampleCounts: QueueCounts = {
  connectionId: "sandbox",
  polledAt: 1000,
  queues: [
    {
      queue: "emails",
      totalDepth: 6,
      countsByState: { created: 3, active: 2, deadLetter: 1 },
      oldestWaitingAge: 9,
    },
  ],
};

describe("QueueBackendService", () => {
  let invokeMock: ReturnType<typeof vi.fn>;
  let captured: ((raw: RawChannelMessage) => void) | undefined;
  let service: QueueBackendService;

  // The real `invoke` forwards a third `options` arg, so assert only the
  // command name and the camelCase argument object it was called with.
  function lastInvoke(): { command: unknown; args: unknown } {
    const call = invokeMock.mock.calls.at(-1);
    return { command: call?.[0], args: call?.[1] };
  }

  beforeEach(() => {
    captured = undefined;
    invokeMock = vi.fn().mockResolvedValue(undefined);
    const testGlobal = window as unknown as TauriTestGlobal;
    testGlobal.__TAURI_INTERNALS__ = {
      invoke: invokeMock,
      transformCallback: (cb) => {
        captured = cb;
        return 42;
      },
    };
    service = TestBed.inject(QueueBackendService);
  });

  it("invokes test_connection with the camelCase connectionId", async () => {
    invokeMock.mockResolvedValueOnce({ name: "sandbox", healthy: true });
    await service.testConnection("sandbox");
    expect(lastInvoke()).toEqual({
      command: "test_connection",
      args: { connectionId: "sandbox" },
    });
  });

  it("invokes list_queues with connectionId and maps the resolved payload", async () => {
    const queues = [
      {
        name: "emails",
        totalDepth: 6,
        countsByState: { created: 3 },
        oldestWaitingAge: 9,
      },
    ];
    invokeMock.mockResolvedValueOnce(queues);
    const result = await service.listQueues("sandbox");
    expect(lastInvoke()).toEqual({
      command: "list_queues",
      args: { connectionId: "sandbox" },
    });
    expect(result).toEqual(queues);
  });

  it("invokes list_jobs with connectionId and filter", async () => {
    const page = { items: [], hasMore: false };
    invokeMock.mockResolvedValueOnce(page);
    const filter = { limit: 20 };
    const result = await service.listJobs("sandbox", filter);
    expect(lastInvoke()).toEqual({
      command: "list_jobs",
      args: { connectionId: "sandbox", filter },
    });
    expect(result).toEqual(page);
  });

  it("invokes get_job with connectionId and id", async () => {
    invokeMock.mockResolvedValueOnce({ id: "job-1" });
    await service.getJob("sandbox", "job-1");
    expect(lastInvoke()).toEqual({
      command: "get_job",
      args: { connectionId: "sandbox", id: "job-1" },
    });
  });

  it("wires a Channel for subscribe_counts and surfaces pushes on a read-only signal", () => {
    const subscription = service.subscribeCounts("sandbox");

    expect(subscription.counts()).toBeNull();
    const { command, args } = lastInvoke();
    expect(command).toBe("subscribe_counts");
    const invokeArgs = args as { connectionId?: unknown; channel?: unknown };
    expect(invokeArgs.connectionId).toBe("sandbox");
    // The Channel must actually be handed to invoke, not just constructed.
    expect(invokeArgs.channel).toBeDefined();
    expect(captured).toBeTypeOf("function");

    captured?.({ message: sampleCounts, index: 0 });

    expect(subscription.counts()).toEqual(sampleCounts);
  });

  it("normalizes a typed CommandError rejection unchanged", async () => {
    invokeMock.mockRejectedValueOnce({
      kind: "connection",
      message: "connection failed",
    });
    await expect(service.listQueues("sandbox")).rejects.toEqual({
      kind: "connection",
      message: "connection failed",
    });
  });

  it("normalizes an unknown rejection to an internal CommandError", async () => {
    invokeMock.mockRejectedValueOnce("boom");
    await expect(service.listQueues("sandbox")).rejects.toEqual({
      kind: "internal",
      message: "boom",
    });
  });
});
