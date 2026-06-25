# TODO: Remaining parity with `main`

This is the rewrite parity backlog. `main` is the functionality reference; `rewrite` should preserve product behaviour while keeping the new package boundaries from `PLAN.md`.

Principles while working this list:

- Keep Rust as runtime truth.
- Keep SwiftUI dumb: render state, call thin APIs.
- Do not recreate old god objects.
- Keep files small and responsibility-focused.
- Put tests in sibling `tests/` folders, not inside `src/`.
- Prefer new rewrite concepts where intentional: Functions → Hooks, Dev servers → Services, TOML config, embedded hook metadata.

## Current known completed parity

- [x] Service log files under `~/.rack/logs/services/*.log`.
- [x] Open service logs in terminal.
- [x] Basic cron hook scheduler for interval schedules.
- [x] Launch at login controller.
- [x] Basic App Intents for start/stop/stop-all/reload hooks/launch login.
- [x] Pingora-based proxy for service hosts and `rack.local` hooks.
- [x] Hook SDK route/cron macros with embedded WASM metadata.
- [x] `rack hook init`, `rack hook build`, `rack hook deploy`.

---

## P0: App surface / Swift parity

### Settings window

Main refs:

- `Sources/Rack/App/RackApp.swift`
- `Sources/Rack/UI/SettingsView.swift`
- `Sources/Rack/UI/GeneralSettingsPage.swift`
- `Sources/Rack/UI/ServersSettingsPage.swift`
- `Sources/Rack/UI/FunctionsRuntimeSettingsPage.swift`

Rewrite refs:

- `packages/mac/src/rack-app.swift`
- `packages/ui/src/menu-bar/menu-bar.swift`

TODO:

- [ ] Add a real Settings window/scene in `packages/mac`.
- [ ] Wire `MenuBarContentView(openSettings:)`; current gear/add-service paths effectively no-op.
- [ ] Add Settings shell in `packages/ui`.
- [ ] Add General settings page.
- [ ] Add Services settings page.
- [ ] Add Hooks settings/runtime page.
- [ ] Keep macOS window ownership in `packages/mac`; keep SwiftUI views in `packages/ui`.

### General settings

Main ref: `Sources/Rack/UI/GeneralSettingsPage.swift`

TODO:

- [ ] Add launch-at-login toggle using `LaunchAtLoginController`.
- [ ] Show launch-at-login error/status messages.
- [ ] Add terminal picker: Ghostty, Terminal, iTerm2, Warp/default.
- [ ] Persist selected terminal through Rack config, not stale `@AppStorage` if possible.
- [ ] Show config file path.
- [ ] Add reveal/open config actions.
- [ ] Add quit action.
- [ ] Add standard ports UI once standard port support exists again.

### Service management UI

Main refs:

- `Sources/Rack/UI/ServersSettingsPage.swift`
- `Sources/Rack/UI/ServerSettingsPanel.swift`
- `Sources/Rack/UI/ServerConfigurationForm.swift`
- `Sources/Rack/Server/ServerStore.swift`

Rewrite refs:

- `packages/ui/src/menu-bar/service-models.swift`
- `packages/ui/src/menu-bar/rack-view-model.swift`
- `packages/core/src/config/mod.rs`
- `packages/cli/src/service.rs`

TODO:

- [ ] Add service add UI.
- [ ] Add service edit UI.
- [ ] Add service remove UI.
- [ ] Add service duplicate UI if still desired.
- [ ] Add per-service detail panel.
- [ ] Expose/edit fields: `name`, `host`, `run`, `working_dir`, `auto_start`.
- [ ] If restored in config, expose/edit `environment`, custom domain, explicit port, args/port flag.
- [ ] Add copy local URL action.
- [ ] Add open working directory action if main had it and it still fits.
- [ ] Add open full logs action from service detail.
- [ ] Add recent output panel in service detail.

### Restart action

Main refs:

