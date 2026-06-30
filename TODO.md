# TODO

Wall-of-bullets parity backlog for the Rack rewrite.

Keep in mind:

- `main` is the behaviour reference, not the architecture reference.
- New code stays under `packages/`.
- Rust owns runtime truth.
- Swift owns macOS shell + UI.
- SwiftUI views stay dumb.
- Do not recreate `ServerStore` or any god object.
- Do not recreate current Settings code. Build the rewrite Settings surface from scratch against the new models/APIs.
- Keep files small and split by responsibility.
- Put tests in sibling `tests/` folders, not `src/`.
- Functions are now Hooks.
- Dev servers are now Services.
- Config is TOML at `~/.config/rack/config.toml`.
- Deployed hooks live under `~/.rack/hooks`.

## Already done

- [x] Service log files under `~/.rack/logs/services/*.log`.
- [x] Open service logs in terminal.
- [x] Basic ANSI log preview in menu bar.
- [x] Service stop state handling fixed.
- [x] Basic bespoke services FFI bridge.
- [x] Pingora-based proxy.
- [x] Basic service host routing for `*.localhost`.
- [x] Basic hook dispatch for `rack.local`.
- [x] Hook WASM execution runtime.
- [x] Hook HTTP route macro with embedded WASM metadata.
- [x] Hook cron macro with embedded WASM metadata.
- [x] Basic deployed cron scheduler for interval schedules.
- [x] `rack hook init`.
- [x] `rack hook build`.
- [x] `rack hook deploy`.
- [x] Launch-at-login controller.
- [x] Basic App Intents for start/stop/stop-all/reload hooks/launch login.

## App / macOS shell

- [x] Add a real app lifecycle owner in `packages/mac`.
- [x] Add `AppDelegate` or equivalent.
- [x] Keep app lifecycle out of SwiftUI views.
- [x] Initialize runtime/model before menu UI appears.
- [x] Initialize App Intent bridge before any shortcut can run.
- [x] Move `RackIntentBridge.model = model` out of the menu `.task`.
- [x] Handle app termination.
- [x] Call runtime shutdown on termination.
- [x] Decide whether app termination stops all running services.
- [x] Ensure Quit button and normal app quit share shutdown semantics.
- [x] Add `rack://` URL scheme to app bundle.
- [x] Add URL handling in mac package.
- [x] Add `rack://settings`.
- [x] Add `rack://service/start?id=...`.
- [x] Add `rack://service/stop?id=...`.
- [x] Add `rack://service/restart?id=...`.
- [x] Add `rack://service/stop-all`.
- [x] Add `rack://hooks/reload`.
- [x] Add compat handling for old `rack://server/...` URLs if desired.
- [x] Add compat handling for old `rack://functions/reload` if desired.
- [x] Restore bundled CLI installer.
- [x] Link bundled CLI to `~/.local/bin/rack`.
- [ ] Add `~/.local/bin` to user shell profile when safe.
- [x] Handle existing non-symlink `rack` safely.
- [x] Surface CLI install errors quietly and usefully.
- [x] Ensure app bundle includes CLI resource.
- [ ] Ensure app bundle includes correct Rust library/binary artifacts.
- [ ] Restore app icon/resources if missing.
- [ ] Revisit signing/notarization flow.

## Settings UI, built from scratch

- [x] Build a new Settings window from scratch.
- [x] Do not port the current/main Settings implementation directly.
- [x] Use `main` only as feature reference for Settings.
- [x] Put Settings window ownership in `packages/mac`.
- [x] Put Settings SwiftUI views in `packages/ui`.
- [x] Build Settings against new rewrite view models/APIs.
- [x] Keep Settings views dumb.
- [x] Add Settings shell/sidebar/tabs.
- [x] Add General page.
- [x] Add Services page.
- [x] Add Hooks page.
- [ ] Add Network/Ports page if that feels cleaner than General.
- [x] Wire menu bar gear button to open Settings.
- [x] Wire “Add Service” empty-state/menu action to open Settings or add-service flow.
- [x] Make Settings usable without menu bar popover state.
- [x] Add launch-at-login toggle.
- [x] Show launch-at-login status/errors.
- [x] Add terminal picker.
- [x] Support Ghostty.
- [x] Support Terminal.app.
- [x] Support iTerm/iTerm2.
- [x] Support Warp/default `.command` fallback.
- [x] Persist terminal selection through Rack config, not stale `@AppStorage`.
- [x] Show config path.
- [x] Add reveal config in Finder.
- [x] Add open config in editor/default app.
- [ ] Add reload config action if safe.
- [ ] Show config parse errors.
- [ ] Add standard ports toggle once runtime support exists.
- [ ] Show standard ports status/errors.
- [x] Add quit action.
- [ ] Add about/version info if useful.

