import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import {
  type ConnectionStatus,
  ConnectionStatusComponent,
} from "./connection-status.component";

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

function render(status: ConnectionStatus) {
  const fixture = TestBed.createComponent(ConnectionStatusComponent);
  fixture.componentRef.setInput("status", status);
  fixture.detectChanges();
  return fixture;
}

describe("ConnectionStatusComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ConnectionStatusComponent],
    }).compileComponents();
  });

  it("renders the exact label for each status", () => {
    for (const status of STATUSES) {
      const root = render(status).nativeElement;
      expect(
        root
          .querySelector('[data-testid="connection-status"]')
          .getAttribute("data-status"),
      ).toBe(status);
      expect(
        root
          .querySelector('[data-testid="connection-label"]')
          .textContent.trim(),
      ).toBe(EXPECTED[status]);
    }
  });

  it("has no accessibility violations", async () => {
    const el = render("connected").nativeElement as HTMLElement;
    expect(await axe(el)).toHaveNoViolations();
  });
});