- `Sources/Rack/Server/ServerStore+Lifecycle.swift`
- `Sources/Rack/UI/ServerSettingsPanel.swift`
- `Sources/Rack/App/RackIntents.swift`

Rewrite refs:

- Backend exists around `packages/services/src/runtime.rs`
- `packages/services/src/ffi/functions.rs`
- `packages/mac/src/rack-services-client.swift`
- `packages/ui/src/menu-bar/rack-view-model.swift`
- `packages/mac/src/rack-intents.swift`

TODO:

- [ ] Bind restart through services FFI.
- [ ] Add `RackServicesClient.restart(id:)`.
- [ ] Add `RackViewModel.restart(id:)`.
- [ ] Add restart UI where appropriate.
- [ ] Add `RestartRackServiceIntent`.
- [ ] Add URL action for restart once URL scheme exists.

### URL scheme handling

Main refs:

- `Sources/Rack/App/RackApp.swift`
- `Makefile` on `main` for `CFBundleURLTypes`
- `README.md`

TODO:

- [ ] Register `rack://` URL scheme in app bundle generation.
- [ ] Add app delegate or `onOpenURL` handling in `packages/mac`.
- [ ] Implement `rack://settings`.
- [ ] Implement `rack://server/start?id=...` or service equivalent.
- [ ] Implement `rack://server/stop?id=...` or service equivalent.
- [ ] Implement `rack://server/restart?id=...` or service equivalent.
- [ ] Implement `rack://server/stop-all` or service equivalent.
- [ ] Implement `rack://functions/reload` as `rack://hooks/reload`, with compat alias if needed.
- [ ] Document new/compat URL scheme.

### App lifecycle

Main ref: `Sources/Rack/App/RackApp.swift`

Rewrite refs:

- `packages/mac/src/rack-app.swift`
- `packages/ui/src/menu-bar/rack-view-model.swift`
- `packages/mac/src/rack-services-client.swift`

TODO:

- [ ] Add explicit lifecycle owner in `packages/mac`, likely `AppDelegate`.
- [ ] Initialize runtime/model before App Intents can run.
- [ ] Move `RackIntentBridge.model` setup out of menu `.task` so shortcuts work before the menu opens.
- [ ] Handle termination.
- [ ] Call runtime shutdown on app termination.
- [ ] Stop active services on Quit/terminate if that remains product behaviour.
- [ ] Keep app lifecycle out of SwiftUI views.

### CLI installer

Main refs:

- `Sources/Rack/App/CLIInstaller.swift`
- `Sources/Rack/App/RackApp.swift`

Rewrite refs:

- `Makefile` bundles CLI into `Contents/Resources/rack`

TODO:

- [ ] Restore bundled CLI installation/symlink flow.
- [ ] Link bundled CLI to `~/.local/bin/rack`.
- [ ] Add/update `~/.zprofile` for `~/.local/bin` if needed.
- [ ] Handle existing non-symlink safely.
- [ ] Surface install failures non-intrusively.

### App Intents polish

Main ref: `Sources/Rack/App/RackIntents.swift`

Rewrite ref: `packages/mac/src/rack-intents.swift`

TODO:

- [ ] Add restart intent.
- [ ] Add launch-at-login intents to shortcuts if desired.
- [ ] Add phrase variants matching main, e.g. “with Rack” and “in Rack”.
- [ ] Ensure entity search includes host/working directory once available.
- [ ] Ensure App Intent bridge is available before menu is opened.

---

## P0: Service runtime parity

### Config compatibility and migration

Main refs:

- `Sources/rack-core/src/config/storage.rs`
- `Sources/rack-core/src/config/models.rs`

Rewrite refs:

- `packages/core/src/config/mod.rs`
- `packages/core/src/config/backfill.rs`
- `packages/core/src/config/write.rs`

TODO:

- [ ] Decide whether to import old `~/.config/rack/config.json` automatically.
- [ ] If yes, implement JSON → TOML migration.
- [ ] Preserve existing main users’ services where possible.
- [ ] Map old ServerBar paths if still relevant.
- [ ] Keep source TOML user-editable and do not rewrite unnecessarily.
- [ ] Add migration tests in `packages/core/tests/`.

