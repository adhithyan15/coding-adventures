// WaterEntry.swift
//
// The single data model for this app — identical across Stages 04, 05, 06.
// Stored in SwiftData locally on the device. No iCloud, no server.
//
// WHY ml, NOT GLASSES:
//   Storing amountMl (not "glasses") future-proofs Apple Health integration.
//   HKQuantitySample for dietary water uses millilitres (or fluid ounces).
//   Converting stored ml to displayed glasses is a trivial UI computation:
//
//       filledGlasses = totalMl / 250
//
//   The inverse is equally simple. Storing an abstract "glasses count" would
//   lose precision and require a migration when we add variable serving sizes.

import SwiftData
import Foundation

@Model final class WaterEntry {
    var id:        UUID
    var timestamp: Date
    var amountMl:  Int   // always 250 in this stage; variable in future stages

    init(amountMl: Int = 250) {
        self.id        = UUID()
        self.timestamp = Date()
        self.amountMl  = amountMl
    }
}
