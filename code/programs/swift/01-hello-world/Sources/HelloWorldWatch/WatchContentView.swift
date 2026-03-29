// WatchContentView.swift
// The one screen shown on Apple Watch.
//
// Watch screens need to be glanceable — the user raises their wrist
// for a few seconds at most. Everything important should be visible
// immediately without scrolling.

import SwiftUI

struct WatchContentView: View {
    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "drop.fill")
                .font(.system(size: 32))
                .foregroundStyle(.blue)

            Text("Hello from\nyour Watch!")
                .font(.headline)
                .multilineTextAlignment(.center)
        }
    }
}

#Preview {
    WatchContentView()
}
