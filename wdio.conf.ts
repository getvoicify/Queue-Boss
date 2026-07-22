// WebdriverIO config for Queue Boss desktop e2e.
//
// Driver provider defaults to `external` (tauri-driver) — C2's Linux CI job
// installs tauri-driver + webkit2gtk-driver and runs this suite headless under
// xvfb. tauri-driver is Windows+Linux only, so local macOS runs need either
// tauri-driver on a supported OS or WDIO_TAURI_PROVIDER=embedded (which further
// requires registering tauri-plugin-wdio-webdriver in the app — deferred).
//
// The app binary path is the repo-ROOT `target/release/<bin>` — the Cargo
// workspace relocates the target dir to the repo root (A3).
const driverProvider = (process.env.WDIO_TAURI_PROVIDER ?? "external") as
  | "embedded"
  | "external"
  | "crabnebula";
const application = process.env.WDIO_TAURI_APP ?? "./target/release/queue-boss";

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./tests/e2e/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [
    {
      "tauri:options": {
        application,
        driverProvider,
      },
    },
  ],
  services: ["@wdio/tauri-service"],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
  logLevel: "warn",
};