### Config model gaps

Main had fields/concepts not currently represented in rewrite.

TODO:

- [ ] Decide fate of `arguments`.
- [ ] Decide fate of per-service `environment`.
- [ ] Decide fate of `customDomain`.
- [ ] Decide fate of explicit configured `port`.
- [ ] Decide fate of `portFlag`.
- [ ] Decide whether `run` remains a single shell command or splits into command/args.
- [ ] If intentionally removed, provide migration behaviour and docs.
- [ ] If restored, update core config, services, CLI, FFI, Swift models, and UI.

### Service launch behaviour

Main ref: `Sources/rack-core/src/process.rs`

Rewrite refs:

- `packages/services/src/process/mod.rs`
- `packages/services/src/supervisor/*`
- `packages/core/src/config/mod.rs`

TODO:

- [ ] Restore or replace login-shell environment loading.
- [ ] Merge current process env.
- [ ] Add per-service env overrides if config supports them.
- [ ] Preserve color env vars (`FORCE_COLOR`, `CLICOLOR_FORCE`, `TERM`).
- [ ] Decide on explicit `PORT`/`HOST` injection.
- [ ] Decide on old `portFlag` behaviour.
- [ ] If no explicit ports by design, document port detection model.
- [ ] Add process launch tests where possible.

### Readiness and failure states

Main refs:

- `Sources/rack-core/src/process_supervisor/readiness.rs`
- `Sources/Rack/Server/ServerStore+Lifecycle.swift`

Rewrite refs:

- `packages/services/src/registry/service.rs`
- `packages/services/src/supervisor/runtime.rs`
- `packages/services/src/snapshot.rs`

TODO:

- [ ] Add explicit failed service state, e.g. `Failed { message }`.
- [ ] Add readiness timeout for services that never open a port.
- [ ] Surface readiness failure in snapshots/FFI/UI.
- [ ] Avoid leaving services stuck in `Starting` forever.
- [ ] Add tests in `packages/services/tests/`.

### Pending route behaviour

Main refs:

- `Sources/rack-core/src/process_supervisor/routes.rs`
- `Sources/rack-core/src/routes.rs`
- `Sources/Rack/Proxy/HTTPProxyHandler+Backend.swift`

Rewrite refs:

- `packages/services/src/runtime.rs`
- `packages/proxy/src/services/*`

TODO:

- [ ] Register service origin with proxy when service enters `Starting`, if desired.
- [ ] Have proxy wait/retry briefly while backend is starting.
- [ ] Preserve good UX for opening `jaf.localhost` immediately after start.
- [ ] Return clear starting/failure responses.

### Service bridge / Unix sockets

Main refs:

- `Sources/rack-core/src/process.rs`
- `Sources/rack-bridge/src/*`
- `Sources/Rack/Proxy/HTTPProxyHandler+Backend.swift`

Rewrite refs:

- `packages/services/src/process/mod.rs`
- `packages/proxy/src/services/destination.rs`

TODO:

- [ ] Decide if `rack-bridge` / Unix socket launch path is still needed.
- [ ] If yes, add destination support for Unix sockets.
- [ ] If no, remove/doc old behaviour and ensure loopback TCP covers all supported services.

### Runtime snapshots and FFI

TODO:

- [ ] Expose all service fields needed by Swift settings.
- [ ] Expose config path.
- [ ] Expose config mutation APIs: add, edit, remove, duplicate, save.
- [ ] Expose restart.
- [ ] Expose failed state.
- [ ] Keep FFI functions thin wrappers over native runtime functions.
- [ ] Keep shared struct layouts/version checks updated.

---

## P0: Proxy parity

### Host routing

Main refs:

- `Sources/rack-core/src/routes.rs`
- `Sources/rack-core/src/proxy.rs`

