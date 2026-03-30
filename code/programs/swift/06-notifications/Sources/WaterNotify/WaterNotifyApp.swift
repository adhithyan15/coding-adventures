// WaterNotifyApp.swift
//
// Entry point. Sets up SwiftData and initialises the notification manager
// before the first scene renders.
//
// ORDER MATTERS:
//   NotificationManager.shared.setup() must be called in init(), not in
//   onAppear of ContentView. By the time onAppear fires, iOS may have already
//   tried to deliver a notification that was queued from a prior session. If
//   the delegate is not set yet, that delivery would be swallowed silently.
//
//   This mirrors the pattern from Stage 05: ConnectivityManager.shared.start()
//   was called in WaterSyncApp.init() for the same reason.

import SwiftUI
import SwiftData

@main
struct WaterNotifyApp: App {

    init() {
        // Start as early as possible:
        //   1. Sets self as UNUserNotificationCenterDelegate
        //   2. Checks/requests authorisation
        //   3. Schedules the static daily reminder set if authorised
        NotificationManager.shared.setup()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .modelContainer(for: WaterEntry.self)
    }
}
