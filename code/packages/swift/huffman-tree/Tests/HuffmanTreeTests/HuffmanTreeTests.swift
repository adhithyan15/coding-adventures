// HuffmanTreeTests.swift — DT27: Huffman Tree Tests
// ============================================================================
//
// Covers: build validation, codeTable, canonicalCodeTable, decodeAll,
// weight, depth, symbolCount, leaves, isValid, edge cases (single symbol,
// two symbols, all equal weights), determinism, byte-range round-trips.

import XCTest
@testable import HuffmanTree

final class HuffmanTreeTests: XCTestCase {

    // MARK: - Build validation

    func testBuildEmptyThrows() throws {
        XCTAssertThrowsError(try HuffmanTree.build([])) { error in
            XCTAssertEqual(error as? HuffmanTree.HuffmanError,
                           HuffmanTree.HuffmanError.emptyWeights)
        }
    }

    func testBuildZeroFrequencyThrows() {
        XCTAssertThrowsError(try HuffmanTree.build([(symbol: 65, frequency: 0)]))
    }

    func testBuildNegativeFrequencyThrows() {
        XCTAssertThrowsError(try HuffmanTree.build([(symbol: 65, frequency: -1)]))
    }

    func testBuildSingleSymbolSucceeds() throws {
        let tree = try HuffmanTree.build([(symbol: 65, frequency: 5)])
        XCTAssertNotNil(tree)
    }

    func testBuildManySymbolsSucceeds() throws {
        let weights = (1...20).map { (symbol: $0, frequency: $0) }
        let tree = try HuffmanTree.build(weights)
        XCTAssertEqual(tree.symbolCount, 20)
    }

    // MARK: - codeTable

    func testCodeTableAABBC() throws {
        // Heap construction trace for A(65,3), B(66,2), C(67,1):
        //   Priority tuples: C=[1,0,67,MAX], B=[2,0,66,MAX], A=[3,0,65,MAX]
        //   Pop C(weight=1), pop B(weight=2) → Internal(w=3, left=C, right=B, order=0)
        //   Heap: A=[3,0,65,MAX], Internal=[3,1,MAX,0]
        //   Pop A (leaf wins tie), pop Internal → root(w=6, left=A, right=Internal)
        // Codes: A→"0", C→"10", B→"11"
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let tbl  = tree.codeTable()
        XCTAssertEqual(tbl[65], "0",  "A should get code '0'")
        XCTAssertEqual(tbl[67], "10", "C should get code '10'")
        XCTAssertEqual(tbl[66], "11", "B should get code '11'")
    }