## Menu bar UI

- [x] Keep current good product shape: small native menu bar popover.
- [x] Show service rows.
- [x] Show status dots/states.
- [x] Show running count.
- [x] Show start/stop controls.
- [x] Keep restart/refresh button removed unless restart is explicitly reintroduced.
- [x] Add restart only where it is semantically correct.
- [x] Keep logs preview.
- [x] Keep ANSI formatting in logs preview.
- [x] Show failed service state when runtime supports it.
- [x] Show starting state clearly.
- [ ] Show detected backend port(s) if useful.
- [x] Show local service URL.
- [x] Add copy URL action.
- [x] Add open URL action.
- [x] Add open logs action.
- [ ] Add open working directory action if useful.
- [x] Add hooks summary section if useful.
- [x] Add hook reload button only if reload actually reloads runtime/proxy/scheduler.
- [x] Otherwise label hook action as summary refresh only or omit it.
- [x] Ensure menu does not own runtime state.

## Service config model

- [x] Decide whether `run` remains one shell command string.
- [x] Decide whether to restore separate `command` + `arguments`.
- [x] Decide whether to restore per-service environment variables.
- [x] Decide whether to restore custom domains.
- [x] Decide whether to restore explicit configured ports.
- [x] Decide whether to restore `portFlag`.
- [x] Keep `host` mapping to `<host>.localhost`.
- [x] Keep `working_dir`.
- [x] Keep `auto_start`.
- [x] Do not migrate old JSON config.
- [x] Remove old `config.json` migration path.
- [x] Keep TOML as the only source config format.
- [x] Keep normal load from source TOML read-only.
- [x] Keep generated/backfilled cache separate if needed.
- [x] Add validation for any restored fields.
- [x] Add config write/update APIs.
- [x] Preserve nice TOML formatting when writing.
- [x] Expose config path over FFI/control API.
- [ ] Expose config parse/validation errors to UI.

## Service runtime

- [x] Add service add API.
- [x] Add service edit API.
- [x] Add service remove API.
- [ ] Add service duplicate API if wanted.
- [x] Add service restart API.
- [x] Add restart to FFI.
- [x] Add restart to Swift client.
- [x] Add restart to view model.
- [x] Add restart App Intent.
- [x] Add failed service state.
- [x] Add readiness timeout.
- [x] Avoid services stuck in `Starting` forever.
- [x] Surface readiness failure message.
- [ ] Track process exit reason where possible.
- [x] Track process group cleanly.
- [ ] Keep registry as source of truth.
- [ ] Treat registry/process-handle disagreement as internal desync error.
- [ ] Do not silently paper over impossible states.
- [ ] Register proxy origin when service starts if pending-route behaviour is desired.
- [ ] Register destination when port is detected.
- [ ] Deregister destination on stop/failure.
- [ ] Add pending route/waiting behaviour if desired.
- [ ] Preserve good UX for opening service URL immediately after clicking start.
- [ ] Restore or replace login-shell env loading.
- [x] Merge current process env.
- [x] Add per-service env overrides if config supports them.
- [x] Keep colour env vars for terminal output.
- [x] Decide whether to inject `PORT`.
- [x] Decide whether to inject `HOST`.
- [x] Decide whether to append old `portFlag`.
- [ ] Decide whether `rack-bridge` / Unix socket launch path is still needed.
- [ ] Add Unix socket backend support if needed.
- [x] Keep local TCP-only model if that is enough.
- [x] Add auto-start services on runtime init.
- [x] Make stop kill full process group reliably.
- [x] Make logs truncate on start/restart.
- [x] Make logs append stdout/stderr chunks safely.
- [ ] Add service runtime tests.

