### Fixed — the XAML install line checks each kind it writes (UI87 §7.6)

- `xaml_platform_install_line` splices each `[host_effects]` kind into a C#
  string literal; it now checks the manifest's dotted-name shape itself and
  fails the build for anything else, instead of relying only on the
  manifest's validation. A test pins the check to the manifest's (same
  accept/refuse on quotes, backslashes, spaces, empty segments, leading
  digits, `-`).
- An empty kind list is written `System.Array.Empty<string>()` rather than
  `new[] {  }`.

