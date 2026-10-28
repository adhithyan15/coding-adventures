import XCTest

@testable import BloomFilter

final class BloomFilterTests: XCTestCase {
  func testDefaultsAndExplicitParameters() throws {
    let defaultFilter = try BloomFilter()
    XCTAssertEqual(defaultFilter.bitCount, 9_586)
    XCTAssertEqual(defaultFilter.hashCount, 7)
    XCTAssertEqual(defaultFilter.bitsSet, 0)
    XCTAssertEqual(defaultFilter.fillRatio, 0)
    XCTAssertEqual(defaultFilter.estimatedFalsePositiveRate, 0)
    XCTAssertFalse(defaultFilter.isOverCapacity)
    XCTAssertFalse(defaultFilter.contains("anything"))

    let explicit = try BloomFilter(bitCount: 10_000, hashCount: 7)
    XCTAssertEqual(explicit.sizeBytes, 1_250)
    XCTAssertFalse(explicit.isOverCapacity)
  }

  func testPersistentAddsDuplicatesAndExactBytes() throws {
    let empty = try BloomFilter(expectedItems: 100)
    let once = empty.adding("cafe\u{0301}")
    let twice = once.adding("cafe\u{0301}")

    XCTAssertFalse(empty.contains("cafe\u{0301}"))
    XCTAssertTrue(once.contains("cafe\u{0301}"))
    XCTAssertEqual(once.bitsSet, twice.bitsSet)
    XCTAssertEqual(once, empty.adding(Array("cafe\u{0301}".utf8)))
    XCTAssertFalse(once.contains("caf\u{00E9}"))
  }

  func testNoFalseNegativesAndBoundedFalsePositives() throws {
    var filter = try BloomFilter(expectedItems: 10_000, falsePositiveRate: 0.01)
    for value in 0..<10_000 {
      filter = filter.adding("present-\(value)")
    }
    for value in 0..<10_000 {
      XCTAssertTrue(filter.contains("present-\(value)"))
    }

    let falsePositives = (0..<10_000).reduce(0) { count, value in
      count + (filter.contains("absent-\(value)") ? 1 : 0)
    }
    XCTAssertLessThan(falsePositives, 200)
    XCTAssertGreaterThan(filter.bitsSet, 0)
    XCTAssertGreaterThan(filter.estimatedFalsePositiveRate, 0)
  }

  func testCapacityAndSizingHelpers() throws {
    XCTAssertEqual(
      try BloomFilter.optimalBitCount(
        expectedItems: 1_000_000,
        falsePositiveRate: 0.01
      ),
      9_585_059
    )
    XCTAssertEqual(
      try BloomFilter.optimalHashCount(bitCount: 9_585_059, expectedItems: 1_000_000),
      7
    )
    XCTAssertGreaterThan(
      try BloomFilter.capacity(memoryBytes: 1_000_000, falsePositiveRate: 0.01),
      0
    )

    var filter = try BloomFilter(expectedItems: 3)
    filter = filter.adding("a").adding("b").adding("c")
    XCTAssertFalse(filter.isOverCapacity)
    filter = filter.adding("d")
    XCTAssertTrue(filter.isOverCapacity)
  }

  func testStableValidationErrorsAndLimits() throws {
    XCTAssertThrowsError(try BloomFilter(expectedItems: 0)) { error in
      XCTAssertEqual(error as? BloomFilterError, .invalidExpectedItems(0))
    }
    for rate in [0.0, 1.0, .infinity, .nan] {
      XCTAssertThrowsError(try BloomFilter(falsePositiveRate: rate)) { error in
        XCTAssertEqual(
          error as? BloomFilterError,
          .invalidFalsePositiveRate(rate)
        )
      }
    }
    XCTAssertThrowsError(try BloomFilter(bitCount: 0, hashCount: 1)) { error in
      XCTAssertEqual(error as? BloomFilterError, .invalidBitCount(0))
    }
    XCTAssertThrowsError(try BloomFilter(bitCount: 1, hashCount: 0)) { error in
      XCTAssertEqual(error as? BloomFilterError, .invalidHashCount(0))
    }
    XCTAssertThrowsError(
      try BloomFilter(
        bitCount: BloomFilterLimits.maximumBitCount + 1,
        hashCount: 1
      )
    ) { error in
      XCTAssertEqual(
        error as? BloomFilterError,
        .allocationLimitExceeded(
          requestedBits: BloomFilterLimits.maximumBitCount + 1,
          maximumBits: BloomFilterLimits.maximumBitCount
        )
      )
    }
    XCTAssertThrowsError(
      try BloomFilter.capacity(memoryBytes: 0, falsePositiveRate: 0.01)
    ) { error in
      XCTAssertEqual(error as? BloomFilterError, .invalidMemoryBytes(0))
    }
    XCTAssertThrowsError(
      try BloomFilter.capacity(
        memoryBytes: BloomFilterLimits.maximumBitCount / 8,
        falsePositiveRate: Double(1).nextDown
      )
    ) { error in
      XCTAssertEqual(error as? BloomFilterError, .capacityOverflow)
    }
  }
}