## Service UI/settings

- [x] Add add-service form from scratch.
- [x] Add edit-service form from scratch.
- [x] Add remove confirmation.
- [ ] Add duplicate action if kept.
- [ ] Add per-service detail panel from scratch.
- [x] Add fields for name.
- [x] Add field for host.
- [x] Add field for run command.
- [x] Add field for working directory.
- [x] Add field for auto-start.
- [x] Add environment variable editor if env is restored.
- [x] Add custom domain field if custom domains are restored.
- [x] Add explicit port field if explicit ports are restored.
- [x] Add command arguments UI if args are restored.
- [ ] Add validation messages.
- [x] Add save/cancel flow.
- [ ] Add dirty-state handling.
- [ ] Add recent output in detail panel.
- [ ] Add open full logs action.
- [ ] Add copy URL.
- [ ] Add open URL.
- [ ] Add start/stop/restart actions.

## Proxy

- [x] Keep Pingora.
- [x] Keep folder structure clean: `server/`, `services/`, `hooks/` style.
- [x] Keep proxy focused on routing.
- [x] Keep service lifecycle out of proxy.
- [x] Let services register origins/destinations with proxy.
- [x] Let hooks package dispatch hook requests.
- [ ] Decide whether nested subdomain fallback is required.
- [ ] Support `fix-auth.myapp.localhost -> myapp` if required.
- [x] Otherwise document nested localhost rejection.
- [x] Add backend wait/retry while service is starting.
- [x] Restore loopback proxy loop detection.
- [x] Return useful `508 Loop Detected` response when needed.
- [ ] Add clear error for unknown service host.
- [ ] Add clear error for service known but not running.
- [ ] Add clear error for backend connection failure.
- [x] Verify WebSocket frame tunnelling, not just upgrade headers.
- [ ] Add explicit WebSocket tunnel if Pingora path is insufficient.
- [ ] Test Vite HMR.
- [ ] Test Bun dev server websockets.
- [ ] Test Next dev server websockets if relevant.
- [x] Bind IPv4 loopback.
- [ ] Bind IPv6 loopback if main parity requires it.
- [ ] Restore HTTPS listener if required.
- [ ] Restore local cert generation/trust if required.
- [ ] Restore standard port forwarding if required.
- [ ] Restore privileged relay/helper if required.
- [ ] Restore `/etc/hosts` management for `rack.local` if required.
- [ ] Wire `use_standard_ports` TOML setting to actual behaviour.
- [ ] Add proxy tests for host parsing.
- [ ] Add proxy tests for unknown hosts.
- [ ] Add proxy tests for starting-service wait.
- [ ] Add proxy tests for loop detection.
- [ ] Add proxy tests for websocket frame tunnel.
- [ ] Add proxy tests for IPv6 if supported.
- [ ] Add proxy tests for HTTPS if supported.

## Hooks runtime

- [x] Keep embedded WASM metadata as source of truth.
- [x] Do not go back to mandatory `manifest.toml`.
- [ ] Add package identity metadata if needed.
- [ ] Add package version metadata if needed.
- [ ] Add route IDs if needed.
- [ ] Add cron IDs if needed.
- [ ] Add hook load error summaries.
- [ ] Show metadata parse errors in UI/CLI.
- [x] Validate required exports at load/deploy time.
- [x] Validate required memory export.
- [x] Validate alloc/dealloc exports.
- [x] Validate route entry exports.
- [x] Validate cron entry exports.
- [x] Add timeout/fuel/epoch interruption for in-process Wasmtime.
- [x] Prevent hanging hooks from blocking runtime threads forever.
- [x] Harden guest pointer/length handling.
- [x] Use checked arithmetic for pointer + length.
- [x] Handle negative guest lengths safely.
- [ ] Review `rack_alloc` / `rack_dealloc` ABI pairing.
- [ ] Add malformed guest output tests.
- [ ] Add hook started/finished/duration logging.
- [ ] Add hook error logging.
- [ ] Add hook stdout/stderr capture or structured logging.
- [ ] Write hook logs under `~/.rack/logs/hooks/` or chosen equivalent.
- [ ] Surface hook failures in UI.
- [ ] Surface hook failures in CLI.

