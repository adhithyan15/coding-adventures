### Added — a package's own Android handler, `compose-android` `[host_effects]` (UI89 §3.12)

- A Compose build that writes its Android project now copies the package's
  `compose-android` `[host_effects]` files into `android/<target>`. They are
  read under the rules the desktop copy keeps: a path relative to the
  package, a source that resolves inside it, and a regular file. The
  target must be a Kotlin source under `src/main/kotlin/`.
- A target that would replace a file the Android project already holds (the
  activity, a shared source, a component, the Gradle files) is refused,
  compared without case.
- `MosaicActivity` installs the `compose-android` handler as the host loads,
  ahead of the platform library, and passes the handler's `kinds` to
  `installMosaicPlatformEffects(it, documentPicker, kinds)`. This works for
  both the strict and the sample activity.
  - A plain install name is imported into `mosaic.android`.
  - A dotted one is called by its full name.
  - `include` and a `:` qualifier are refused, because Kotlin has neither.
- The desktop `compose` handler and every other backend ignore
  `compose-android` entries. With no such handler the activity is unchanged.
- The generated Android README says where a package's Android handler comes
  from.
