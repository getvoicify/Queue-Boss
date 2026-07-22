// WebdriverIO config for Queue Boss desktop e2e.
// `driverProvider` + `appBinaryPath` live in the @wdio/tauri-service SERVICE options;
// `external` drives tauri-driver (Linux CI). tauri-driver and WebKitWebDriver are
// auto-detected; WDIO_TAURI_DRIVER / WDIO_NATIVE_DRIVER pin those paths when set.
const driverProvider = (process.env.WDIO_TAURI_PROVIDER ?? "external") as
  | "external"
  | "official"
  | "crabnebula"
  | "embedded";
const application = process.env.WDIO_TAURI_APP ?? "./target/release/queue-boss";

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./tests/e2e/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "tauri",
      "tauri:options": {
        application,
      },
    },
  ],
  services: [
    [
      "@wdio/tauri-service",
      {
        driverProvider,
        appBinaryPath: application,
        ...(process.env.WDIO_TAURI_DRIVER
          ? { tauriDriverPath: process.env.WDIO_TAURI_DRIVER }
          : {}),
        ...(process.env.WDIO_NATIVE_DRIVER
          ? { nativeDriverPath: process.env.WDIO_NATIVE_DRIVER }
          : {}),
      },
    ],
  ],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    // 3 min: the external tauri-driver provider has no embedded window-focus
    // plugin, so the service's per-command focus hook adds ~5s per focus command.
    timeout: 180000,
  },
  logLevel: "warn",
};
