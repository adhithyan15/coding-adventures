### Fixed — a layout variant may not take a name another export or variant claims, on every backend (UI48 ENV2, §7.2/§7.5/§7.9)

- One check, `check_layout_namespace`, now refuses a layout variant -- of ANY
  export, not only the root -- whose root would take a name already claimed in
  the package's namespace, on SwiftUI, Compose and Flutter as on Qt and XAML.
  It closes the gap UI48 §7.2/§7.5/§7.9 recorded:
  - `Card.touch.mll` beside an exported `CardTouch` (`CardTouch`, or
    `CardTouchView` on SwiftUI) -- refused, naming the variant's `.mll` and
    the export;
  - `Card.touch-bar.mll` and `CardTouch.bar.mll`, which spell one root
    (`CardTouchBar`) -- refused, naming both files;
  - a name the project shell declares (`Mosaic.host.mll` → `MosaicHost`).
- Each backend describes its namespace as data (`LayoutNamespace`): the
  variant-root naming function, what an export's default file declares
  (Compose `<X>`, `<X>Event`, `<X>Props`/`<X>Props<n>`; SwiftUI `<X>View`,
  `<X>Event`; Flutter `<X>`, `<X>Event` and the whole `<X>Event<Case>` prefix;
  Qt `<X>`; XAML `<X>`, `<X>Event`, `<X>Mosaic...`), what a variant's own file
  claims (XAML's support namespace), and the shell's reserved names.
- New `COMPOSE_SHELL_RESERVED_NAMES` (`MosaicApp`, `MosaicStartup`,
  `MosaicRuntimeHost`, `MosaicPlatformRouter`, ...), pinned by
  `every_public_name_the_compose_shell_declares_is_reserved` against every
  Kotlin source a permissive and a native-complete Compose project generate,
  in both directions. SwiftUI needs no list: a variant root ends in `View` and
  no SwiftUI shell type does, pinned by
  `no_swiftui_shell_type_can_be_a_variant_root`.
- SwiftUI, Compose and Flutter check before anything is written, flat builds
  included (the flat artifacts are what a consumer compiles together). XAML's
  `xaml_check_variant_types` keeps its "variants need a default `<C>.mll`"
  check and now delegates the names, so it also refuses a variant against
  another export's names before the emitter does (messages now say "`X`
  twice ... the export Y (Y.mll)" and "... the WinUI shell"). Qt's
  `qt_cmake_with_layout_variants` delegates too and still checks only in its
  project shell, where the QML module is.
- Every package in `code/programs/mosaic`, `code/packages/mosaic` and the
  emitter fixtures was emitted on all five backends, flat and as a project,
  before and after: the same 440 builds succeed and the same 20 fail, for
  their existing reasons.
- Tests: `a_variant_root_named_like_another_export_is_refused`,
  `two_exports_variants_that_spell_one_root_are_refused`,
  `a_multi_export_package_whose_variant_roots_are_distinct_builds`,
  `a_variant_root_may_not_take_a_name_the_backend_declares` (each over
  SwiftUI, Compose and Flutter, flat and project), plus the two shell pins.
