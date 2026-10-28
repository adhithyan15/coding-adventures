import XCTest

@testable import HyperLogLog

final class HyperLogLogTests: XCTestCase {
  func testPrecisionBoundsAndEmptySketch() throws {
    let sketch = try HyperLogLog()
    XCTAssertEqual(sketch.precision, 14)
    XCTAssertEqual(sketch.numRegisters, 16_384)
    XCTAssertEqual(sketch.count(), 0)
    XCTAssertTrue(sketch.isEmpty)

    XCTAssertThrowsError(try HyperLogLog(precision: 3)) { error in
      XCTAssertEqual(error as? HyperLogLogError, .invalidPrecision(3))
    }
    XCTAssertThrowsError(try HyperLogLog(precision: 17)) { error in
      XCTAssertEqual(error as? HyperLogLogError, .invalidPrecision(17))
    }
  }

  func testPersistentUpdatesDuplicatesAndExactBytes() throws {
    let empty = try HyperLogLog(precision: 10)
    let once = empty.adding("same")
    var repeated = once
    for _ in 0..<1_000 {
      repeated = repeated.adding("same")
    }

    XCTAssertEqual(empty.count(), 0)
    XCTAssertEqual(once.count(), 1)
    XCTAssertEqual(repeated.count(), 1)
    XCTAssertEqual(once, empty.adding(Array("same".utf8)))
  }

  func testDeterministicAccuracy() throws {
    var sketch = try HyperLogLog()
    for value in 0..<10_000 {
      sketch = sketch.adding("element-\(value)")
    }
    XCTAssertGreaterThanOrEqual(sketch.count(), 9_500)
    XCTAssertLessThanOrEqual(sketch.count(), 10_500)
  }

  func testMergePreservesInputsAndRejectsPrecisionMismatch() throws {
    var left = try HyperLogLog(precision: 10)
    var right = try HyperLogLog(precision: 10)
    for value in 0..<500 {
      left = left.adding("left-\(value)")
      right = right.adding("right-\(value)")
    }
    let leftBefore = left
    let rightBefore = right
    let merged = try left.merged(with: right)

    XCTAssertEqual(left, leftBefore)
    XCTAssertEqual(right, rightBefore)
    XCTAssertGreaterThanOrEqual(merged.count(), 850)
    XCTAssertLessThanOrEqual(merged.count(), 1_150)

    let otherPrecision = try HyperLogLog(precision: 14)
    XCTAssertThrowsError(try left.merged(with: otherPrecision)) { error in
      XCTAssertEqual(
        error as? HyperLogLogError,
        .precisionMismatch(left: 10, right: 14)
      )
    }
  }

  func testMathHelpersAndStableErrors() throws {
    XCTAssertEqual(try HyperLogLog.memoryBytes(for: 14), 12_288)
    XCTAssertEqual(try HyperLogLog.optimalPrecision(for: 0.01), 14)
    XCTAssertEqual(try HyperLogLog.optimalPrecision(for: 1.0), 4)
    XCTAssertEqual(try HyperLogLog.optimalPrecision(for: Double.leastNonzeroMagnitude), 16)
    XCTAssertEqual(try HyperLogLog.errorRate(for: 14), 0.008125, accuracy: 0.000_001)

    XCTAssertThrowsError(try HyperLogLog.memoryBytes(for: 20)) { error in
      XCTAssertEqual(error as? HyperLogLogError, .invalidPrecision(20))
    }
    XCTAssertThrowsError(try HyperLogLog.optimalPrecision(for: 0)) { error in
      XCTAssertEqual(error as? HyperLogLogError, .invalidDesiredError(0))
    }
    XCTAssertThrowsError(try HyperLogLog.optimalPrecision(for: .infinity)) { error in
      XCTAssertEqual(error as? HyperLogLogError, .invalidDesiredError(.infinity))
    }
  }

  func testSmallAndLargeRangeCorrectionPaths() throws {
    let one = try HyperLogLog(precision: 4).adding("one")
    XCTAssertGreaterThanOrEqual(one.count(), 1)
    XCTAssertLessThanOrEqual(one.count(), 2)

    let registers = [UInt8](repeating: 25, count: 16)
    let large = HyperLogLog(testingPrecision: 4, registers: registers)
    XCTAssertGreaterThan(large.count(), 100_000_000)
    XCTAssertEqual(large.registerValuesForTesting.count, 16)
  }
}
