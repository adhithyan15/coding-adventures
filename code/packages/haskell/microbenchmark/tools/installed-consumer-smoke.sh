#!/bin/sh
set -eu

package_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
consumer_environment="$temporary/consumer.ghc.environment"
cabal install --lib "--package-env=$consumer_environment" ../rank-statistics .
GHC_ENVIRONMENT="$consumer_environment" ghc \
  -outputdir "$temporary/objects" \
  -o "$temporary/installed-consumer" \
  "$package_root/tools/InstalledConsumer.hs"
GHC_ENVIRONMENT="$consumer_environment" "$temporary/installed-consumer"