## Hook HTTP routing

- [x] Normalize leading slash.
- [ ] Decide trailing slash semantics.
- [x] Reject `/` if still reserved.
- [x] Reject `/_*` if still reserved.
- [ ] Add glob route support if still needed.
- [ ] Choose most specific route.
- [x] Detect equal-specificity conflicts.
- [x] Return useful route conflict errors.
- [ ] Inject route metadata into request.
- [ ] Include package in request metadata.
- [ ] Include route id in request metadata.
- [ ] Include route pattern in request metadata.
- [ ] Include matched path in request metadata.
- [ ] Align no-route 404 behaviour.
- [ ] Align conflict 409 behaviour if applicable.
- [x] Add route normalization tests.
- [ ] Add glob tests.
- [x] Add route conflict tests.
- [ ] Add route metadata tests.

## Hook SDK

- [ ] Decide whether zero-arg `#[rack::route]` compatibility is required.
- [ ] Decide whether zero-arg `#[rack::cron]` compatibility is required.
- [x] Keep new `#[rack::route(GET, "path")]` syntax.
- [x] Keep new `#[rack::cron("every 5 minutes")]` syntax.
- [x] Add/restore `#[rack::payload]` if needed.
- [x] Add `Payload` trait.
- [x] Add typed `Request<T>`.
- [x] Add JSON body parsing.
- [x] Add `String` payload support.
- [x] Add `()` payload support.
- [x] Restore handler `Result<Response>` support.
- [x] Restore `?` support in handlers.
- [x] Map bad payload to 400.
- [ ] Map handler errors to 500.
- [x] Expose request method.
- [x] Expose request path.
- [x] Expose full URI/query.
- [x] Expose host.
- [x] Expose headers.
- [x] Expose `header(name)` helper.
- [x] Expose body.
- [ ] Expose route metadata.
- [x] Validate response status `100..=599`.
- [ ] Normalize response headers.
- [ ] Decide duplicate response header behaviour.
- [x] Add response helpers: `ok`.
- [x] Add response helpers: `created`.
- [x] Add response helpers: `bad_request`.
- [x] Add response helpers: `teapot`.
- [x] Add response helpers: `server_error`.
- [x] Add response builder `.text()`.
- [x] Add response builder `.html()`.
- [x] Add response builder `.csv()`.
- [x] Add response builder `.json()`.
- [x] Add response builder `.bytes()`.
- [x] Align text content type/charset with main.
- [x] Restore `rack::log::info`.
- [x] Restore `rack::log::warn`.
- [x] Restore `rack::log::error`.
- [ ] Restore `rack::fs!` if still part of public SDK.
- [x] Add SDK compile tests.
- [ ] Add macro invalid-signature tests.

## Cron hooks

- [x] Keep interval schedules working.
- [x] Add calendar schedule parser if required.
- [x] Support `friday at 17:00` if required.
- [x] Support `weekdays at 9:30am` if required.
- [x] Port/reuse main schedule semantics where useful.
- [x] Add `CronEvent` argument support.
- [x] Include package in cron event.
- [x] Include hook id in cron event.
- [x] Include schedule in cron event.
- [x] Include scheduled-at timestamp in cron event.
- [ ] Log cron started.
- [ ] Log cron finished.
- [ ] Log cron errors.
- [ ] Decide first-run semantics.
- [x] Add interval schedule tests.
- [x] Add calendar schedule tests.
- [x] Add cron event payload tests.

## Hook CLI

