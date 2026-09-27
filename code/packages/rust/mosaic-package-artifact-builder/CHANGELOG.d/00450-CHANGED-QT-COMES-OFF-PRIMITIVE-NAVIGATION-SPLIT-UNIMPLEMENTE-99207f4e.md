### Changed — Qt comes off `primitive.navigation-split-unimplemented` (UI29-6, #15481)

The Qt emitter now lowers `HostNavigationSplit` to Qt Quick Controls'
`SplitView`, so Qt native-complete reports no longer claim that the primitive
is missing. The permanent lack of adaptive collapse remains explicit as the
non-gating `interaction.navigation-split-collapse-static` behaviour entry.
Flutter remains gated until its lowering slice lands.

