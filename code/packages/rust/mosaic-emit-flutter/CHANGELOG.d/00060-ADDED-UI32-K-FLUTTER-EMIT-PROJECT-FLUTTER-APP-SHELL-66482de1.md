### Added — UI32-K-flutter — `--emit-project` Flutter app shell

L5 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2 React #4297, L3 HTML #4309, L4 WebComponent #4315). `mosaic-compile --backend flutter --emit-project` now produces a flutter-create-shaped scaffold alongside the component `.dart`:

- `pubspec.yaml` — pinned Flutter SDK `>=3.32.0 <4.0.0` + Dart `>=3.5.0 <4.0.0` per UI32 §3.6.3. Dart pub package name follows snake_case rules (§3.6.2 Flutter row) — auto-derived as `mosaic_{snake(name)}` (e.g., `ProfileCard` → `mosaic_profile_card`).
- `lib/main.dart` — `MaterialApp` shell that mounts the component as `Scaffold.body`'s `Center(child: <Component>())`. Imports the component package-locally from `lib/{Component}.dart`, which keeps the generated shell valid under Dart's package-boundary rules.
- `README.md` — `flutter pub get && flutter run` recipe + file map.

New public API (matches L2/L3/L4 pattern):

- `pub struct EmitOptions` — `emit_project`, `pinned_flutter_sdk`, `pinned_dart_sdk`, `package_name` override.
- `pub struct ProjectFiles` — `pubspec_yaml`, `main_dart`, `readme`.
- `pub enum ProjectShellError` — `InvalidDartPubName(String)` surfaced through `PipelineEmitError::UnsafeSlotName`.
- `pub struct PipelineEmitResultWithProject` — `output`, `component_name`, `project: Option<ProjectFiles>`.
- `pub fn from_pipeline_with_options(...)` — new entry. Existing `from_pipeline(...)` unchanged.

UI32 §3.6.2 Flutter row: Dart pub names MUST match `[a-z][a-z0-9_]*` (lowercase, digits, underscores; must start with letter; no leading underscore; no hyphens; no uppercase). `is_valid_dart_pub_name` enforces this; an explicit invalid `package_name` fails-loud via `ProjectShellError::InvalidDartPubName`.

11 new tests cover the spec §3 gates plus a Dart-pub-name truth table (8 accept/reject vectors) and a main.dart structural test (MaterialApp + Scaffold + Component() mount). Total tests: 48 (was 37, +11).

