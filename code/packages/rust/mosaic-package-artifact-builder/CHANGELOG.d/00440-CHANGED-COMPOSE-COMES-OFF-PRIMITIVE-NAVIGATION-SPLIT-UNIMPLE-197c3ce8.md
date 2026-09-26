### Changed — Compose comes off `primitive.navigation-split-unimplemented` (UI29-6, #15481)

The Compose emitter now lowers `HostNavigationSplit` to Material 3's adaptive
`NavigationSuiteScaffoldLayout`, so Compose native-complete reports no longer
claim that the primitive is missing. Flutter remains gated until its own
lowering slice lands.

