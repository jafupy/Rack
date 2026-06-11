#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_TARGET="$ROOT_DIR/dist/Rack.app"
APP_LINK="/Applications/Rack.app"

if [[ ! -d "$APP_TARGET" ]]; then
  echo "Missing $APP_TARGET. Run ./scripts/build-app.sh first." >&2
  exit 1
fi

if [[ -e "$APP_LINK" && ! -L "$APP_LINK" ]]; then
  echo "$APP_LINK exists and is not a symlink. Move it aside before shimming." >&2
  exit 1
fi

ln -sfn "$APP_TARGET" "$APP_LINK"
echo "$APP_LINK -> $APP_TARGET"
