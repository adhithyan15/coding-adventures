// ContentView.swift
// The one and only screen of this app.
//
// In SwiftUI, every piece of UI is a View. A View is a struct that
// has a single requirement: a `body` property that returns some View.
// SwiftUI reads `body`, builds a description of the screen, and
// renders it. When state changes, it re-reads `body` and updates
// only the parts that changed.

import SwiftUI

struct ContentView: View {

    // `body` describes what this view looks like.
    // `some View` means "some concrete type that conforms to View"
    // — we don't have to name the exact type, Swift infers it.
    var body: some View {
        // VStack arranges its children in a vertical column.
        VStack(spacing: 16) {

            // SF Symbols is Apple's built-in icon library.
            // "drop.fill" is the filled water drop symbol.
            // systemName: refers to an SF Symbol by name.
            Image(systemName: "drop.fill")
                .font(.system(size: 64))
                .foregroundStyle(.blue)

            Text("Hello, World!")
                .font(.largeTitle)
                .bold()

            Text("Welcome to your first iOS app.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        // padding() adds breathing room between the content and the
        // edges of the screen.
        .padding()
    }
}

// PreviewProvider lets Xcode show a live preview of this view
// in the canvas without running the full simulator.
#Preview {
    ContentView()
}
