import { TestBed } from "@angular/core/testing";
import { describe, expect, it, vi } from "vitest";
import { QueueBackendService } from "../tauri/queue-backend.service";
import { ConnectionFacade } from "./connection.facade";

describe("ConnectionFacade", () => {
  it("sets status to connected when testConnection resolves", async () => {
    const testConnection = vi
      .fn()
      .mockResolvedValue({ name: "sandbox", healthy: true });
    TestBed.configureTestingModule({
      providers: [
        { provide: QueueBackendService, useValue: { testConnection } },
      ],
    });
    const facade = TestBed.inject(ConnectionFacade);

    expect(facade.status()).toBe("idle");

    await facade.connect("sandbox");

    expect(testConnection).toHaveBeenCalledWith("sandbox");
    expect(facade.status()).toBe("connected");
  });

  it("sets status to error when testConnection rejects", async () => {
    const testConnection = vi
      .fn()
      .mockRejectedValue({ kind: "connection", message: "x" });
    TestBed.configureTestingModule({
      providers: [
        { provide: QueueBackendService, useValue: { testConnection } },
      ],
    });
    const facade = TestBed.inject(ConnectionFacade);

    await facade.connect("sandbox");

    expect(facade.status()).toBe("error");
  });
});
