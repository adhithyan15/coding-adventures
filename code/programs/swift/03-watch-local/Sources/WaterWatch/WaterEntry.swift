// WaterEntry.swift
//
// The data model for a single logged drink.
//
// This file is intentionally identical to the WaterEntry model in
// Stage 04 (iOS persistence). Using the same schema on both platforms
// means Stage 05 (Watch Sync) can merge the two stores without any
// data transformation — a UUID from the Watch matches a UUID on the
// iPhone, so deduplication is a simple equality check.
//
// SwiftData requires watchOS 10+. The Apple Watch Ultra (original)
// shipped with watchOS 9 but supports watchOS 11, so this works on
// the target hardware.

import SwiftData
import Foundation

/// A single logged drink, stored in the Watch's local SwiftData database.
///
/// # What SwiftData does automatically
/// The `@Model` macro generates:
/// - A SQLite schema with columns for each stored property
/// - Change tracking so `@Query` views re-render when data changes
/// - Automatic saves on the next run loop tick after `context.insert(_:)`
///
/// No manual `save()` call is needed. No migrations needed yet (version 1).
@Model
final class WaterEntry {

    /// Unique identifier — also used as the sync key in Stage 05.
    /// UUIDs are globally unique, so a UUID generated on the Watch
    /// will never collide with one generated on the iPhone.
    var id: UUID

    /// When the drink was logged.
    /// Used to filter "today's" entries: timestamp >= midnight today.
    var timestamp: Date

    /// Volume of the drink in millilitres.
    /// Hardcoded to 250ml for this stage. Configurable amounts are
    /// introduced when the settings screen is added in Foveo.
    var amountMl: Int

    /// Creates a new drink entry with the current time and default volume.
    init(amountMl: Int = 250) {
        self.id        = UUID()
        self.timestamp = Date()
        self.amountMl  = amountMl
    }
}
