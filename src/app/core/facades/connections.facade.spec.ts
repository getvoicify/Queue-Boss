import { TestBed } from "@angular/core/testing";
import { describe, expect, it, vi } from "vitest";
import type { PgConnectConfig } from "../models";
import { QueueBackendService } from "../tauri/queue-backend.service";
import { ConnectionsFacade, SANDBOX_CONNECTION_ID } from "./connections.facade";

const config: PgConnectConfig = {
  connectionString: "postgres://localhost/pgboss",
};

function configure(backend: Partial<QueueBackendService>): ConnectionsFacade {
  TestBed.configureTestingModule({
    providers: [{ provide: QueueBackendService, useValue: backend }],
  });
  return TestBed.inject(ConnectionsFacade);
}

describe("ConnectionsFacade", () => {
  it("seeds a connected sandbox entry and an active sandbox id", () => {
    const facade = configure({});
    expect(facade.activeConnectionId()).toBe(SANDBOX_CONNECTION_ID);
    expect(facade.statusFor(SANDBOX_CONNECTION_ID)).toEqual({
      status: "connected",
    });
    expect(facade.entries()).toContainEqual({
      id: SANDBOX_CONNECTION_ID,
      status: "connected",
      message: undefined,
    });
  });

  it("transitions the pending id connecting → connected and rekeys the active id on success", async () => {
    const connectPgboss = vi.fn().mockResolvedValue("pgboss");
    const facade = configure({ connectPgboss });

    const pending = facade.connect(config);
    expect(facade.statusFor("pgboss")).toEqual({ status: "connecting" });

    await pending;
    expect(connectPgboss).toHaveBeenCalledWith(config);
    expect(facade.statusFor("pgboss")).toEqual({ status: "connected" });
    expect(facade.activeConnectionId()).toBe("pgboss");
  });

  it("marks the pending id errored with the sanitized message and keeps sandbox active", async () => {
    const connectPgboss = vi.fn().mockRejectedValue({
      kind: "connection",
      message: "database is not reachable",
    });
    const facade = configure({ connectPgboss });

    await facade.connect(config);

    expect(facade.statusFor("pgboss")).toEqual({
      status: "error",
      message: "database is not reachable",
    });
    expect(facade.activeConnectionId()).toBe(SANDBOX_CONNECTION_ID);
  });

  it("disconnect drops the entry and resets the active id to sandbox", async () => {
    const connectPgboss = vi.fn().mockResolvedValue("pgboss");
    const disconnect = vi.fn().mockResolvedValue(undefined);
    const facade = configure({ connectPgboss, disconnect });

    await facade.connect(config);
    await facade.disconnect("pgboss");

    expect(disconnect).toHaveBeenCalledWith("pgboss");
    expect(facade.statusFor("pgboss")).toBeUndefined();
    expect(facade.activeConnectionId()).toBe(SANDBOX_CONNECTION_ID);
  });

  it("refuses to disconnect the sandbox and never calls the backend", async () => {
    const disconnect = vi.fn();
    const facade = configure({ disconnect });

    await expect(
      facade.disconnect(SANDBOX_CONNECTION_ID),
    ).rejects.toMatchObject({ kind: "unsupported" });
    expect(disconnect).not.toHaveBeenCalled();
    expect(facade.statusFor(SANDBOX_CONNECTION_ID)).toEqual({
      status: "connected",
    });
  });
});
