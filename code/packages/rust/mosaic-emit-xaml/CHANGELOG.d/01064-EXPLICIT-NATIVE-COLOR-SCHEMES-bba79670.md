### Fixed — Explicit native color schemes

- Lower single `color-scheme: light` / `dark` values to subtree `RequestedTheme`, so native template resources follow authored surfaces. Reject unsupported preference lists and resource expressions rather than emitting invalid enum values (#18262).
