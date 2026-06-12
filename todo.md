# Rack.

## resilience
- [x] Crash notifications (UNUserNotificationCenter)
- [x] Auto-restart with backoff (3 tries, then notify and stop)

## control
- [ ] Server groups (start/stop/restart cluster)
- [ ] AppleScript suite (start/stop/restart/status)
- [ ] Unix socket API (`~/.config/rack/control.sock`, JSON-RPC-ish)
- [ ] `rack` CLI binary (add/import/start/stop via shell)

## distribution
- [ ] Auto-updater (GitHub Releases API → download `.zip` → replace app → relaunch)
- [ ] Homebrew Cask formula (`jafupy/rack/rack`)

## config
- [ ] `.rack` scaffold import (slurp into global config)

## infra (maybe never)
- [ ] Tunneling (BYO relay server or $2 sub)
- [ ] libghostty embed (evaluate if worth the weight)
- [ ] Per-server HTTPS (local CA, Keychain install)
