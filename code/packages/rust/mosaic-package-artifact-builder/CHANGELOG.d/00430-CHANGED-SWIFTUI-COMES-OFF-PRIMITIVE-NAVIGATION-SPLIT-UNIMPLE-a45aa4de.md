### Changed — SwiftUI comes off `primitive.navigation-split-unimplemented` (UI29-6, #15481)

The SwiftUI emitter now lowers `HostNavigationSplit` to native
`NavigationSplitView`, so SwiftUI native-complete reports no longer claim that
the primitive is missing. Flutter remains gated until its own lowering slice
lands.