- [x] Add `rack hook list`.
- [x] Add `rack hook ls` alias.
- [x] Add `rack hook remove`.
- [x] Add `rack hook rm` alias.
- [x] Add `rack hook uninstall` alias if useful.
- [x] Add `rack hook test`.
- [x] Add `rack hook install` if deploy should not cover it.
- [x] Add `rack hook compile` alias if useful.
- [ ] Add `rack fn ...` compatibility group if desired.
- [ ] Add `rack hook deploy --replace`.
- [ ] Add non-destructive copy install mode if desired.
- [ ] Make deploy move/symlink behaviour explicit in help text.
- [ ] Allow `hook init` with omitted path if desired.
- [ ] Allow `hook init` into existing empty directory.
- [ ] Reject non-empty init target clearly.
- [ ] Sanitize package names robustly.
- [ ] Ensure wasm target is installed or print actionable error.
- [ ] Use `cargo metadata` to discover package/cdylib output.
- [ ] Validate embedded metadata during build/deploy.
- [ ] Show deployed hooks/routes/crons/errors in list output.
- [ ] Add hook CLI tests.

## Hooks UI

- [x] Add Hooks Settings page from scratch.
- [x] Show deployed hooks.
- [x] Show HTTP routes.
- [x] Show cron hooks.
- [x] Show hook load errors.
- [ ] Show route conflicts.
- [ ] Show hook logs if available.
- [x] Add open hook directory action if useful.
- [x] Add remove deployed hook action if safe.
- [x] Add reload hooks action only when runtime reload is real.
- [x] Reload should update proxy registry.
- [x] Reload should update cron scheduler.
- [x] Reload should update UI summaries.

## CLI services

- [x] Decide whether to keep top-level `rack ls`.
- [x] Decide whether to keep top-level `rack start <name>`.
- [x] Decide whether to keep top-level `rack stop <name>`.
- [x] Decide whether to keep top-level `rack rm <name>`.
- [x] Support service lookup by ID.
- [x] Support service lookup by name if useful.
- [x] Support service lookup by host if useful.
- [x] Add `rack service edit`.
- [x] Add `rack service restart`.
- [ ] Add `rack service duplicate` if useful.
- [x] Improve `rack service list` URLs.
- [x] Include proxy port in URL when not using standard ports.
- [x] Show starting/running/stopped/failed state.
- [x] Show detected backend port(s).
- [ ] Add machine-readable output if useful.
- [ ] Align control socket path with app.
- [ ] Document `RACK_CONTROL_SOCKET`.

## `rack dev`

- [ ] Reimplement `rack dev` if still wanted.
- [ ] Detect project type.
- [ ] Infer service name.
- [ ] Infer host.
- [ ] Infer run command.
- [ ] Register service with running Rack app.
- [ ] Start service.
- [ ] Adapt behaviour to no-explicit-port model.
- [ ] Add tests for project detection.

## FFI / bridge

- [x] Keep bespoke bridge; do not switch to UniFFI.
- [x] Keep per-package `ffi/` folders where useful.
- [x] Keep FFI functions thin.
- [x] FFI should call native package functions and nothing else.
- [x] Standardize struct layout/order.
- [x] Keep ABI version checks.
- [x] Keep struct size checks.
- [x] Add config mutation FFI.
- [x] Add restart FFI.
- [x] Add failed-state FFI.
- [x] Add config path FFI.
- [x] Add hook reload FFI when real reload exists.
- [ ] Add hook summaries/errors FFI improvements.
- [x] Avoid returning dangling pointers.
- [x] Clearly define ownership for Rust-allocated memory.
- [x] Clearly define ownership for Swift-provided buffers.
- [ ] Add tests or compile-time checks where possible.
- [x] Keep Swift ABI declarations grouped and small.
- [x] Keep safe Swift client separate from raw ABI.

## Standard ports / DNS / local networking

- [ ] Decide if standard ports are v1 scope.
- [ ] Decide if HTTPS is v1 scope.
- [ ] Decide if local cert trust is v1 scope.
- [ ] Decide if `/etc/hosts` management is v1 scope.
- [ ] Restore privileged helper/relay if needed.
- [ ] Add install/uninstall flow for helper.
- [ ] Add standard port status.
- [ ] Add standard port errors.
- [ ] Wire `use_standard_ports` config to runtime.
- [ ] Add Settings toggle.
- [ ] Add docs for permissions.
- [ ] Add tests where practical.

