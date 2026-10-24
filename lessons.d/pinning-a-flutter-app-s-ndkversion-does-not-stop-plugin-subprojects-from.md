---
category: Cross-platform & Windows BUILD_windows
---

# Pinning a Flutter app's ndkVersion does not stop plugin subprojects from downloading Flutter's NDK mid-build

X-7 (#17458) stopped a corrupt NDK download ("Archive is not a ZIP
archive") failing the Flutter Android build. It rewrote
`android/app/build.gradle.kts` to use the runner's preinstalled NDK. On PR
#17914 the same error came back, this time configuring project `:jni`.
path_provider_android depends on package:jni, which builds C code through
CMake with `ndkVersion = flutter.ndkVersion`. Flutter declares that
read-only (`val ndkVersion: String = "28.2.13676358"` in
FlutterExtension.kt), so the app can't redirect its plugins to another NDK.
The Android Gradle plugin then downloaded about 1 GB in the middle of the
build, once, with no retry.

**Fix:** `code/scripts/ensure-flutter-android-ndk.sh` reads the pinned
version from the Flutter SDK. If the NDK is missing, it installs it with
sdkmanager, up to three times with backoff. It accepts the result only when
`ndk/<v>/source.properties` names that exact version, because sdkmanager can
just warn about a corrupt archive and still exit 0. Tests drive it with a
fake sdkmanager (`test_ensure_flutter_android_ndk.py`).

**Do differently:** an app-level setting doesn't reach a plugin's own Gradle
project. When a download fails in a Gradle subproject, check which project
was being configured (`:jni`, not `:app`) before deciding which build file
to change. For a large toolchain download, install it up front with a
verified, retried step rather than leaving it to the build tool.
