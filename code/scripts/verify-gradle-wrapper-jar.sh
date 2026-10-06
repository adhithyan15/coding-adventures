#!/usr/bin/env bash
# verify-gradle-wrapper-jar.sh -- refuse a gradle-wrapper.jar Gradle did not publish.
#
# Usage: verify-gradle-wrapper-jar.sh <path/to/gradle-wrapper.jar>
#
# Why this exists
# ---------------
# CI writes the Mosaic Android project's wrapper by running `gradle wrapper`
# with whatever Gradle the runner has installed, then copies the resulting
# gradle-wrapper.jar into the project and runs it (never the unverifiable
# gradlew script, for which Gradle publishes no checksum). That jar is the
# first code to execute in the build: it downloads the Gradle distribution and
# checks it against gradle-wrapper.properties' distributionSha256Sum. So the
# distribution pin is only as trustworthy as the jar enforcing it. A tampered
# jar could skip the check.
#
# How the check works
# -------------------
# The jar `gradle wrapper` writes is byte-for-byte the wrapper jar of the
# Gradle version that RAN the task (not the version it names), and Gradle
# publishes that jar's SHA-256 for every release:
#
#     https://services.gradle.org/distributions/gradle-<version>-wrapper.jar.sha256
#
# This is the same list gradle/actions/wrapper-validation checks against. So:
#
#     installed Gradle version  ->  published checksum  ==  sha256(jar) ?
#
#     jar from a real Gradle 9.1.0   matches 9.1.0's published sum   -> exit 0
#     jar edited or swapped          matches nothing                 -> exit 1
#     checksum unreachable/garbled   cannot prove anything           -> exit 1
#
# It fails CLOSED: if the published checksum cannot be fetched, the build stops
# rather than trusting an unverified jar.
#
# The checksum is fetched over HTTPS only, redirects included: the published
# sum is the one thing the jar is judged against.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <path/to/gradle-wrapper.jar>" >&2
  exit 2
fi
jar="$1"
if [[ ! -f "$jar" ]]; then
  echo "::error::no gradle-wrapper.jar at $jar" >&2
  exit 1
fi

# The installed Gradle's version, e.g. "9.1.0". `gradle --version` prints a
# line "Gradle 9.1.0". The output goes to a file before it is parsed: a reader
# that stops at its first match would close the pipe early, and under pipefail
# that fails the step (see the lesson on `nm | grep -q`).
version_report="$(mktemp)"
trap 'rm -f "$version_report"' EXIT
gradle --no-daemon --version > "$version_report"
version="$(sed -n 's/^Gradle \([0-9][0-9A-Za-z.+-]*\)$/\1/p' "$version_report" | sed -n 1p)"
if [[ ! "$version" =~ ^[0-9]+(\.[0-9]+)+([.-][0-9A-Za-z.+-]+)?$ ]]; then
  echo "::error::could not read the installed Gradle version from \`gradle --version\`" >&2
  exit 1
fi

# Gradle's published checksum for that version's wrapper jar. Retried, because
# services.gradle.org has been briefly unreachable from runners before. A
# transient failure should not fail the build when a retry would succeed.
url="https://services.gradle.org/distributions/gradle-${version}-wrapper.jar.sha256"
if ! expected="$(curl --proto =https --proto-redir =https --tlsv1.2 --fail --silent --show-error --location --retry 5 --retry-all-errors --retry-delay 3 "$url" | tr -d '[:space:]')"; then
  echo "::error::could not fetch $url" >&2
  exit 1
fi
if [[ ! "$expected" =~ ^[0-9a-f]{64}$ ]]; then
  echo "::error::$url did not return a SHA-256" >&2
  exit 1
fi

# This file's own SHA-256: sha256sum on Linux, shasum on macOS.
if command -v sha256sum > /dev/null; then
  actual="$(sha256sum -- "$jar" | cut -d' ' -f1)"
else
  actual="$(shasum -a 256 -- "$jar" | cut -d' ' -f1)"
fi

if [[ "$actual" != "$expected" ]]; then
  echo "::error::$jar is not Gradle ${version}'s published wrapper jar (sha256 $actual, published $expected)" >&2
  exit 1
fi
echo "gradle-wrapper.jar matches Gradle ${version}'s published checksum ($expected)"
