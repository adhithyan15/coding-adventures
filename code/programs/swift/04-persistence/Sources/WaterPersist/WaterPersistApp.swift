// WaterPersistApp.swift
//
// App entry point. Sets up the SwiftData ModelContainer which creates
// (or opens) the SQLite database on the device's local storage.
//
// The container is injected into the SwiftUI environment so every view
// in the app can access it via @Environment(\.modelContext).

import SwiftUI
import SwiftData

@main
struct WaterPersistApp: App {

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        // SwiftData creates the SQLite store the first time the app runs.
        // On subsequent launches it opens the existing file — this is
        // exactly what makes persistence work across app restarts.
        .modelContainer(for: WaterEntry.self)
    }
}
