# e2e (WebdriverIO)

- CI-first: C2's ubuntu `e2e` job builds a release bundle, then runs
  `xvfb-run -a npm run e2e` via **tauri-driver** (the `external` provider).
- `tauri-driver` is Linux/Windows only. Local runs need it on a supported OS,
  or set `WDIO_TAURI_PROVIDER=embedded` — which additionally requires
  registering `tauri-plugin-wdio-webdriver` in the app (deferred).
- Binary path: repo-root `target/release/queue-boss` (override with
  `WDIO_TAURI_APP`).
