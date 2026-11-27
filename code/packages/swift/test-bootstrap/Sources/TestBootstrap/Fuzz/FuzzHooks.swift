// ============================================================================
// FuzzHooks.swift -- sabotage seams for the negative controls (VER06l section 10)
// ============================================================================
//
// A conformance run that cannot fail proves nothing. These `internal` seams are
// the engine entry points the corpus runner calls, as swappable closures: a test
// replaces one with a broken version (`@testable import TestBootstrap`), runs the
// corpus, and the runner must report problems. Production code never writes
// them; the table lives behind a lock because tests may run on any thread.

import Foundation

enum FuzzHooks {
    struct Table {
        var bitFlip: ([UInt8], BigInt, BigInt) -> [UInt8] = { Mutate.bitFlipBig($0, bitOffset: $1, count: $2) }
        var byteFlip: ([UInt8], BigInt, BigInt) -> [UInt8] = { Mutate.byteFlipBig($0, byteOffset: $1, count: $2) }
        var arithmetic: ([UInt8], BigInt, BigInt, Int, Endian) -> [UInt8] = {
            Mutate.byteArithmeticBig($0, byteOffset: $1, delta: $2, width: $3, endian: $4)
        }
        var boundary: ([UInt8], BigInt, Int, BigInt, Endian) -> [UInt8] = {
            Mutate.boundaryInsertBig($0, byteOffset: $1, width: $2, val: $3, endian: $4)
        }
        var blockDelete: ([UInt8], BigInt, BigInt) -> [UInt8] = { Mutate.blockDeleteBig($0, offset: $1, length: $2) }
        var blockInsert: ([UInt8], BigInt, [UInt8]) -> [UInt8] = { Mutate.blockInsertBig($0, offset: $1, insert: $2) }
        var blockDuplicate: ([UInt8], BigInt, BigInt, BigInt) -> [UInt8] = {
            Mutate.blockDuplicateBig($0, srcOffset: $1, length: $2, dstOffset: $3)
        }
        var splice: ([UInt8], [UInt8], BigInt, BigInt) -> [UInt8] = {
            Mutate.spliceBig($0, $1, targetSplit: $2, donorSplit: $3)
        }
        /// (data, seed, steps, maxLen, workCap) -> counted result.
        var sequence: ([UInt8], UInt64, Int, Int, Int) throws -> Mutate.Counted = {
            try Mutate.counted($0, seed: $1, steps: $2, maxLen: $3, workCap: $4)
        }
        var corpusAdmit: ([UInt64], [UInt64]) -> (admitted: Bool, newEdges: Int) = {
            Bitmap.corpusAdmit(existing: $0, trace: $1)
        }
        var isDuplicate: (String, [String], String, [String]) -> Bool = {
            FuzzSignature.isDuplicateCrash(kindA: $0, framesA: $1, kindB: $2, framesB: $3)
        }
        var isFailureId: (String?) -> Bool = { FuzzFailureIds.isFuzzFailureId($0) }
        var formatTap: (Int, String, String, String) throws -> String = {
            try FuzzSignature.formatTapFailure($0, target: $1, failureId: $2, sig: $3)
        }
        var tfvValid: (String) -> Bool = { Tfv.isValid($0) }
    }

    private static let lock = NSLock()
    private static var storage = Table()

    /// A snapshot of the table (the runner reads it once per evaluation).
    static var current: Table {
        lock.lock(); defer { lock.unlock() }
        return storage
    }

    /// Replaces one or more hooks.
    static func modify(_ change: (inout Table) -> Void) {
        lock.lock(); defer { lock.unlock() }
        change(&storage)
    }

    /// Restores every hook to its production value.
    static func reset() {
        lock.lock(); defer { lock.unlock() }
        storage = Table()
    }
}