## Docs

- [x] Update README for rewrite architecture.
- [x] Document package layout.
- [x] Document TOML config path.
- [x] Document services config format.
- [x] Document that legacy JSON config is not migrated.
- [x] Document service no-explicit-port model.
- [x] Document proxy host model.
- [x] Document `rack.local` hooks.
- [x] Document hook SDK.
- [x] Document proc macro metadata.
- [x] Document hook deploy layout.
- [x] Document CLI service commands.
- [x] Document CLI hook commands.
- [x] Document App Intents/Shortcuts.
- [x] Document URL scheme.
- [ ] Document standard ports/DNS if restored.
- [x] Document logs paths.

## Tests

- [x] Remove core config migration tests.
- [x] Add core config write/update tests.
- [x] Add core validation tests for any restored fields.
- [ ] Add services lifecycle tests.
- [ ] Add services restart tests.
- [x] Add services failed readiness tests.
- [x] Add services readiness timeout tests.
- [ ] Add services env merge tests.
- [ ] Add services log tests.
- [ ] Add services FFI ABI tests.
- [ ] Add proxy host tests.
- [x] Add proxy backend wait tests.
- [x] Add proxy loop detection tests.
- [x] Add proxy websocket frame tests.
- [ ] Add proxy IPv6 tests if supported.
- [ ] Add proxy HTTPS tests if supported.
- [x] Add hooks route normalization tests.
- [ ] Add hooks glob tests.
- [x] Add hooks conflict tests.
- [x] Add hooks reserved path tests.
- [x] Add hooks request metadata tests.
- [x] Add hooks response normalization tests.
- [ ] Add hooks timeout tests.
- [x] Add hooks malformed guest ABI tests.
- [ ] Add hooks cron interval tests.
- [x] Add hooks cron calendar tests.
- [ ] Add hooks logging tests.
- [ ] Add CLI service integration tests.
- [x] Add CLI hook lifecycle tests.
- [ ] Add Swift/manual QA checklist.
- [ ] Test menu opens and displays services.
- [ ] Test start/stop/restart from menu.
- [ ] Test logs preview colours.
- [ ] Test open logs in configured terminal.
- [ ] Test Settings opens.
- [ ] Test add/edit/remove service.
- [ ] Test launch-at-login toggle.
- [ ] Test App Intents before menu opens.
- [ ] Test URL scheme actions.
- [ ] Test quit cleanup.

## Build / release

- [x] Add CI for `cargo test`.
- [x] Add CI for `make swift`.
- [x] Add formatting checks.
- [ ] Add release packaging smoke test.
- [x] Ensure app bundle layout is correct.
- [x] Ensure Rust artifacts are included correctly.
- [x] Ensure bundled CLI works.
- [x] Ensure CLI installer works.
- [ ] Ensure app can run from `/Applications`.
- [ ] Ensure launch-at-login works from built app bundle.
- [ ] Ensure app sandbox/signing story is clear if applicable.

## Cleanup

- [ ] Decide what to do with untracked `packages/hooks/src/std/`.
- [x] Remove stale old `standardPortsEnabled` AppStorage if replaced by TOML.
- [ ] Remove stale old settings assumptions.
- [ ] Remove any dead FFI hodge-podge once bridge is cleaned.
- [ ] Split any large files that drift past maintainable size.
- [ ] Keep proxy folders nested by responsibility.
- [ ] Keep services folders nested by responsibility.
- [ ] Keep hooks folders nested by responsibility.
- [ ] Keep tests out of `src`.

## Suggested next order

- [x] Build Settings shell from scratch.
- [x] Add config/service mutation APIs.
- [x] Add service add/edit/remove UI.
- [x] Add lifecycle owner/AppDelegate.
- [x] Add URL scheme.
- [x] Add restart end-to-end.
- [x] Add service failed/readiness states.
- [x] Add hook list/remove/test CLI.
- [x] Fill hook SDK request/response gaps.
- [x] Add cron calendar/event parity.
- [x] Verify/fix WebSocket proxying.
- [ ] Decide standard ports/HTTPS scope.
- [x] Add docs/release polish.
