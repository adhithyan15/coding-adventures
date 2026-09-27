### Added — non-gating platform behaviour degradations (UI29-6, #15491)

`mosaic-degradations.json` now separates permanent platform limitations into
`behaviorDegradations`. Qt and Flutter record
`interaction.navigation-split-collapse-static` for
`HostNavigationSplit(collapse: auto)`. Flutter's still-missing primitive
lowering continues to fail the ordinary capability gate; Qt's capability gap
is now closed without hiding the collapse limitation in an allowlist.
`collapse: never` records no behaviour degradation.


