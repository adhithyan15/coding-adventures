// HelloWorldApp.swift
// The entry point of the iOS app.
//
// @main tells Swift this is where the program starts. SwiftUI apps
// don't have a traditional `main()` function — the App protocol
// handles that for us.

import SwiftUI

@main
struct HelloWorldApp: App {
    var body: some Scene {
        // WindowGroup is the standard container for an iOS app's UI.
        // On iPad it can support multiple windows; on iPhone it's
        // always a single window. We don't need to think about that
        // distinction yet.
        WindowGroup {
            ContentView()
        }
    }
}
