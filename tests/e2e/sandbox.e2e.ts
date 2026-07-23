// Live-update e2e (extends C1's launch-smoke): boot -> Enter Sandbox -> queue
// rows -> a per-state count changes across a poll.
// Runs on C2's Linux e2e job (tauri-driver + xvfb); tauri-driver is Linux/Windows only.
//
// Timeouts are generous: the external tauri-driver provider has no embedded
// @wdio/tauri-plugin, so the service's per-command window-focus hook adds ~5s to
// each focus command ($/$$/click) — the mocha cap is 180s.
//
// Color-contrast a11y is NOT asserted here: real-webview axe is blocked by the
// tauri-driver single-webview / window-handle model (raw inject leaves
// window.axe undefined; @axe-core/webdriverio closes the session), and jsdom
// (C7 vitest-axe) cannot run contrast rules. C7's jsdom axe already covers
// structure/labels/keyboard; color-contrast is verified by the documented
// manual dark-theme contrast check recorded in the PR body.
describe("Queue Boss sandbox live update", () => {
  it("opens the connect form while the sandbox status chip stays visible", async () => {
    const open = await $('[data-testid="open-connect"]');
    await open.waitForClickable({ timeout: 30000 });
    await open.click();

    const form = await $('[data-testid="connect-form"]');
    await form.waitForDisplayed({ timeout: 30000 });

    // The connect form and the always-present sandbox status chip coexist —
    // opening connect does not tear down the sandbox connection.
    const sandboxChip = await $('[data-testid="connection-status-sandbox"]');
    await sandboxChip.waitForDisplayed({ timeout: 30000 });

    // Return to the overview so the live-counts spec below starts there.
    const overview = await $('[data-testid="nav-overview"]');
    await overview.click();
    const enter = await $('[data-testid="enter-sandbox"]');
    await enter.waitForDisplayed({ timeout: 30000 });
  });

  it("drills from the jobs list into a job's detail with no extension rows in the sandbox", async () => {
    const jobsNav = await $('[data-testid="nav-jobs"]');
    await jobsNav.waitForClickable({ timeout: 30000 });
    await jobsNav.click();

    // The sandbox serves jobs, so the list renders at least one row.
    await browser.waitUntil(
      async () => (await $$('[data-testid="job-row"]')).length > 0,
      {
        timeout: 30000,
        interval: 250,
        timeoutMsg: "job rows never rendered on the jobs screen",
      },
    );

    const firstRow = await $('[data-testid="job-row"]');
    await firstRow.waitForClickable({ timeout: 30000 });
    await firstRow.click();

    // The detail panel renders only once the job AND its capabilities resolve.
    const detail = await $('[data-testid="job-detail"]');
    await detail.waitForDisplayed({ timeout: 30000 });

    // The sandbox advertises no extensions, so no capability-gated rows appear.
    const extensionRows = await $$('[data-testid^="job-extension-"]');
    expect(extensionRows).toHaveLength(0);

    // Return to the overview so the live-counts spec below starts there.
    const overview = await $('[data-testid="nav-overview"]');
    await overview.click();
    const enter = await $('[data-testid="enter-sandbox"]');
    await enter.waitForDisplayed({ timeout: 30000 });
  });

  it("enters the sandbox and streams live-updating queue counts", async () => {
    const enter = await $('[data-testid="enter-sandbox"]');
    await enter.waitForClickable({ timeout: 30000 });
    await enter.click();

    await browser.waitUntil(
      async () => (await $$('[data-testid="queue-row"]')).length > 0,
      {
        timeout: 30000,
        interval: 250,
        timeoutMsg: "queue rows never rendered after entering the sandbox",
      },
    );

    // A continuous-queue per-state count keeps moving poll-to-poll; the bare
    // depth total saturates once the visible window fills (~25.6s), so assert
    // on a completed count instead to prove a live update across >=1 poll.
    const completed = await $('[data-testid="count-emails-completed"]');
    await completed.waitForExist({ timeout: 30000 });
    const initial = await completed.getText();
    await browser.waitUntil(
      async () => (await completed.getText()) !== initial,
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: "emails completed count did not change across a poll",
      },
    );
  });
});