Rewrite refs:

- `packages/proxy/src/services/*`
- `packages/proxy/tests/route.rs`

TODO:

- [ ] Decide whether nested/base-subdomain fallback is still required.
- [ ] If yes, support `fix-auth.myapp.localhost` resolving to base service `myapp`.
- [ ] If no, document intentional rejection of nested localhost hosts.
- [ ] Keep hooks on `rack.local`.
- [ ] Keep services on `*.localhost`.

### Backend wait/retry and loop detection

Main refs:

- `Sources/Rack/Proxy/HTTPProxyHandler.swift`
- `Sources/Rack/Proxy/HTTPProxyHandler+Backend.swift`
- `Sources/rack-core/src/proxy.rs`

Rewrite ref: `packages/proxy/src/server/*`

TODO:

- [ ] Add backend wait/retry while service is starting.
- [ ] Restore loopback proxy loop detection.
- [ ] Return HTTP `508 Loop Detected` with useful guidance where appropriate.
- [ ] Add tests for loop detection and starting-service waits.

### WebSocket proxying

Main refs:

- `Sources/Rack/Proxy/ProxyServer.swift`
- `Sources/Rack/Proxy/WebSocketProxy.swift`

Rewrite refs:

- `packages/proxy/src/server/*`
- `packages/proxy/tests/listener.rs`

TODO:

- [ ] Verify current Pingora WebSocket upgrade/tunnel behaviour against real Vite/Bun apps.
- [ ] If tests only cover handshake but not frame tunnelling, add frame tunnel tests.
- [ ] Implement explicit upgrade tunnelling if Pingora path is insufficient.
- [ ] Test common dev servers: Vite, Next, Bun, Rails/Hotwire if relevant.

### HTTPS and standard ports

Main refs:

- `Sources/Rack/Proxy/ProxyServer.swift`
- `Sources/Rack/Proxy/ProxyPortForwarding.swift`
- `Sources/RackPortRelay/*`
- `Sources/rack-bridge/src/tunnel.rs`

Rewrite refs:

- `packages/proxy/src/server/*`
- `packages/services/src/runtime.rs`
- `packages/core/public/default-config.toml`

TODO:

- [ ] Restore HTTPS listener if still required.
- [ ] Restore local TLS certificate generation/trust if still required.
- [ ] Restore privileged standard port forwarding for 80/443 if still required.
- [ ] Restore `/etc/hosts` management for `rack.local` if still required.
- [ ] Bundle any privileged helper/relay in app build if still required.
- [ ] Wire `use_standard_ports` TOML config to actual runtime behaviour.
- [ ] Expose status/errors in Settings.

### Listener coverage

Main ref: `Sources/Rack/Proxy/ProxyServer.swift`

TODO:

- [ ] Bind IPv4 loopback.
- [ ] Bind IPv6 loopback (`::1`) if main behaviour is still needed.
- [ ] Ensure port selection/fallback matches main or document changes.

### `rack.local` behaviour

Main refs:

- `Sources/Rack/Proxy/HTTPProxyHandler.swift`
- `Sources/rack-core/src/proxy.rs`

Rewrite refs:

- `packages/proxy/src/hooks.rs`
- `packages/hooks/src/http/*`

TODO:

- [ ] Confirm root `rack.local/` response should be hook dispatch or simple Rack response.
- [ ] Preserve reserved `/_*` semantics if still needed.
- [ ] Add compatibility tests for `rack.local` behaviours users rely on.

---

## P0: Hooks runtime and SDK parity

### CLI hook command parity

Main refs:

- `Sources/rack-cli-rs/src/main.rs`
- `Sources/rack-cli-rs/src/function_cli/*`

Rewrite refs:

- `packages/cli/src/main.rs`
- `packages/cli/src/hook.rs`

TODO:

