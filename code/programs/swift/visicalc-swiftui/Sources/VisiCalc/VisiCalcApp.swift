// VisiCalcApp.swift — top-level SwiftUI app entry (VC2-swiftui).
//
// Cross-platform: macOS + iOS.  SwiftUI's `App` protocol works
// identically on both, so the `body: some Scene` block is shared.
// Only the platform-specific lifecycle wiring differs:
//
//   - macOS: an `NSApplicationDelegate` that flips the activation
//     policy to `.regular` so `swift run` puts the demo in the
//     Dock and brings the window forward (no .app bundle, no
//     Info.plist, so the default is BackgroundOnly).
//   - iOS:  no delegate needed — the system always treats apps as
//     foreground.
//
// The `ContentView` next door does the real visual work and is
// pure SwiftUI (no AppKit / UIKit imports), so it compiles
// unchanged on both platforms.

import SwiftUI

#if os(macOS)
import AppKit

/// macOS-only lifecycle delegate.  See file-header comment for why
/// it's needed under `swift run`.
final class VisiCalcAppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }
}
#endif

@main
struct VisiCalcApp: App {
    #if os(macOS)
    @NSApplicationDelegateAdaptor(VisiCalcAppDelegate.self) var appDelegate
    #endif

    var body: some Scene {
        WindowGroup("VisiCalc") {
            ContentView()
        }
    }
}
