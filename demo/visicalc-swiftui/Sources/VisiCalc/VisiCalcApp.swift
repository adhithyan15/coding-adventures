// VisiCalcApp.swift — top-level SwiftUI app entry (VC2-swiftui).
//
// @main App scaffold. The ContentView next door does the real work;
// this file wires the WindowGroup AND attaches an
// NSApplicationDelegate that elevates the running process to a
// regular foreground app.
//
// Why the AppDelegate? `swift run` launches the binary without an
// .app bundle / Info.plist, so macOS treats the process as
// BackgroundOnly by default. The SwiftUI WindowGroup still creates
// a window but it never appears in the Dock and never activates,
// which makes the demo hard to find when launched from a terminal.
// The delegate's `applicationDidFinishLaunching` flips the
// activation policy to `.regular` (Dock icon + menu bar + window
// focus) and forces activation. Production .app bundles get the
// same behaviour via their Info.plist's `LSUIElement = false` so
// this is a no-op there.

import SwiftUI
import AppKit

final class VisiCalcAppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }
}

@main
struct VisiCalcApp: App {
    @NSApplicationDelegateAdaptor(VisiCalcAppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup("VisiCalc") {
            ContentView()
        }
    }
}