- [ ] Add aliases/compat group for `rack fn ...` if desired.
- [ ] Add `rack hook add` or decide `init` replaces it.
- [ ] Add `rack hook compile` alias for build if desired.
- [ ] Add `rack hook test`.
- [ ] Add `rack hook install` or define deploy as replacement.
- [ ] Add `rack hook list` / `ls`.
- [ ] Add `rack hook remove` / `rm` / `uninstall`.
- [ ] Add `--replace` or equivalent for deploy/install.
- [ ] Add copy install mode if symlink/move deploy is not enough.
- [ ] Add clear deployed-hook error reporting.

### `hook init` parity

Main ref: `Sources/rack-cli-rs/src/function_cli/init.rs`

Rewrite ref: `packages/cli/src/hook.rs`

TODO:

- [ ] Allow omitted path/current directory if desired.
- [ ] Allow existing empty directory.
- [ ] Reject non-empty directories with clear error.
- [ ] Sanitize package names like main did.
- [ ] Decide whether no `manifest.toml` is final.
- [ ] Keep embedded proc metadata as new source of truth.
- [ ] Ensure template demonstrates route and cron patterns.
- [ ] Ensure required wasm target is installed or print actionable error.

### Hook build/deploy packaging

Main refs:

- `Sources/rack-cli-rs/src/function_cli/build.rs`
- `Sources/rack-cli-rs/src/function_cli/install.rs`

Rewrite refs:

- `packages/cli/src/hook.rs`
- `packages/services/src/hooks/mod.rs`

TODO:

- [ ] Decide final WASM target: current rewrite uses `wasm32-unknown-unknown`; main used `wasm32-wasip1`.
- [ ] Use `cargo metadata` to identify package/cdylib output robustly.
- [ ] Validate project shape before build.
- [ ] Produce stable artifact path/name if useful.
- [ ] Validate embedded metadata at build/deploy time.
- [ ] Avoid surprising destructive deploy moves, or document them clearly.
- [ ] Ensure symlink-back deploy works robustly across paths/filesystems.

### SDK macro surface

Main refs:

- `Sources/rack-macros/src/lib.rs`
- `Sources/rack-sdk-rs/src/*`

Rewrite refs:

- `packages/sdk-macro/src/*`
- `packages/sdk/src/*`

TODO:

- [ ] Decide whether to preserve zero-arg `#[rack::route]` / `#[rack::cron]` compatibility.
- [ ] Keep new syntax like `#[rack::route(GET, "path")]` as desired.
- [ ] Add/restore `#[rack::payload]` if public SDK parity matters.
- [ ] Add typed `Request<T>`.
- [ ] Add `Payload` trait.
- [ ] Add JSON body parsing.
- [ ] Add `String` and `()` payload implementations.
- [ ] Restore handler `Result<Response>` and `?` support.
- [ ] Map payload errors to 400.
- [ ] Map handler errors to 500.
- [ ] Add compile tests for valid/invalid macro signatures.

### HTTP request API

Main ref: `Sources/rack-sdk-rs/src/request.rs`

Rewrite refs:

- `packages/sdk/src/http.rs`
- `packages/hooks/src/http/dispatch/*`
- `packages/proxy/src/hooks.rs`

TODO:

- [ ] Expose method.
- [ ] Expose path.
- [ ] Expose full URI/query.
- [ ] Expose host.
- [ ] Expose headers.
- [ ] Expose `header(name)` helper.
- [ ] Expose body.
- [ ] Expose package/hook metadata.
- [ ] Expose route id.
- [ ] Expose route pattern.
- [ ] Expose matched path.
- [ ] Ensure proxy sends enough request data to hooks.

### HTTP response API

Main refs:

- `Sources/rack-sdk-rs/src/http_response.rs`
- `Sources/rack-sdk-rs/src/response.rs`
- `Sources/rack-core/src/functions/runtime/response.rs`

Rewrite refs:

- `packages/sdk/src/response.rs`
- `packages/hooks/src/http/dispatch/*`

TODO:

