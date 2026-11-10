#!/bin/bash
# Build, sign, install and launch the game on a connected iPhone.
# Usage: scripts/phone.sh [phone name or UDID]
# Signing: the EAS ad hoc profile and distribution certificate in release/credentials
# (get them with `eas credentials -p ios` in release/, then "Download credentials").
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PROFILE=${EXACT_PROFILE:-$ROOT/release/credentials/ios/adhoc.mobileprovision}
SIGNING="$HOME/Library/Keychains/exact-beat-em-up-signing.keychain-db"
[ -f "$PROFILE" ] || { echo "No provisioning profile at $PROFILE"; exit 1; }
if [ -f "$SIGNING" ]; then
  security unlock-keychain -p "$(security find-generic-password -s exact-beat-em-up-signing -a codesign -w)" "$SIGNING"
fi
IDENTITY=${EXACT_IDENTITY:-$(security find-identity -v -p codesigning "$SIGNING" | awk '/iPhone Distribution/ { print $2; exit }')}
[ -n "$IDENTITY" ] || { echo "No iPhone Distribution identity in $SIGNING"; exit 1; }

cd "$ROOT"
export EXACT_IDENTITY=$IDENTITY EXACT_PROFILE=$PROFILE
if [ -n "${1:-}" ]; then
  bun exact.mjs ios --device --phone "$1" --run
else
  bun exact.mjs ios --device --run
fi
