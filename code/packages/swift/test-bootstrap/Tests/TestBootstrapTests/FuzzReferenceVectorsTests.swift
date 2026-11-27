import XCTest
@testable import TestBootstrap

/// The 820 oracle vectors of Go's `fuzz/reference_vectors_test.go`, generated from
/// the independent Python reference (`code/scripts/fuzz_conformance.py`). The Swift
/// test parses that Go source directly, so the table is never copied. Each vector
/// runs through the `BigInt` entry point (the corpus runner's) and, where the
/// arguments fit, the `UInt64`/`Int64` one. The corpus alone cannot pin these: its
/// splice has equal splits and its mutate_sequence case does not pin every
/// constant (VER06 section 9 item 13).
final class FuzzReferenceVectorsTests: XCTestCase {
    struct Vector {
        let op: String
        let args: [String]
        let out: String
    }

    static func loadVectors() throws -> [Vector] {
        let url = FuzzSupport.repoFile("code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go")
        let text = try String(contentsOf: url, encoding: .utf8)
        var vectors: [Vector] = []
        for line in text.split(separator: "\n") where line.hasPrefix("\t{\"") {
            // `{"op", []string{"a", "b"}, "out"},`: every quoted string, in order.
            var strings: [String] = []
            var current: String?
            for ch in line {
                if ch == "\"" {
                    if let c = current { strings.append(c); current = nil } else { current = "" }
                } else if current != nil {
                    current!.append(ch)
                }
            }
            XCTAssertGreaterThanOrEqual(strings.count, 2, String(line))
            vectors.append(Vector(op: strings[0], args: Array(strings[1..<(strings.count - 1)]), out: strings[strings.count - 1]))
        }
        return vectors
    }

    private func big(_ s: String) -> BigInt { try! BigInt.parse(s) }

    func testTheTableIsTheWholeOracleSet() throws {
        let vectors = try Self.loadVectors()
        XCTAssertEqual(vectors.count, 820)
        var counts: [String: Int] = [:]
        for v in vectors { counts[v.op, default: 0] += 1 }
        XCTAssertEqual(counts, ["arithmetic": 120, "bit_flip": 70, "block_delete": 60, "block_duplicate": 60, "block_insert": 60,
                                "boundary": 120, "byte_flip": 70, "sequence": 200, "splice": 60])
    }

    func testEveryVectorMatchesThroughTheBigAndTheMachineIntegerPaths() throws {
        var machinePaths = 0
        for (i, v) in try Self.loadVectors().enumerated() {
            let a = v.args
            let want = FuzzSupport.hex(v.out)
            let data = a.isEmpty ? [] : FuzzSupport.hex(a[0])
            func u(_ k: Int) -> UInt64? { big(a[k]).asUInt64 }
            func s(_ k: Int) -> Int64? { big(a[k]).asInt.map { Int64($0) } }
            let endian: (Int) -> Endian = { a[$0] == "big" ? .big : .little }
            var got: [UInt8]
            var small: [UInt8]?
            switch v.op {
            case "bit_flip":
                got = Mutate.bitFlipBig(data, bitOffset: big(a[1]), count: big(a[2]))
                if let o = u(1), let c = u(2) { small = Mutate.bitFlip(data, bitOffset: o, count: c) }
            case "byte_flip":
                got = Mutate.byteFlipBig(data, byteOffset: big(a[1]), count: big(a[2]))
                if let o = u(1), let c = u(2) { small = Mutate.byteFlip(data, byteOffset: o, count: c) }
            case "arithmetic":
                let w = big(a[3]).asInt ?? -1
                got = Mutate.byteArithmeticBig(data, byteOffset: big(a[1]), delta: big(a[2]), width: w, endian: endian(4))
                if let o = u(1), let d = s(2) { small = Mutate.byteArithmetic(data, byteOffset: o, delta: d, width: w, endian: endian(4)) }
            case "boundary":
                let w = big(a[2]).asInt ?? -1
                got = Mutate.boundaryInsertBig(data, byteOffset: big(a[1]), width: w, val: big(a[3]), endian: endian(4))
                if let o = u(1), let val = u(3) { small = Mutate.boundaryInsert(data, byteOffset: o, width: w, val: val, endian: endian(4)) }
            case "block_delete":
                got = Mutate.blockDeleteBig(data, offset: big(a[1]), length: big(a[2]))
                if let o = u(1), let l = u(2) { small = Mutate.blockDelete(data, offset: o, length: l) }
            case "block_insert":
                let ins = FuzzSupport.hex(a[2])
                got = Mutate.blockInsertBig(data, offset: big(a[1]), insert: ins)
                if let o = u(1) { small = Mutate.blockInsert(data, offset: o, insert: ins) }
            case "block_duplicate":
                got = Mutate.blockDuplicateBig(data, srcOffset: big(a[1]), length: big(a[2]), dstOffset: big(a[3]))
                if let so = u(1), let l = u(2), let d = u(3) { small = Mutate.blockDuplicate(data, srcOffset: so, length: l, dstOffset: d) }
            case "splice":
                let donor = FuzzSupport.hex(a[1])
                got = Mutate.spliceBig(data, donor, targetSplit: big(a[2]), donorSplit: big(a[3]))
                if let t = u(2), let d = u(3) { small = Mutate.splice(data, donor, targetSplit: t, donorSplit: d) }
            case "sequence":
                got = try Mutate.mutateSequence(data, seed: u(1)!, steps: Int(s(2)!), maxLen: Int(s(3)!))
            default:
                XCTFail("unknown vector op \(v.op)")
                return
            }
            XCTAssertEqual(got, want, "vector \(i) \(v.op) \(a)")
            if let small {
                machinePaths += 1
                XCTAssertEqual(small, want, "vector \(i) \(v.op) \(a) (machine-integer path)")
            }
        }
        XCTAssertGreaterThan(machinePaths, 250, "the machine-integer paths must really be exercised")
    }
}
