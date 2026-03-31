// WaterEntry.swift
//
// The data model for a single logged drink.
//
// SwiftData persists this to a SQLite file in the app's container on
// the device. The file survives app restarts, OS updates, and even
// device reboots — it only disappears if the user deletes the app.
//
// This schema is intentionally identical to Stage 03 (Watch) so that
// Stage 05 (Watch Sync) can merge both stores without translation.

import SwiftData
import Foundation

/// A single logged drink, stored in the device's local SwiftData database.
///
/// # How SwiftData works
/// The `@Model` macro inspects this class at compile time and generates:
/// - A SQLite table with one column per stored property
/// - Automatic change tracking so @Query views re-render on insert/delete
/// - Codable support for CloudKit sync (used in Foveo, not this stage)
///
/// SwiftData saves automatically — no `context.save()` call is needed.
/// Saves happen on the next run loop tick after `context.insert(_:)`.
@Model
final class WaterEntry {

    /// Globally unique identifier.
    /// Used as the deduplication key when Watch and iPhone sync in Stage 05.
    var id: UUID

    /// When the drink was logged, in the user's local timezone.
    /// Filtered by `>= midnight today` to compute the daily total.
    var timestamp: Date

    /// Volume in millilitres. Defaults to 250ml (one standard glass).
    var amountMl: Int

    init(amountMl: Int = 250) {
        self.id        = UUID()
        self.timestamp = Date()
        self.amountMl  = amountMl
    }
}
