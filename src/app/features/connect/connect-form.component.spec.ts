import { readFileSync } from "node:fs";
import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import type { PgConnectConfig } from "../../core/models";
import { ConnectFormComponent } from "./connect-form.component";

function type(root: HTMLElement, id: string, value: string): void {
  const field = root.querySelector(`#${id}`) as HTMLInputElement;
  field.value = value;
  field.dispatchEvent(new Event("input"));
}

function submitForm(root: HTMLElement): void {
  root
    .querySelector('[data-testid="connect-form"]')
    ?.dispatchEvent(new Event("submit", { cancelable: true, bubbles: true }));
}

function discrete() {
  const fixture = TestBed.createComponent(ConnectFormComponent);
  fixture.componentRef.setInput("mode", "discrete");
  fixture.detectChanges();
  return fixture;
}

describe("ConnectFormComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ConnectFormComponent],
    }).compileComponents();
  });

  it("renders the connection-string field by default and the discrete fields after toggling", () => {
    const fixture = TestBed.createComponent(ConnectFormComponent);
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;

    expect(root.querySelector('[data-testid="connect-form"]')).not.toBeNull();
    expect(root.querySelector("#cf-connection-string")).not.toBeNull();
    expect(root.querySelector("#cf-host")).toBeNull();

    (
      root.querySelector(
        '[data-testid="connect-mode-toggle"]',
      ) as HTMLButtonElement
    ).click();
    fixture.detectChanges();

    expect(root.querySelector("#cf-connection-string")).toBeNull();
    expect(root.querySelector("#cf-host")).not.toBeNull();
    expect(root.querySelector("#cf-password")).not.toBeNull();
  });

  it("honours the mode input by rendering the discrete fields", () => {
    const root = discrete().nativeElement as HTMLElement;
    expect(root.querySelector("#cf-host")).not.toBeNull();
    expect(root.querySelector("#cf-connection-string")).toBeNull();
  });

  it("emits a connection-string config on submit", () => {
    const fixture = TestBed.createComponent(ConnectFormComponent);
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;
    const emitted: PgConnectConfig[] = [];
    fixture.componentInstance.submit.subscribe((c) => emitted.push(c));

    type(root, "cf-connection-string", "postgres://localhost/pgboss");
    fixture.detectChanges();
    submitForm(root);

    expect(emitted).toEqual([
      { connectionString: "postgres://localhost/pgboss" },
    ]);
  });

  it("emits a discrete config including the optional schema on submit", () => {
    const fixture = discrete();
    const root = fixture.nativeElement as HTMLElement;
    const emitted: PgConnectConfig[] = [];
    fixture.componentInstance.submit.subscribe((c) => emitted.push(c));

    type(root, "cf-host", "db.example.com");
    type(root, "cf-port", "6432");
    type(root, "cf-database", "app");
    type(root, "cf-user", "reader");
    type(root, "cf-password", "s3cret");
    type(root, "cf-schema", "pgboss");
    fixture.detectChanges();
    submitForm(root);

    expect(emitted).toEqual([
      {
        host: "db.example.com",
        port: 6432,
        database: "app",
        user: "reader",
        password: "s3cret",
        sslMode: "prefer",
        schema: "pgboss",
      },
    ]);
  });

  it("keeps submit disabled while invalid and while connecting", () => {
    const fixture = TestBed.createComponent(ConnectFormComponent);
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;
    const submit = () =>
      root.querySelector('[data-testid="connect-submit"]') as HTMLButtonElement;

    expect(submit().disabled).toBe(true);

    type(root, "cf-connection-string", "postgres://x");
    fixture.detectChanges();
    expect(submit().disabled).toBe(false);

    fixture.componentRef.setInput("connecting", true);
    fixture.detectChanges();
    expect(submit().disabled).toBe(true);
  });

  it("renders a labelled password field of type password", () => {
    const root = discrete().nativeElement as HTMLElement;
    const pw = root.querySelector("#cf-password") as HTMLInputElement;
    expect(pw.type).toBe("password");
    const label = root.querySelector('label[for="cf-password"]');
    expect(label?.textContent?.trim()).toBe("Password");
  });

  it("has no accessibility violations in discrete mode", async () => {
    const fixture = discrete();
    expect(await axe(fixture.nativeElement)).toHaveNoViolations();
  });

  it("has no accessibility violations in connection-string mode", async () => {
    const fixture = TestBed.createComponent(ConnectFormComponent);
    fixture.componentRef.setInput("mode", "connectionString");
    fixture.detectChanges();
    expect(await axe(fixture.nativeElement)).toHaveNoViolations();
  });

  it("stays dumb: never references the interface service or Tauri", () => {
    const src = readFileSync(
      "src/app/features/connect/connect-form.component.ts",
      "utf8",
    );
    expect(src).not.toContain("QueueBackendService");
    expect(src).not.toContain("@tauri-apps/api");
  });
});
