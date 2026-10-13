### Added — Compose pipeline project shells carry story fixtures (EM-6, #14704)

- **`compose_pipeline_project_files`** (new, public) returns the runnable
  Compose Desktop project `mosaic-compile --backend compose --emit-project`
  writes beside one component (UI32 §2.2). It reuses the permissive shell
  `pkg --emit-project` writes, so the two cannot drift. It has no layout
  variants, host effects, bundled runtime or application id, and its banners
  name the pipeline command. A component named like a shell file (`Main`,
  `MosaicPlatform`, …) is refused instead of overwriting it.
- **Story fixtures replace sample values** in `MosaicAppShell.kt`:
  - text, image, colour and one-of slots become escaped strings;
  - numbers become `Double` literals and bools become `Boolean` literals;
  - `list<text>` and `list<list<text>>` become `listOf<…>(…)`.

  A number or bool that does not parse, or a list of the wrong shape, keeps
  the sample, so the project still compiles. A runtime that sends props
  still wins. The package shell passes no fixtures, so its call sites are
  unchanged.
- **`mosaicStringList` / `mosaicStringListList`** take an optional
  `fallback` (default `emptyList()`).
- **`escape_kotlin_string` escapes `$`.** Kotlin reads `$name` and `${…}`
  in a string literal as a template, so a value such as `"${exit()}"` in a
  fixture or slot default would have run as code in the generated shell,
  and `"$5"` would not compile.