- [ ] Validate statuses are `100..=599`.
- [ ] Normalize/lowercase headers consistently.
- [ ] Decide duplicate header behaviour.
- [ ] Add helpers: `ok`, `created`, `bad_request`, `teapot`, `server_error`.
- [ ] Add builder helpers: `.text()`, `.html()`, `.csv()`, `.json()`, `.bytes()`.
- [ ] Align content types with main, including charset.
- [ ] Test response normalization.

### Hook route matching

Main refs:

- `Sources/rack-core/src/functions/manifest.rs`
- `Sources/rack-core/src/functions/routing.rs`
- `Sources/rack-core/src/functions/tests.rs`

Rewrite refs:

- `packages/sdk-macro/src/metadata.rs`
- `packages/hooks/src/http/dispatch/router.rs`

TODO:

- [ ] Normalize leading slash.
- [ ] Trim trailing slash if main behaviour should hold.
- [ ] Reject `/` if still reserved.
- [ ] Reject `/_*` reserved paths if still needed.
- [ ] Add glob route support if still needed.
- [ ] Choose most specific route.
- [ ] Detect equal-specificity conflicts.
- [ ] Return useful conflict/reserved-path errors.
- [ ] Add route matching tests in `packages/hooks/tests/`.

### Hook runtime safety and behaviour

Main refs:

- `Sources/rack-core/src/functions/runtime/process.rs`
- `Sources/rack-core/src/functions/runtime/timeout.rs`
- `Sources/rack-core/src/functions/runtime/dispatch.rs`

Rewrite refs:

- `packages/hooks/src/runtime/wasm.rs`
- `packages/hooks/src/http/dispatch/*`

TODO:

- [ ] Add timeout/fuel/epoch interruption for in-process Wasmtime.
- [ ] Prevent hanging hooks from blocking runtime threads indefinitely.
- [ ] Validate required exports at load/deploy time.
- [ ] Surface load errors in hook summaries.
- [ ] Harden guest memory pointer/length handling.
- [ ] Use checked arithmetic for pointer + length.
- [ ] Handle negative lengths safely.
- [ ] Review `rack_alloc` / `rack_dealloc` ABI pairing.
- [ ] Add tests for malformed guest outputs where feasible.

### Hook logging

Main refs:

- `Sources/rack-sdk-rs/src/log.rs`
- `Sources/rack-core/src/functions/logs.rs`
- `Sources/rack-core/src/functions/runtime/process.rs`

TODO:

- [ ] Restore `rack::log::info`.
- [ ] Restore `rack::log::warn`.
- [ ] Restore `rack::log::error`.
- [ ] Capture hook stderr/stdout or structured logs.
- [ ] Write hook logs under `~/.rack/logs/hooks/` or chosen equivalent.
- [ ] Add hook invocation started/finished/duration logs.
- [ ] Surface hook failures in UI/CLI.

### Cron parity

Main refs:

- `Sources/rack-core/src/functions/scheduler.rs`
- `Sources/rack-core/src/schedule.rs`
- `Sources/rack-sdk-rs/src/request.rs`
- `Sources/rack-macros/src/lib.rs`

Rewrite refs:

- `packages/services/src/hooks/scheduler.rs`
- `packages/hooks/src/runtime/wasm.rs`
- `packages/sdk/src/cron.rs`
- `packages/sdk-macro/src/cron.rs`

TODO:

- [ ] Add calendar schedule parser support: e.g. `friday at 17:00`, `weekdays at 9:30am`.
- [ ] Reuse/port main schedule semantics where still desired.
- [ ] Add `CronEvent` argument support.
- [ ] Include package/id/schedule/scheduled_at in cron event.
- [ ] Log `cron.started`, `cron.finished`, `cron.error` equivalents.
- [ ] Decide first-run semantics: after interval vs computed `next_after`.
- [ ] Add tests for interval and calendar cron schedules.

### Embedded metadata vs old manifest

Main refs:

- `Sources/rack-core/src/functions/manifest.rs`
- `Sources/rack-cli-rs/src/function_cli/types.rs`

