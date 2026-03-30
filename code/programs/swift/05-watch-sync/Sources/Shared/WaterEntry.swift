// WaterEntry.swift  (Shared)
//
// The SwiftData model used by BOTH the iPhone app and the Watch app.
// Each device maintains its own independent SQLite store — this file
// is compiled into both targets.
//
// The UUID `id` is the sync key. When a drink logged on the Watch
// arrives on the iPhone (and vice versa), we check whether that UUID
// already exists before inserting, which makes sync idempotent:
// delivering the same entry twice never creates a duplicate.

import SwiftData
import Foundation

@Model
final class WaterEntry {

    /// Globally unique identifier — the deduplication key for sync.
    var id: UUID

    /// When the drink was logged. Filters "today" via >= midnight.
    var timestamp: Date

    /// Volume in ml. Default 250 = one standard glass.
    var amountMl: Int

    init(amountMl: Int = 250) {
        self.id        = UUID()
        self.timestamp = Date()
        self.amountMl  = amountMl
    }
}