    func testCodeTableSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 42, frequency: 7)])
        XCTAssertEqual(tree.codeTable()[42], "0")
    }

    func testCodeTableTwoSymbols() throws {
        let tree = try HuffmanTree.build([(65,10),(66,1)])
        let tbl  = tree.codeTable()
        XCTAssertEqual(tbl[65]?.count, 1)
        XCTAssertEqual(tbl[66]?.count, 1)
    }

    func testCodeTableCodesAreDistinct() throws {
        let tree = try HuffmanTree.build([(1,5),(2,3),(3,2),(4,1)])
        let tbl  = tree.codeTable()
        let codes = Array(tbl.values)
        let unique = Set(codes)
        XCTAssertEqual(codes.count, unique.count, "All codes should be distinct")
    }

    func testCodeTablePrefixFree() throws {
        let tree = try HuffmanTree.build([(1,5),(2,3),(3,2),(4,1),(5,1)])
        let codes = Array(tree.codeTable().values)
        for i in codes.indices {
            for j in codes.indices where i != j {
                let a = codes[i], b = codes[j]
                XCTAssertFalse(b.hasPrefix(a),
                               "'\(a)' should not be a prefix of '\(b)'")
            }
        }
    }

    func testCodeTableAllSymbolsPresent() throws {
        let inputs = [(10,5),(20,3),(30,2),(40,1)]
        let tree   = try HuffmanTree.build(inputs)
        let tbl    = tree.codeTable()
        for (sym, _) in inputs {
            XCTAssertNotNil(tbl[sym], "symbol \(sym) should be in code table")
        }
    }

    // MARK: - codeFor

    func testCodeForMatchesCodeTable() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let tbl  = tree.codeTable()
        XCTAssertEqual(tree.codeFor(65), tbl[65])
        XCTAssertEqual(tree.codeFor(66), tbl[66])
        XCTAssertEqual(tree.codeFor(67), tbl[67])
    }

    func testCodeForUnknownSymbol() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2)])
        XCTAssertNil(tree.codeFor(99))
    }

    func testCodeForSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 1, frequency: 1)])
        XCTAssertEqual(tree.codeFor(1), "0")
    }

    // MARK: - canonicalCodeTable

    func testCanonicalCodeTableAABBC() throws {
        let tree  = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let canon = tree.canonicalCodeTable()
        // Lengths: A=1, B=2, C=2. Sorted: A(1), B(2), C(2).
        // Canonical: A→"0", B→"10", C→"11"
        XCTAssertEqual(canon[65], "0",  "canonical A → 0")
        XCTAssertEqual(canon[66], "10", "canonical B → 10")
        XCTAssertEqual(canon[67], "11", "canonical C → 11")
    }

    func testCanonicalCodeTableSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 5, frequency: 10)])
        XCTAssertEqual(tree.canonicalCodeTable()[5], "0")
    }

    func testCanonicalPreservesLengths() throws {
        let weights = [(1,5),(2,3),(3,2),(4,1),(5,1)]
        let tree    = try HuffmanTree.build(weights)
        let regular = tree.codeTable()
        let canon   = tree.canonicalCodeTable()
        for (sym, code) in regular {
            XCTAssertEqual(code.count, canon[sym]?.count,
                           "canonical length mismatch for symbol \(sym)")
        }
    }

    func testCanonicalCodeTablePrefixFree() throws {
        let weights = [(1,5),(2,3),(3,2),(4,1),(5,1)]
        let tree    = try HuffmanTree.build(weights)
        let codes   = Array(tree.canonicalCodeTable().values)
        for i in codes.indices {
            for j in codes.indices where i != j {
                XCTAssertFalse(codes[j].hasPrefix(codes[i]))
            }
        }
    }

    func testCanonicalIsDeterministic() throws {
        let weights = [(1,5),(2,3),(3,2),(4,1)]
        let c1 = try HuffmanTree.build(weights).canonicalCodeTable()
        let c2 = try HuffmanTree.build(weights).canonicalCodeTable()
        for (sym, code) in c1 {
            XCTAssertEqual(code, c2[sym])
        }
    }

    // MARK: - decodeAll

    func testDecodeAllSingleA() throws {
        // A has code "0"
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let decoded = try tree.decodeAll("0", count: 1)
        XCTAssertEqual(decoded, [65])
    }

    func testDecodeAllAABC() throws {
        // A='0', C='10', B='11'  →  AABC = "0" + "0" + "11" + "10" = "001110"
        let tree    = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let decoded = try tree.decodeAll("001110", count: 4)
        XCTAssertEqual(decoded, [65, 65, 66, 67])
    }

    func testDecodeAllSingleLeafTree() throws {
        let tree    = try HuffmanTree.build([(symbol: 42, frequency: 5)])
        let decoded = try tree.decodeAll("000", count: 3)
        XCTAssertEqual(decoded, [42, 42, 42])
    }

    func testDecodeAllZeroSymbols() throws {
        let tree    = try HuffmanTree.build([(symbol: 1, frequency: 1)])
        let decoded = try tree.decodeAll("", count: 0)
        XCTAssertEqual(decoded, [])
    }

    func testDecodeAllRoundTrip() throws {
        let tree    = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let tbl     = tree.codeTable()
        let message = [65, 65, 65, 66, 66, 67]
        let bits    = message.map { tbl[$0]! }.joined()
        let decoded = try tree.decodeAll(bits, count: message.count)
        XCTAssertEqual(decoded, message)
    }

    func testDecodeAllExhaustedThrows() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        XCTAssertThrowsError(try tree.decodeAll("1011", count: 5))
    }

    func testDecodeAllFiveSymbolRoundTrip() throws {
        let weights = [(1,10),(2,5),(3,3),(4,2),(5,1)]
        let tree    = try HuffmanTree.build(weights)
        let tbl     = tree.codeTable()
        let message = [1,2,3,4,5,1,1,3,2]
        let bits    = message.map { tbl[$0]! }.joined()
        let decoded = try tree.decodeAll(bits, count: message.count)
        XCTAssertEqual(decoded, message)
    }

    // MARK: - weight

    func testWeight() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        XCTAssertEqual(tree.weight, 6)
    }

    func testWeightSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 0, frequency: 100)])
        XCTAssertEqual(tree.weight, 100)
    }

    // MARK: - depth

    func testDepthAABBC() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        XCTAssertEqual(tree.depth, 2)
    }

    func testDepthSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 1, frequency: 5)])
        XCTAssertEqual(tree.depth, 0)
    }

    func testDepthTwoEqualSymbols() throws {
        let tree = try HuffmanTree.build([(1,1),(2,1)])
        XCTAssertEqual(tree.depth, 1)
    }

    // MARK: - symbolCount

    func testSymbolCount() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        XCTAssertEqual(tree.symbolCount, 3)
    }

    func testSymbolCountSingle() throws {
        let tree = try HuffmanTree.build([(symbol: 7, frequency: 99)])
        XCTAssertEqual(tree.symbolCount, 1)
    }

    func testSymbolCountTen() throws {
        let weights = (1...10).map { (symbol: $0, frequency: $0) }
        let tree    = try HuffmanTree.build(weights)
        XCTAssertEqual(tree.symbolCount, 10)
    }

    // MARK: - leaves

    func testLeavesOrder() throws {
        // Tree: root → left=A(65), right=Internal → left=C(67), right=B(66)
        // In-order: A, C, B
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        let lvs  = tree.leaves()
        XCTAssertEqual(lvs.count, 3)
        XCTAssertEqual(lvs[0].0, 65)
        XCTAssertEqual(lvs[0].1, "0")
        XCTAssertEqual(lvs[1].0, 67)
        XCTAssertEqual(lvs[1].1, "10")
        XCTAssertEqual(lvs[2].0, 66)
        XCTAssertEqual(lvs[2].1, "11")
    }

    func testLeavesAllSymbolsAppearOnce() throws {
        let weights = [(1,5),(2,3),(3,2),(4,1),(5,1)]
        let tree    = try HuffmanTree.build(weights)
        let lvs     = tree.leaves()
        XCTAssertEqual(lvs.count, 5)
        let symbols = lvs.map { $0.0 }
        XCTAssertEqual(symbols.count, Set(symbols).count)
    }

    func testLeavesSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 99, frequency: 7)])
        let lvs  = tree.leaves()
        XCTAssertEqual(lvs.count, 1)
        XCTAssertEqual(lvs[0].0, 99)
        XCTAssertEqual(lvs[0].1, "0")
    }

    // MARK: - isValid

    func testIsValidBasic() throws {
        let tree = try HuffmanTree.build([(65,3),(66,2),(67,1)])
        XCTAssertTrue(tree.isValid())
    }

    func testIsValidSingleSymbol() throws {
        let tree = try HuffmanTree.build([(symbol: 1, frequency: 10)])
        XCTAssertTrue(tree.isValid())
    }

    func testIsValidLargeTree() throws {
        let weights = (1...15).map { (symbol: $0, frequency: $0 * 2) }
        let tree    = try HuffmanTree.build(weights)
        XCTAssertTrue(tree.isValid())
    }

    // MARK: - All equal weights

    func testAllEqualTwoSymbols() throws {
        let tree = try HuffmanTree.build([(1,1),(2,1)])
        XCTAssertEqual(tree.depth, 1)
        XCTAssertTrue(tree.isValid())
    }

    func testAllEqualFourSymbols() throws {
        let tree = try HuffmanTree.build([(1,1),(2,1),(3,1),(4,1)])
        XCTAssertEqual(tree.depth, 2)
        XCTAssertTrue(tree.isValid())
    }

    func testAllEqualEightSymbols() throws {
        let weights = [(1,1),(2,1),(3,1),(4,1),(5,1),(6,1),(7,1),(8,1)]
        let tree    = try HuffmanTree.build(weights)
        XCTAssertEqual(tree.depth, 3)
        XCTAssertTrue(tree.isValid())
    }

    // MARK: - Determinism

    func testDeterministicCodeTable() throws {
        let weights = [(1,5),(2,3),(3,2),(4,1),(5,1)]
        let t1 = try HuffmanTree.build(weights)
        let t2 = try HuffmanTree.build(weights)
        let c1 = t1.codeTable()
        let c2 = t2.codeTable()
        for (sym, code) in c1 {
            XCTAssertEqual(code, c2[sym])
        }
    }

    func testTieBreakingFourEqual() throws {
        let tree = try HuffmanTree.build([(1,1),(2,1),(3,1),(4,1)])
        XCTAssertEqual(tree.depth, 2)
        XCTAssertTrue(tree.isValid())
    }

    // MARK: - Byte-range round-trip

    func testByteRangeRoundTrip() throws {
        let weights = (0...15).map { (symbol: $0, frequency: $0 + 1) }
        let tree    = try HuffmanTree.build(weights)
        let tbl     = tree.codeTable()
        let message = Array(0...15)
        let bits    = message.map { tbl[$0]! }.joined()
        let decoded = try tree.decodeAll(bits, count: message.count)
        XCTAssertEqual(decoded, message)
    }

    // MARK: - Large round-trip

    func testLargeRoundTrip() throws {
        let weights = [(65,10),(66,8),(67,6),(68,4),(69,3),(70,2),(71,1)]
        let tree    = try HuffmanTree.build(weights)
        let tbl     = tree.codeTable()
        let message = [65,66,67,68,69,70,71,65,65,66]
        let bits    = message.map { tbl[$0]! }.joined()
        let decoded = try tree.decodeAll(bits, count: message.count)
        XCTAssertEqual(decoded, message)
    }
}
