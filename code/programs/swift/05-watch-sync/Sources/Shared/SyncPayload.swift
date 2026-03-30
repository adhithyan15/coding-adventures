// SyncPayload.swift  (Shared)
//
// The data structure transferred between iPhone and Watch via WatchConnectivity.
//
// We use a plain struct — not WaterEntry — because WatchConnectivity's
// transferUserInfo takes [String: Any], not SwiftData model objects.
// The payload is just three fields: id, timestamp, amountMl.
// On the receiving side we reconstruct a WaterEntry from these fields.

import Foundation

struct SyncPayload {
    let id: UUID
    let timestamp: Date
    let amountMl: Int

    /// Create a payload from a logged WaterEntry.
    init(from entry: WaterEntry) {
        self.id        = entry.id
        self.timestamp = entry.timestamp
        self.amountMl  = entry.amountMl
    }

    /// Deserialise from the [String: Any] received over WatchConnectivity.
    /// Returns nil if any required field is missing or malformed.
    init?(from dict: [String: Any]) {
        guard
            let idString = dict["id"]        as? String,
            let id       = UUID(uuidString: idString),
            let ts       = dict["timestamp"] as? TimeInterval,
            let ml       = dict["amountMl"]  as? Int
        else { return nil }

        self.id        = id
        self.timestamp = Date(timeIntervalSince1970: ts)
        self.amountMl  = ml
    }

    /// Serialise to [String: Any] for transferUserInfo.
    func toDictionary() -> [String: Any] {
        [
            "id":        id.uuidString,
            "timestamp": timestamp.timeIntervalSince1970,
            "amountMl":  amountMl
        ]
    }
}
