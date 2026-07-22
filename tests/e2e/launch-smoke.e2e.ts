// Launch-smoke: booted by C2's Linux CI e2e job (tauri-driver + xvfb). Asserts the window title.
describe("Queue Boss launch smoke", () => {
  it("boots the app window with the expected title", async () => {
    const title = await browser.getTitle();
    expect(title).toContain("Queue Boss");
  });
});
