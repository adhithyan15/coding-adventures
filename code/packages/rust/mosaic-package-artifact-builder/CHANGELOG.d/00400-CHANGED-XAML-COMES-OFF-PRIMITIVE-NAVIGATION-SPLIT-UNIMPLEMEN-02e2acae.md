### Changed — XAML comes off `primitive.navigation-split-unimplemented` (UI29-6, #15481)

The XAML emitter lowers `HostNavigationSplit` to `NavigationView` as of slice
`K-xaml`, so XAML is no longer reported as missing it. UI84 §3: the lowering
records the drop, so closing the gap closes the report, in the same change.

Flutter still reports it, and its slice removes the backend the same way.

## 2026-09-13

