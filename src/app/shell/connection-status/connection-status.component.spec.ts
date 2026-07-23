import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { ConnectionStatus } from "../../core/models";
import { ConnectionStatusComponent } from "./connection-status.component";

const STATUSES: readonly ConnectionStatus[] = [
  "idle",
  "connecting",
  "connected",
  "error",
];

const EXPECTED: Record<ConnectionStatus, string> = {
  idle: "Idle",
  connecting: "Connecting…",
  connected: "Connected",
  error: "Connection error",
};

function render(
  status: ConnectionStatus,
  connectionId = "sandbox",
  message?: string,
) {
  const fixture = TestBed.createComponent(ConnectionStatusComponent);
  fixture.componentRef.setInput("status", status);
  fixture.componentRef.setInput("connectionId", connectionId);
  if (message !== undefined) {
    fixture.componentRef.setInput("message", message);
  }
  fixture.detectChanges();
  return fixture;
}

describe("ConnectionStatusComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ConnectionStatusComponent],
    }).compileComponents();
  });

  it("keys the testid by connectionId and renders the exact label per status", () => {
    for (const status of STATUSES) {
      const root = render(status, "sandbox").nativeElement;
      expect(
        root
          .querySelector('[data-testid="connection-status-sandbox"]')
          .getAttribute("data-status"),
      ).toBe(status);
      expect(
        root
          .querySelector('[data-testid="connection-label"]')
          .textContent.trim(),
      ).toBe(EXPECTED[status]);
    }
  });

  it("renders the optional error message when provided", () => {
    const root = render(
      "error",
      "pgboss",
      "database is not reachable",
    ).nativeElement;
    expect(
      root
        .querySelector('[data-testid="connection-message"]')
        .textContent.trim(),
    ).toBe("database is not reachable");
  });

  it("has no accessibility violations", async () => {
    const el = render("connected", "sandbox").nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
