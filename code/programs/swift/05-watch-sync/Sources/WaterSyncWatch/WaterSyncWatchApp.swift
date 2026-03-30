// WaterSyncWatchApp.swift  (Watch)
//
// Entry point for the Watch app. Starts connectivity immediately so
// transferUserInfo deliveries queued while the app wasn't running are
// received as soon as the app launches.

import SwiftUI
import SwiftData

@main
struct WaterSyncWatchApp: App {

    init() {
        WatchConnectivityManager.shared.start()
    }

    var body: some Scene {
        WindowGroup {
            WatchContentView()
        }
        .modelContainer(for: WaterEntry.self)
    }
}
