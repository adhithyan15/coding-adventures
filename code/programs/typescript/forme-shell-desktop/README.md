# Forme Desktop

`forme-shell-desktop` is the installable Tauri v2 host for Forme's first-run
authoring product. The webview receives the capability-free FM09 React shell;
the Rust process owns project storage, identity creation, exact product builds,
loopback preview, publication targets, and workspace retirement.

The FM-B068 product boundary targets macOS. Its worker is confined with
Seatbelt and process resource limits, and local publication commits use macOS
atomic directory exchange. Another platform is not supported until it has an
equivalent reviewed sandbox, directory-identity model, and atomic publication
primitive.

## Authority boundary

The main window has no generic filesystem, HTTP, shell, process, environment,
opener, or dialog permission. It can invoke only the application's closed
commands. Project paths and publication destinations never cross IPC. Native
storage uses a fixed application-data child, compare-and-swap revisions,
same-directory atomic replacement, complete FM09 state validation, and
owner-only permissions. Native authority roots and their ancestors also reject
macOS extended ACL entries that grant mutating access despite restrictive mode
bits.

The product pipeline is `authoring snapshot -> router -> static renderer ->
filesystem emitter`. It is constructed by the real Forme orchestrator with the
reviewed classless theme and reproducible-build settings. A version-pinned
single-executable Node worker bundles that complete dependency closure. Its
single-record protocol returns only revision-bound manifest, size, and digest
metadata; Rust copies it into a private per-run workspace, rechecks its
identity, launches it inside a network- and subprocess-denied sandbox, and
rechecks every contained output byte.
Preview and publication therefore cannot drift onto a second renderer.

The first workspace open asks for an empty local publication folder when no
reviewed target exists. Only an opaque target identity and a basename summary
reach the webview. Before each publish the native host rechecks the directory
identity, owner/mode/ancestor safety, bounded same-device contents, and
ownership marker; rebuilds the exact saved revision; stages through pinned
directory descriptors; flushes the complete tree; and swaps it atomically.
Descriptor-relative bounded cleanup never follows links or crosses mounts. It
refuses non-empty unowned folders, changed directory identities, profile roots,
and worker output that does not match the canonical FM08 manifest.

## Development

```sh
sh BUILD
```

The package gate type-checks and tests the TypeScript product composition,
builds and smoke-tests the single-executable product worker, runs Rust native
tests, and builds an unsigned local `Forme.app` bundle. Distribution signing
and notarization are release responsibilities outside this package gate. Set
`FORME_SEA_NODE` to an injectible
Node.js 20+ executable when the active Node distribution uses a separate
`libnode` and therefore cannot receive a SEA blob.

On non-macOS builders, `sh BUILD` runs the portable TypeScript build, tests,
and coverage gate, then explicitly skips the unsupported native product gate.
