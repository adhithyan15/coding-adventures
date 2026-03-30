// WaterSyncApp.swift  (iPhone)
//
// Entry point. Sets up SwiftData and starts the WatchConnectivity session.
// Connectivity must be started before any UI appears so the session
// delegate is set before any messages could arrive.

import SwiftUI
import SwiftData

@main
struct WaterSyncApp: App {

    init() {
        // Start connectivity as early as possible so we don't miss
        // any transferUserInfo deliveries that were queued while the
        // app was not running.
        ConnectivityManager.shared.start()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .modelContainer(for: WaterEntry.self)
    }
}
