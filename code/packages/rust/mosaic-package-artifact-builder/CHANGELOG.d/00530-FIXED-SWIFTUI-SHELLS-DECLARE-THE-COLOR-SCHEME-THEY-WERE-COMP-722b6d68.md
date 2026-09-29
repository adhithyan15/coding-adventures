### Fixed — SwiftUI shells declare the color scheme they were compiled with (UI32)

A dark-themed app on a light-mode device drew dark components on a white
window, with a dark status bar over white and unreadable titles, because the
platform draws its chrome in the system scheme. When the root component's
stylesheet is `.dark.msl` or `.light.msl`, the generated `App.swift` now
applies `.preferredColorScheme(.dark)` / `(.light)` to the root view. A
theme-neutral stylesheet declares nothing. Verified on the iPhone simulator
with Trestle.

