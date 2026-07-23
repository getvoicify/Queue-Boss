import { TestBed } from "@angular/core/testing";
import { provideRouter, Router } from "@angular/router";
import { beforeEach, describe, expect, it } from "vitest";
import { axe } from "vitest-axe";
import { PrimaryNavComponent } from "./primary-nav.component";

describe("PrimaryNavComponent", () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PrimaryNavComponent],
      providers: [
        provideRouter([
          { path: "overview", children: [] },
          { path: "jobs", children: [] },
          { path: "lifecycle", children: [] },
          { path: "connect", children: [] },
        ]),
      ],
    }).compileComponents();
  });

  it("renders labelled links to Overview and Lifecycle", () => {
    const fixture = TestBed.createComponent(PrimaryNavComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement;
    const overview = el.querySelector('[data-testid="nav-overview"]');
    const lifecycle = el.querySelector('[data-testid="nav-lifecycle"]');
    expect(overview.textContent.trim()).toBe("Overview");
    expect(overview.getAttribute("href")).toBe("/overview");
    expect(lifecycle.textContent.trim()).toBe("Lifecycle");
    expect(lifecycle.getAttribute("href")).toBe("/lifecycle");
    const connect = el.querySelector('[data-testid="open-connect"]');
    expect(connect.textContent.trim()).toBe("Connect");
    expect(connect.getAttribute("href")).toBe("/connect");
    const jobs = el.querySelector('[data-testid="nav-jobs"]');
    expect(jobs.textContent.trim()).toBe("Jobs");
    expect(jobs.getAttribute("href")).toBe("/jobs");
  });

  it("marks the active link and leaves the inactive one unmarked", async () => {
    const fixture = TestBed.createComponent(PrimaryNavComponent);
    fixture.detectChanges();
    await TestBed.inject(Router).navigate(["/lifecycle"]);
    await fixture.whenStable();
    fixture.detectChanges();
    const el = fixture.nativeElement;
    const active = el.querySelector('[data-testid="nav-lifecycle"]');
    const inactive = el.querySelector('[data-testid="nav-overview"]');
    expect(active.getAttribute("aria-current")).toBe("page");
    expect(active.classList.contains("primary-nav__link--active")).toBe(true);
    expect(inactive.getAttribute("aria-current")).toBeNull();
    expect(inactive.classList.contains("primary-nav__link--active")).toBe(
      false,
    );
  });

  it("has no accessibility violations", async () => {
    const fixture = TestBed.createComponent(PrimaryNavComponent);
    fixture.detectChanges();
    expect(await axe(fixture.nativeElement)).toHaveNoViolations();
  });
});
