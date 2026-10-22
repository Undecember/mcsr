#!/usr/bin/env bash
set -euo pipefail

if command -v keyd.rvaiya >/dev/null 2>&1; then
  keyd_bin=keyd.rvaiya
elif command -v keyd >/dev/null 2>&1; then
  keyd_bin=keyd
else
  echo 'keyd: missing'
  exit 1
fi

echo "session: ${XDG_SESSION_TYPE:-unknown} / ${XDG_CURRENT_DESKTOP:-unknown}"
gnome-shell --version
"$keyd_bin" --version
if "$keyd_bin" --help 2>&1 | rg -q 'check \['; then
  echo 'keyd config check: available'
else
  echo 'keyd config check: unavailable'
fi
if command -v keyd-application-mapper >/dev/null 2>&1; then
  echo 'keyd application mapper: available'
else
  echo 'keyd application mapper: unavailable'
fi
if id -nG | tr ' ' '\n' | rg -qx keyd; then
  echo 'keyd group: active'
else
  echo 'keyd group: not active for this login'
fi
gnome-extensions info mcsr-toolkit-probe@local 2>&1 || true
