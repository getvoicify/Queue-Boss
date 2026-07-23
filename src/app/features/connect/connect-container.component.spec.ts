import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ConnectionsFacade } from "../../core/facades/connections.facade";
import { ConnectContainerComponent } from "./connect-container.component";

describe("ConnectContainerComponent", () => {
  const connect = vi.fn();
  let status: { status: string; message?: string } | undefined;

  beforeEach(async () => {
    connect.mockClear();
    status = undefined;
    await TestBed.configureTestingModule({
      imports: [ConnectContainerComponent],
      providers: [
        {
          provide: ConnectionsFacade,
          useValue: { connect, statusFor: () => status },
        },
      ],
    }).compileComponents();
  });

  it("renders the dumb connect form", () => {
    const fixture = TestBed.createComponent(ConnectContainerComponent);
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="connect-form"]'),
    ).not.toBeNull();
  });

  it("forwards a submitted config to the facade connect intent", () => {
    const fixture = TestBed.createComponent(ConnectContainerComponent);
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;

    const cs = root.querySelector("#cf-connection-string") as HTMLInputElement;
    cs.value = "postgres://localhost/pgboss";
    cs.dispatchEvent(new Event("input"));
    fixture.detectChanges();
    root
      .querySelector('[data-testid="connect-form"]')
      ?.dispatchEvent(new Event("submit", { cancelable: true }));

    expect(connect).toHaveBeenCalledWith({
      connectionString: "postgres://localhost/pgboss",
    });
  });

  it("surfaces the pending connection's error message", () => {
    status = { status: "error", message: "database is not reachable" };
    const fixture = TestBed.createComponent(ConnectContainerComponent);
    fixture.detectChanges();
    expect(
      fixture.nativeElement
        .querySelector('[data-testid="connect-error"]')
        .textContent.trim(),
    ).toBe("database is not reachable");
  });
});
