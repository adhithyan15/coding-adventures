#!/usr/bin/env bash
# Build a generated Mosaic Android project's debug APK through a Gradle
# wrapper jar that is checked against Gradle's published checksum first
# (UI89 §3.5, §3.9).
#
#   assemble-mosaic-android-debug.sh <android-project> [--with-android-test]
#
# The generated project names its Gradle in
# gradle/wrapper/gradle-wrapper.properties but ships no wrapper jar or
# scripts. So:
#
#   1. the jar is written by whatever Gradle is installed, in an empty
#      directory, so that Gradle never configures the Android plugin it may
#      not support;
#   2. verify-gradle-wrapper-jar.sh refuses a jar that is not the one Gradle
#      published for that version;
#   3. the verified jar itself runs `assembleDebug`, with gradlew's JVM
#      options, rather than a generated gradlew script: Gradle publishes no
#      checksum for the script, so it is the one part of the wrapper nothing
#      can verify. Naming the main class (not `-jar`) works with every
#      Gradle's wrapper jar, including old ones without a Main-Class.
#
# With --with-android-test it also builds the instrumented test APK
# (`assembleDebugAndroidTest`, UI89 §4.2) from whatever the caller put under
# <android-project>/src/androidTest. The app APK is the same either way.
#
# The APK lands in <android-project>/build/outputs/apk/debug, the test APK in
# <android-project>/build/outputs/apk/androidTest/debug. Needs `gradle`
# on PATH, a JDK (JAVA_HOME or `java` on PATH) and the Android SDK
# (ANDROID_HOME) for the project's own build.
set -euo pipefail

usage="usage: $0 <android-project> [--with-android-test]"
tasks=(assembleDebug)
case $# in
  1) ;;
  2)
    if [[ "$2" != "--with-android-test" ]]; then
      echo "$usage" >&2
      exit 2
    fi
    tasks+=(assembleDebugAndroidTest)
    ;;
  *)
    echo "$usage" >&2
    exit 2
    ;;
esac
project="$1"
properties="$project/gradle/wrapper/gradle-wrapper.properties"
if [[ ! -f "$properties" ]]; then
  echo "no $properties: not a generated Mosaic Android project" >&2
  exit 2
fi

# The version the project pins, e.g. 8.14.3 from
# `distributionUrl=https\://services.gradle.org/distributions/gradle-8.14.3-bin.zip`.
gradle_version="$(sed -n 's|^distributionUrl=.*/gradle-\(.*\)-bin\.zip$|\1|p' "$properties")"
# It reaches a command line below, so it must look like a version.
if [[ ! "$gradle_version" =~ ^[0-9]+(\.[0-9]+)*(-[A-Za-z0-9.]+)?$ ]]; then
  echo "could not read a Gradle version from $properties" >&2
  exit 2
fi
# The wrapper checks the Gradle it downloads only against a pinned
# checksum; without one it would run whatever the URL served.
if ! grep -Eq '^distributionSha256Sum=[0-9a-f]{64}$' "$properties"; then
  echo "$properties pins no distributionSha256Sum" >&2
  exit 2
fi

here="$(CDPATH='' cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" > /dev/null && pwd -P)"
seed="$(mktemp -d)"
trap 'rm -rf -- "$seed"' EXIT
touch "$seed/settings.gradle.kts"
gradle --no-daemon -q -p "$seed" wrapper --gradle-version "$gradle_version"
cp "$seed/gradle/wrapper/gradle-wrapper.jar" "$project/gradle/wrapper/gradle-wrapper.jar"
bash "$here/verify-gradle-wrapper-jar.sh" "$project/gradle/wrapper/gradle-wrapper.jar"

cd -- "$project"
"${JAVA_HOME:+$JAVA_HOME/bin/}java" -Xmx64m -Xms64m -Dorg.gradle.appname=gradlew \
  -cp gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain \
  --no-daemon --stacktrace "${tasks[@]}"
