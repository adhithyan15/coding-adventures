# DPI-aware TaskApp native smoke geometry

The smoke test's 1280/520 widths express logical layout units. MoveWindow takes
physical pixels. At 144 DPI the nominal wide window remained below the logical
NavigationView expanded breakpoint, producing false failures in eager and
deferred builds alike (#18865, related investigation #18515).

Read the target window DPI and scale requested width and height by DPI / 96,
rounding to integral pixels before MoveWindow. Fail explicitly if DPI lookup
returns zero. Preserve the existing narrow/wide assertions and lifecycle tests.
At 96 DPI the dimensions are unchanged. At 144 DPI the wide width is 1920 and
the narrow width is 780. Validate using actual native smoke on the 144-DPI
desktop; this repairs test geometry, not the unresolved Board crash.
