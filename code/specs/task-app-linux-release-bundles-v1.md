# TaskApp portable Linux release bundles v1

Issue: [#13611](https://github.com/adhithyan15/coding-adventures/issues/13611)

## Decision

TaskApp releases publish one verified Linux x86_64 application archive for each
strict native backend already gated on Linux: Qt, Flutter, and Compose Desktop.
These payloads are called **portable bundles**. They are not called installers,
distribution packages, or universally self-contained binaries.

The source-project ZIP remains available beside each bundle. That keeps the
generated Mosaic output inspectable while giving a user a direct launch path.

## Payload contract

Each `task-app-<backend>-linux-bundle-v<semver>.tar.gz` has one root directory
and contains the complete application tree produced by the backend's release
build plus four release-owned files:

- `SOURCE_COMMIT` — the exact 40-character source commit;
- `BUNDLE.json` — product, version, platform, backend, application identity,
  executable, Rust runtime, state path, and launcher;
- `INSTALL.txt` — honest compatible-system prerequisites and launch steps;
- `launch-trestle` — a POSIX entrypoint that can be invoked from any working
  directory and never requires `MOSAIC_APP_LIBRARY`.

The archive builder rejects an executable or runtime outside the selected bundle
root, an external symlink target, and any Rust runtime that is not byte-identical
to the selected `task-mosaic-app` release artifact.

## Local state and the pre-upgrade snapshot

The Mosaic application identity remains `task-app`. Flutter and Compose store the
live snapshot at `$XDG_DATA_HOME/task-app/mosaic-state.v1.json` (falling back to
`~/.local/share`). Qt's `QStandardPaths::AppDataLocation`, with the generated
`TaskApp` executable name, places it under
`$XDG_DATA_HOME/TaskApp/task-app/mosaic-state.v1.json`.

Before launch, `launch-trestle` copies an existing live snapshot once to:

```text
$XDG_DATA_HOME/task-app/backups/pre-v<VERSION>-<BACKEND>.json
```

The copy uses a temporary file followed by an atomic rename and never overwrites
an existing snapshot for that version/backend pair. Backup/restore UX and the
cross-platform recovery contract remain the focused follow-up in #13614.

## Release verification

The release workflow must, for each backend:

1. build `task-mosaic-app` in release mode;
2. generate the project with `--profile native-complete` and that runtime;
3. build the backend's release/distributable tree;
4. compare its installed `libmosaic_app.so` byte-for-byte with the selected Rust
   artifact;
5. create and re-extract the `.tar.gz` payload;
6. launch the extracted `launch-trestle` from `/`, without a runtime override,
   under the backend-appropriate headless display; and
7. include the bundle in the exact-payload manifest and `SHA256SUMS`.

Qt and Flutter bundles retain compatible system-library prerequisites disclosed
in `INSTALL.txt`. Compose includes its generated JVM runtime but still targets a
compatible glibc-based Linux x86_64 system. Distro-native packages, signing, and
update channels are future work.
