// ConnectivityManager.swift  (iPhone)
//
// Manages the WatchConnectivity session on the iPhone side.
//
// KEY DESIGN DECISION — transferUserInfo vs sendMessage:
//
//   sendMessage    requires both apps running AND in Bluetooth range.
//                  Fails silently if Watch app isn't in foreground.
//                  Wrong for our use case.
//
//   transferUserInfo  queued by the OS. Delivered even if neither app
//                     is running. Survives phone reboots and Watch
//                     restarts. Correct for reliable water logging.
//
// Flow when Watch logs a drink:
//   Watch → transferUserInfo → OS queue → iPhone receives here
//   → deduplicate by UUID → insert into SwiftData if new

import WatchConnectivity
import SwiftData
import Foundation

@Observable
final class ConnectivityManager: NSObject, WCSessionDelegate {

    static let shared = ConnectivityManager()

    /// True when the Watch app is reachable in real time.
    /// Used to show a sync status indicator in the UI.
    var isWatchReachable = false

    /// Called when an entry arrives from the Watch.
    /// Set by ContentView to give us access to the model context.
    var onReceiveEntry: ((SyncPayload) -> Void)?

    func start() {
        // isSupported() returns false on iPads without a paired Watch.
        // Always check before activating.
        guard WCSession.isSupported() else { return }
        WCSession.default.delegate = self
        WCSession.default.activate()
    }

    /// Sends a drink entry to the Watch via guaranteed-delivery queue.
    func send(_ entry: WaterEntry) {
        guard WCSession.default.activationState == .activated else { return }
        let payload = SyncPayload(from: entry)
        WCSession.default.transferUserInfo(payload.toDictionary())
    }

    // ── WCSessionDelegate ─────────────────────────────────────────────

    /// Called when the Watch sends us a drink entry (offline-safe delivery).
    func session(_ session: WCSession,
                 didReceiveUserInfo userInfo: [String: Any]) {
        guard let payload = SyncPayload(from: userInfo) else { return }
        DispatchQueue.main.async { [weak self] in
            self?.onReceiveEntry?(payload)
        }
    }

    func sessionReachabilityDidChange(_ session: WCSession) {
        DispatchQueue.main.async { [weak self] in
            self?.isWatchReachable = session.isReachable
        }
    }

    func session(_ session: WCSession,
                 activationDidCompleteWith state: WCSessionActivationState,
                 error: Error?) {
        DispatchQueue.main.async { [weak self] in
            self?.isWatchReachable = session.isReachable
        }
    }

    // Required on iOS (not needed on watchOS)
    func sessionDidBecomeInactive(_ session: WCSession) {}
    func sessionDidDeactivate(_ session: WCSession) {
        // Re-activate after Apple Watch switching
        WCSession.default.activate()
    }
}

// SyncPayload is defined in Sources/Shared/SyncPayload.swift
// so both the iPhone and Watch targets can use it.