Rewrite refs:

- `packages/sdk-macro/src/metadata.rs`
- `packages/hooks/src/runtime/metadata.rs`
- `packages/services/src/hooks/mod.rs`

TODO:

- [ ] Keep embedded WASM metadata as source of truth.
- [ ] Add package identity/version metadata if needed.
- [ ] Add route IDs if needed.
- [ ] Add cron IDs/function names if needed.
- [ ] Show metadata parse/load errors in `rack hook list` and UI.
- [ ] Document migration from `manifest.toml` to proc macro metadata.

### Hook UI/runtime page

Main ref: `Sources/Rack/UI/FunctionsRuntimeSettingsPage.swift`

Rewrite refs:

- `packages/ui/src/menu-bar/*`
- `packages/mac/src/rack-services-client.swift`
- `packages/mac/src/ffi/rack-hooks-payload.swift`

TODO:

- [ ] Add hooks runtime/settings UI page.
- [ ] Show deployed hooks.
- [ ] Show HTTP routes.
- [ ] Show cron hooks.
- [ ] Show hook load errors/conflicts.
- [ ] Add reload hooks button if safe.
- [ ] Make reload actually reload proxy registry/scheduler, or label it as summary refresh only.

---

## P1: CLI/service UX parity

### Top-level CLI commands

Main refs:

- `Sources/rack-cli-rs/src/main.rs`
- `Sources/rack-core/src/config/ipc.rs`

Rewrite refs:

- `packages/cli/src/main.rs`
- `packages/cli/src/service.rs`
- `packages/services/src/control/*`

TODO:

- [ ] Decide whether to preserve top-level `rack ls`.
- [ ] Decide whether to preserve top-level `rack start <name>`.
- [ ] Decide whether to preserve top-level `rack stop <name>`.
- [ ] Decide whether to preserve top-level `rack rm <name>`.
- [ ] Support name/host lookup in addition to ID if user-friendly.
- [ ] Align control socket path with app expectations.
- [ ] Document `RACK_CONTROL_SOCKET` override.

### `rack dev`

Main ref: `Sources/rack-cli-rs/src/dev.rs`

TODO:

- [ ] Reimplement `rack dev` auto-registration if still desired.
- [ ] Detect common project types.
- [ ] Infer service name/host.
- [ ] Infer run command.
- [ ] Register with running app.
- [ ] Start service.
- [ ] Keep behaviour compatible with new no-explicit-port model.

### CLI list output

Main ref: `Sources/rack-cli-rs/src/main.rs`

Rewrite refs:

- `packages/cli/src/service.rs`
- `packages/services/src/snapshot.rs`

TODO:

- [ ] Show accurate reachable URL including proxy port if not standard.
- [ ] Show running/stopped/starting/failed status clearly.
- [ ] Show detected backend ports where useful.
- [ ] Show service host/origin.
- [ ] Support concise human output and machine-readable output if useful.

### Config CRUD over CLI/control socket

TODO:

- [ ] Add service edit command.
- [ ] Add service duplicate command if kept.
- [ ] Add batch delete if kept.
- [ ] Ensure CLI config mutations notify/rerender running app state.
- [ ] Ensure mutations keep TOML formatting reasonably clean.

---

## P1: Standard ports, DNS, and local networking

Main refs:

- `Sources/Rack/Proxy/ProxyPortForwarding.swift`
- `Sources/RackPortRelay/*`
- `Sources/rack-bridge/src/tunnel.rs`

TODO:

- [ ] Decide v1 standard-port strategy.
- [ ] Restore privileged helper/relay if needed.
- [ ] Restore install/uninstall flow.
- [ ] Add status/error reporting.
- [ ] Add Settings toggle.
- [ ] Add tests where practical.
- [ ] Document required permissions.
- [ ] Ensure `rack.local` resolution works without manual setup, if promised.

---

## P1: Testing backlog

### Core/config tests

