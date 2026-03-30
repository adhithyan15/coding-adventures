// WaterWatchApp.swift
//
// Entry point for the WaterWatch standalone watchOS app.
//
// "Standalone" means this app does not require a paired iPhone app to
// function. The Watch stores all data in its own local SwiftData database.
// The iPhone is irrelevant until Stage 05, when WatchConnectivity sync
// is added.
//
// The ModelContainer is created once here and injected into the SwiftUI
// environment. Every view in the app receives it automatically via
// @Environment(\.modelContext).

import SwiftUI
import SwiftData

@main
struct WaterWatchApp: App {

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        // .modelContainer(for:) creates the SQLite store on the Watch's
        // local storage the first time the app runs, and opens it on
        // subsequent launches. No configuration beyond the model type
        // is needed for this stage.
        .modelContainer(for: WaterEntry.self)
    }
}
