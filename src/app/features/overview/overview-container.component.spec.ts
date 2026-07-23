import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ConnectionsFacade } from "../../core/facades/connections.facade";
import type { QueueCounts } from "../../core/models";
import { QueueBackendService } from "../../core/tauri/queue-backend.service";
import { OverviewContainerComponent } from "./overview-container.component";

const liveCounts: QueueCounts = {
  connectionId: "sandbox",
  polledAt: 1000,
  queues: [
    {
      queue: "emails",
      totalDepth: 3,
      countsByState: { active: 1, completed: 2 },
      oldestWaitingAge: 5,
    },
    {
      queue: "webhooks",
      totalDepth: 1,
      countsByState: { created: 1 },
      oldestWaitingAge: 2,
    },
  ],
};

describe("OverviewContainerComponent", () => {
  const source = signal<QueueCounts | null>(null);
  const stop = vi.fn();
  const subscribeCounts = vi.fn(() => ({ counts: source.asReadonly(), stop }));
  const active = signal("sandbox");

  beforeEach(async () => {
    source.set(null);
    active.set("sandbox");
    subscribeCounts.mockClear();
    await TestBed.configureTestingModule({
      imports: [OverviewContainerComponent],
      providers: [
        { provide: QueueBackendService, useValue: { subscribeCounts } },
        {
          provide: ConnectionsFacade,
          useValue: { activeConnectionId: active.asReadonly() },
        },
      ],
    }).compileComponents();
  });

  function render() {
    const fixture = TestBed.createComponent(OverviewContainerComponent);
    fixture.detectChanges();
    return fixture;
  }

  it("offers a real Enter Sandbox button and no queue rows before connecting", () => {
    const el = render().nativeElement as HTMLElement;
    const button = el.querySelector('[data-testid="enter-sandbox"]');
    expect(button).not.toBeNull();
    expect(button?.tagName).toBe("BUTTON");
    expect(button?.textContent?.trim()).toBe("Enter Sandbox");
    expect(el.querySelectorAll('[data-testid="queue-row"]').length).toBe(0);
  });

  it("streams live sandbox counts into the overview on Enter Sandbox", () => {
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;

    (
      el.querySelector('[data-testid="enter-sandbox"]') as HTMLButtonElement
    ).click();
    fixture.detectChanges();

    expect(subscribeCounts).toHaveBeenCalledWith("sandbox");
    expect(el.querySelector('[data-testid="enter-sandbox"]')).toBeNull();

    source.set(liveCounts);
    fixture.detectChanges();

    expect(el.querySelectorAll('[data-testid="queue-row"]').length).toBe(2);
    expect(
      el
        .querySelector('[data-testid="count-emails-completed"]')
        ?.textContent?.trim(),
    ).toBe("2");
    expect(
      el.querySelector('[data-testid="depth-emails"]')?.textContent?.trim(),
    ).toBe("3");
  });

  it("rekeys the overview to the active connection's counts without Enter", () => {
    const fixture = render();
    const el = fixture.nativeElement as HTMLElement;

    active.set("pgboss");
    fixture.detectChanges();

    expect(subscribeCounts).toHaveBeenCalledWith("pgboss");
    expect(el.querySelector('[data-testid="enter-sandbox"]')).toBeNull();

    source.set(liveCounts);
    fixture.detectChanges();

    expect(el.querySelectorAll('[data-testid="queue-row"]').length).toBe(2);
  });
});
