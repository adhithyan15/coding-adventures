// HelloWorldWatchApp.swift
// Entry point for the Apple Watch companion app.
//
// watchOS apps are structured identically to iOS apps from a
// SwiftUI perspective — same @main, same App protocol, same
// WindowGroup. The difference is the hardware: a 49mm display
// on the Ultra with a Digital Crown and Action Button.

import SwiftUI

@main
struct HelloWorldWatchApp: App {
    var body: some Scene {
        WindowGroup {
            WatchContentView()
        }
    }
}
