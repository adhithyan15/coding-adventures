// WatchConnectivityManager.swift  (Watch)
//
// Mirrors ConnectivityManager on the iPhone side.
// On watchOS, WCSession.isSupported() always returns true so there's
// no guard — but we must still activate before sending anything.
//
// The Watch uses the same transferUserInfo primitive as the iPhone:
// guaranteed delivery, OS-queued, survives Watch restarts and long
// gaps out of Bluetooth range.

import WatchConnectivity
import Foundation

@Observable
final class WatchConnectivityManager: NSObject, WCSessionDelegate {

    static let shared = WatchConnectivityManager()

    var isPhoneReachable = false

    /// Called when an entry arrives from the iPhone.
    var onReceiveEntry: ((SyncPayload) -> Void)?

    func start() {
        WCSession.default.delegate = self
        WCSession.default.activate()
    }

    /// Sends a drink entry to the iPhone via guaranteed-delivery queue.
    func send(_ entry: WaterEntry) {
        guard WCSession.default.activationState == .activated else { return }
        let payload = SyncPayload(from: entry)
        WCSession.default.transferUserInfo(payload.toDictionary())
    }

    // ── WCSessionDelegate ─────────────────────────────────────────────

    func session(_ session: WCSession,
                 didReceiveUserInfo userInfo: [String: Any]) {
        guard let payload = SyncPayload(from: userInfo) else { return }
        DispatchQueue.main.async { [weak self] in
            self?.onReceiveEntry?(payload)
        }
    }

    func sessionReachabilityDidChange(_ session: WCSession) {
        DispatchQueue.main.async { [weak self] in
            self?.isPhoneReachable = session.isReachable
        }
    }

    func session(_ session: WCSession,
                 activationDidCompleteWith state: WCSessionActivationState,
                 error: Error?) {
        DispatchQueue.main.async { [weak self] in
            self?.isPhoneReachable = session.isReachable
        }
    }
}
