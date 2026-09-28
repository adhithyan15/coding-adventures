### Security

Added explicit `validate_component_name` and `validate_package_name`
helpers that run at the top of `build_package`, before any I/O.
Component names from the manifest flow into:

- File paths via `out_dir.join(format!("{component}.{ext}"))` — a
  malicious manifest entry like `../../etc/passwd` would escape
  the dist directory.
- The generated `index.html`, `index.js`, and
  `MosaicPackage.props` files — a name like `Grid"; alert(1)//`
  or `Grid --><script>` would inject into the aggregated
  comment/import/XML.
- The XAML branch writes THREE files per component, tripling the
  blast radius.

The TOML parser catches some malformed names (`"` characters) at
manifest-parse time, but quote-free traversal/injection (`../foo`,
`Grid<script>`) sneaks past. The validators are the second line of
defence — they enforce strict `[A-Za-z][A-Za-z0-9_]*` for
components (matches the PascalCase convention every existing
package uses) and `[a-z][a-z0-9-]*` for packages (matches the
`mosaic-pkg-*` kebab convention). New `BuildError::UnsafeName`
variant carries `kind` (`"component"` / `"package"`), the
offending name, and a one-line reason.

The vector was caught during the U29-2 follow-up security review;
the existing React/SwiftUI/Qt paths were affected too — the
validators close the vector for all six backends at once.

