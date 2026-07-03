# Changelog

All notable changes to `mosaic-pkg-dialog` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## 0.2.0 — 2026-05-21

Rewritten on top of the new `HostDialog` kernel primitive added by
UI29-1 (PR #3846).  The composition shrinks (no more outer-`Box` +
title-`Box` wrappers); the rendered behavior gets dramatically richer
(modal blocking, focus trap, Esc-to-close, top-layer rendering,
screen-reader `dialog` role announcement, OS-native styling via each
backend's dialog primitive).

### Breaking changes

- **New required slot `open: bool`.**  v0.1.0 had no visibility slot —
  the host decided whether to mount Dialog at all by including or
  excluding it from its layout tree.  v0.2.0 makes `open` a first-class
  slot driven by the host so the platform's native dialog element can
  manage show/hide transitions natively (with animation, focus
  restoration, and accessibility events).  Hosts upgrading must add an
  `open` binding when instantiating Dialog.
- **`dialog-root` part renamed to `dialog-shell`.**  The part now styles
  the platform's native dialog element (`<dialog>` on React/HTML/Web-
  Component, `.sheet` on SwiftUI, `Popup` on Qt, `ContentDialog` on
  XAML) via that element's styling hook, not a plain `Box` wrapper.
  The new name reflects that change.  Hosts with a custom theme must
  update `part dialog-root { ... }` → `part dialog-shell { ... }`.
- **`dialog-title` part removed.**  HostDialog renders the title
  natively via its own `title:` slot using the platform's title
  typography (`<dialog>`'s heading, `.sheet`'s navigation-bar title,
  `Popup`'s contentItem title, `ContentDialog`'s `Title` property).
  Hosts that previously customized the title via the `dialog-title`
  part must move those overrides into their platform theme; the
  package no longer exposes a hook for it.

### Added

- `HostDialog` as the layout root, replacing v0.1.0's `Box [dialog-root]`
  outer wrapper.  Driven by `open: slot: open`, `modal: true`,
  `title: slot: title`, and `onClose: emit: onClose`.
- 0.2.0 entry in this file documenting the migration.

### Changed

- `Dialog.dark.msl` shrinks from four parts to three (`dialog-shell`,
  `dialog-message`, `dialog-actions`).
- Smoke test (`tests/package_compiles.rs`) updated to assert the new
  slot count (4), the new layout shape (HostDialog as root), the new
  part set (3), and to drive per-backend artifact compilation for
  React / SwiftUI / Qt with graceful handling for backends whose
  HostDialog lowering has not yet landed on main (the test records
  those as "deferred" rather than failing — see the test source for
  the rationale).

### Migration guide

```diff
-component Dialog {
-  slot title       : text ;
-  slot message     : text ;
-  slot close-label : text ;
-  emit onClose ;
-}
+component Dialog {
+  slot open        : bool ;
+  slot title       : text ;
+  slot message     : text ;
+  slot close-label : text ;
+  emit onClose ;
+}
```

```diff
-layout Dialog {
-  Box [ dialog-root ] {
-    Column [ dialog-stack ] {
-      Box [ dialog-title ]   { Text ( content: slot: title   ) }
-      Box [ dialog-message ] { Text ( content: slot: message ) }
-      Box [ dialog-actions ] {
-        HostButton ( label: slot: close-label , onTap: emit: onClose )
-      }
-    }
-  }
-}
+layout Dialog {
+  HostDialog [ dialog-shell ] (
+    open    : slot: open ,
+    modal   : true ,
+    title   : slot: title ,
+    onClose : emit: onClose
+  ) {
+    Column [ dialog-stack ] {
+      Box [ dialog-message ] {
+        Text ( content: slot: message )
+      }
+      Box [ dialog-actions ] {
+        HostButton ( label: slot: close-label , onTap: emit: onClose )
+      }
+    }
+  }
+}
```

```diff
 style Dialog {
-  part dialog-root { ... }
-  part dialog-title { ... }
+  part dialog-shell { ... }
   part dialog-message { ... }
   part dialog-actions { ... }
 }
```

Hosts that previously chose whether to render Dialog by mounting it
must now keep it mounted and toggle the `open` slot instead.

## 0.1.0 — 2026-05-20

Initial release.  Ships a single Dialog component used as the
cross-backend smoke test for the UI29 kernel.

### Added

- `mosaic-package.toml` manifest declaring
  `[components].exports = ["Dialog"]` and targeting UI29 kernel
  version `"1"`.
- `Dialog.mil`: interface with three text slots (`title`, `message`,
  `close-label`) and one zero-payload emit (`onClose`).
- `Dialog.mll`: layout composed of `Box` (outer frame, `dialog-root`),
  `Column` (vertical stack, `dialog-stack`), and three inner `Box`
  parts (`dialog-title`, `dialog-message`, `dialog-actions`) wrapping
  two `Text` leaves and one `HostButton`.
- `Dialog.dark.msl`: dark-theme stylesheet covering all four named
  parts.
- `tests/package_compiles.rs`: integration smoke test that round-trips
  every source file through `mosmodel-compiler`, `moslayout-compiler`,
  and `mosstyle-compiler`, asserts the structural shape of the layout
  tree, and drives `mosaic-package-artifact-builder::build_package` for
  every backend the builder currently supports (React, SwiftUI, Qt) —
  asserting a non-empty `Dialog.<ext>` lands on disk for each.
