#!/bin/bash
# Build the signed App Store archive and submit it to TestFlight with EAS Submit.
# Usage: scripts/testflight.sh <build number> [version]
# EAS Build cannot build an Exact app, so the .ipa is built here. Signing: the App
# Store profile in release/credentials (get it with `eas credentials -p ios` in
# release/, profile "production", then "Download credentials") and the distribution
# certificate in the keychain exact-beat-em-up-signing.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BUILD=${1:?usage: scripts/testflight.sh <build number> [version]}
VERSION=${2:-$(node -p 'require(process.argv[1]).expo.version' "$ROOT/release/app.json")}
PROFILE=${EXACT_PROFILE:-$ROOT/release/credentials/ios/appstore.mobileprovision}
SIGNING="$HOME/Library/Keychains/exact-beat-em-up-signing.keychain-db"
export PATH="$HOME/.cargo/bin:$PATH"

[ -f "$PROFILE" ] || { echo "No App Store profile at $PROFILE"; exit 1; }
if [ -f "$SIGNING" ]; then
  security unlock-keychain -p "$(security find-generic-password -s exact-beat-em-up-signing -a codesign -w)" "$SIGNING"
fi
IDENTITY=${EXACT_IDENTITY:-$(security find-identity -v -p codesigning "$SIGNING" | awk '/iPhone Distribution/ { print $2; exit }')}
[ -n "$IDENTITY" ] || { echo "No iPhone Distribution identity in $SIGNING"; exit 1; }

IPA="$ROOT/.exact/release/Rooftop-Rumble-$VERSION-$BUILD.ipa"
mkdir -p "$(dirname "$IPA")"
(cd "$ROOT" && EXACT_UPDATE_TRUST=production EXACT_IDENTITY=$IDENTITY EXACT_PROFILE=$PROFILE \
  EXACT_VERSION=$VERSION EXACT_BUILD_NUMBER=$BUILD EXACT_NON_EXEMPT_ENCRYPTION=false \
  bun exact.mjs ios --device --archive "$IPA")

(cd "$ROOT/release" && eas submit -p ios --profile production --path "$IPA" --non-interactive --wait)
echo "Apple processes the build next. Check it with:"
echo "  (cd release && eas submit:status -p ios --profile production --json --non-interactive)"