- [ ] JSON migration tests if implemented.
- [ ] Old config field mapping tests.
- [ ] TOML round-trip/write tests for new mutable settings.
- [ ] Validation tests for restored fields.

### Services tests

- [ ] Start → running → stop lifecycle tests.
- [ ] Restart tests.
- [ ] Failed readiness tests.
- [ ] Readiness timeout tests.
- [ ] Environment merge tests.
- [ ] Log file creation/append/truncate tests.
- [ ] FFI ABI layout/version tests.

### Proxy tests

- [ ] Nested host fallback tests if supported.
- [ ] Backend wait/retry tests.
- [ ] Loop detection tests.
- [ ] Full WebSocket frame tunnel tests.
- [ ] IPv6 listener tests.
- [ ] HTTPS listener tests if restored.
- [ ] Standard port behaviour tests if practical.

### Hooks tests

- [ ] Route normalization tests.
- [ ] Glob matching tests.
- [ ] Route conflict tests.
- [ ] Reserved path tests.
- [ ] Request metadata injection tests.
- [ ] Response normalization tests.
- [ ] Timeout/hanging hook tests.
- [ ] Bad guest ABI tests.
- [ ] Cron interval tests.
- [ ] Cron calendar schedule tests.
- [ ] Hook logging tests.

### CLI tests

- [ ] `rack service add/list/start/stop/remove` integration tests.
- [ ] `rack service edit` tests once added.
- [ ] `rack dev` tests once added.
- [ ] `rack hook init/build/deploy/list/remove/test` tests.
- [ ] Compat alias tests if aliases are kept.

### Swift/UI tests or manual QA

- [ ] Menu bar opens and displays services.
- [ ] Start/stop/restart from menu.
- [ ] Logs preview ANSI colours.
- [ ] Open logs in configured terminal.
- [ ] Settings window opens.
- [ ] Add/edit/remove service from Settings.
- [ ] Launch at login toggle.
- [ ] App Intents before menu opens.
- [ ] URL scheme actions.
- [ ] Quit/termination cleanup.

---

## P2: Documentation and release polish

### Docs

- [ ] Update README for rewrite architecture.
- [ ] Document TOML config path: `~/.config/rack/config.toml`.
- [ ] Document generated/cache config behaviour if present.
- [ ] Document services config format.
- [ ] Document hook SDK and proc metadata.
- [ ] Document hook deploy layout under `~/.rack/hooks`.
- [ ] Document CLI commands.
- [ ] Document App Intents/Shortcuts.
- [ ] Document URL scheme.
- [ ] Document standard ports/DNS setup if restored.

### Build/release

- [ ] Ensure app bundle includes Rust dynamic/static artifacts correctly.
- [ ] Ensure CLI bundle/install works.
- [ ] Ensure app icon/resources match main.
- [ ] Ensure signing/notarization flow if applicable.
- [ ] Add CI for `cargo test`.
- [ ] Add CI for `make swift` or Swift build.
- [ ] Add formatting checks.
- [ ] Add release packaging smoke test.

### Cleanup

- [ ] Decide what to do with untracked `packages/hooks/src/std/`.
- [ ] Remove stale old concepts from UI, e.g. `standardPortsEnabled` AppStorage if replaced by TOML.
- [ ] Remove dead hodge-podge FFI if replaced by cleaner bridge modules.
- [ ] Keep package folders nested by responsibility.
- [ ] Split any files that drift past maintainable size.

---

## Suggested implementation order

1. Settings window + service config CRUD UI/API.
2. App lifecycle/AppDelegate + URL scheme + early AppIntent bridge.
3. Restart FFI/UI/Intent.
4. Service runtime failed/readiness states.
5. Hook CLI list/remove/test.
6. Hook request/response SDK parity.
7. Cron calendar schedules + event payload.
8. Proxy backend wait/loop detection/WebSocket verification.
9. Standard ports/HTTPS/DNS if still in v1 scope.
10. Config migration from main JSON if desired.
11. Docs/release polish.
