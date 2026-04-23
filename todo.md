# Rack.

## networking
- [ ] DNS responder (UDP 53535, hand-rolled, `.r` only)
- [ ] IP pool (`127.255.255.1`–`.254`, step down on exhaust)
- [ ] `lo0` alias mgmt + `/etc/resolver/r` setup (SMJobBless or osascript fallback)
- [ ] Inject `RACK_HOST`/`RACK_PORT` into spawned processes

## resilience
- [ ] Crash notifications (UNUserNotificationCenter)
- [ ] Auto-restart with backoff (3 tries, then notify and stop)

## control
- [ ] Server groups (start/stop/restart cluster)
- [ ] AppleScript suite (start/stop/restart/status)
- [ ] Unix socket API (`~/.config/rack/control.sock`, JSON-RPC-ish)
- [ ] `rack` CLI binary (add/import/start/stop via shell)

## config
- [ ] `.rack` scaffold import (slurp into global config)

## infra (maybe never)
- [ ] Tunneling (BYO relay server or $2 sub)
- [ ] libghostty embed (evaluate if worth the weight)
- [ ] Per-server HTTPS (local CA, Keychain install)
