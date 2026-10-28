import Foundation
import HashFunctions

/// Stable validation failures for the DT22 filter.
public enum BloomFilterError: Error, Equatable, Sendable {
  case invalidExpectedItems(Int)
  case invalidFalsePositiveRate(Double)
  case invalidBitCount(Int)
  case invalidHashCount(Int)
  case invalidMemoryBytes(Int)
  case allocationLimitExceeded(requestedBits: Int, maximumBits: Int)

  public static func == (left: Self, right: Self) -> Bool {
    switch (left, right) {
    case (.invalidExpectedItems(let leftValue), .invalidExpectedItems(let rightValue)):
      leftValue == rightValue
    case (
      .invalidFalsePositiveRate(let leftValue),
      .invalidFalsePositiveRate(let rightValue)
    ):
      leftValue == rightValue || (leftValue.isNaN && rightValue.isNaN)
    case (.invalidBitCount(let leftValue), .invalidBitCount(let rightValue)):
      leftValue == rightValue
    case (.invalidHashCount(let leftValue), .invalidHashCount(let rightValue)):
      leftValue == rightValue
    case (.invalidMemoryBytes(let leftValue), .invalidMemoryBytes(let rightValue)):
      leftValue == rightValue
    case (
      .allocationLimitExceeded(let leftBits, let leftMaximum),
      .allocationLimitExceeded(let rightBits, let rightMaximum)
    ):
      leftBits == rightBits && leftMaximum == rightMaximum
    default:
      false
    }
  }
}

/// Fixed defaults and resource limits for filter construction.
public enum BloomFilterLimits {
  public static let defaultExpectedItems = 1_000
  public static let defaultFalsePositiveRate = 0.01
  public static let maximumBitCount = 134_217_728
  public static let maximumHashCount = 64
}

/// A persistent Bloom filter with deterministic DT17 double hashing.
///
/// Insertions return new values and never modify the receiver. Inputs are exact
/// bytes or exact UTF-8 strings; the implementation never uses Swift's
/// randomized `Hasher` or a caller-provided executable encoder.
public struct BloomFilter: Equatable, Sendable {
  public let bitCount: Int
  public let hashCount: Int
  public let expectedItems: Int
  public let itemsAdded: Int
  private let bits: [UInt8]
  private let storedBitsSet: Int

  public init(
    expectedItems: Int = BloomFilterLimits.defaultExpectedItems,
    falsePositiveRate: Double = BloomFilterLimits.defaultFalsePositiveRate
  ) throws {
    let bitCount = try Self.optimalBitCount(
      expectedItems: expectedItems,
      falsePositiveRate: falsePositiveRate
    )
    let hashCount = try Self.optimalHashCount(
      bitCount: bitCount,
      expectedItems: expectedItems
    )
    self.init(
      bitCount: bitCount,
      hashCount: hashCount,
      expectedItems: expectedItems,
      itemsAdded: 0,
      bits: [UInt8](repeating: 0, count: (bitCount + 7) / 8),
      bitsSet: 0
    )
  }

  public init(bitCount: Int, hashCount: Int) throws {
    try Self.validate(bitCount: bitCount)
    try Self.validate(hashCount: hashCount)
    self.init(
      bitCount: bitCount,
      hashCount: hashCount,
      expectedItems: 0,
      itemsAdded: 0,
      bits: [UInt8](repeating: 0, count: (bitCount + 7) / 8),
      bitsSet: 0
    )
  }

  private init(
    bitCount: Int,
    hashCount: Int,
    expectedItems: Int,
    itemsAdded: Int,
    bits: [UInt8],
    bitsSet: Int
  ) {
    self.bitCount = bitCount
    self.hashCount = hashCount
    self.expectedItems = expectedItems
    self.itemsAdded = itemsAdded
    self.bits = bits
    self.storedBitsSet = bitsSet
  }

  /// Return a new filter that has observed these exact bytes.
  public func adding(_ bytes: [UInt8]) -> Self {
    var updated = bits
    var updatedBitsSet = storedBitsSet
    for index in hashIndices(for: bytes) {
      let byteIndex = index / 8
      let mask = UInt8(1 << (index % 8))
      if updated[byteIndex] & mask == 0 {
        updated[byteIndex] |= mask
        updatedBitsSet += 1
      }
    }
    return Self(
      bitCount: bitCount,
      hashCount: hashCount,
      expectedItems: expectedItems,
      itemsAdded: itemsAdded == Int.max ? Int.max : itemsAdded + 1,
      bits: updated,
      bitsSet: updatedBitsSet
    )
  }

  /// Return a new filter that has observed the string's exact UTF-8 bytes.
  public func adding(_ text: String) -> Self {
    adding(Array(text.utf8))
  }

  public func contains(_ bytes: [UInt8]) -> Bool {
    hashIndices(for: bytes).allSatisfy { index in
      let byteIndex = index / 8
      let mask = UInt8(1 << (index % 8))
      return bits[byteIndex] & mask != 0
    }
  }

  public func contains(_ text: String) -> Bool {
    contains(Array(text.utf8))
  }

  public var bitsSet: Int { storedBitsSet }
  public var fillRatio: Double { Double(storedBitsSet) / Double(bitCount) }
  public var estimatedFalsePositiveRate: Double {
    storedBitsSet == 0 ? 0 : pow(fillRatio, Double(hashCount))
  }
  public var isOverCapacity: Bool {
    expectedItems > 0 && itemsAdded > expectedItems
  }
  public var sizeBytes: Int { bits.count }

  public static func optimalBitCount(
    expectedItems: Int,
    falsePositiveRate: Double
  ) throws -> Int {
    try validate(expectedItems: expectedItems)
    try validate(falsePositiveRate: falsePositiveRate)
    let logTwo = log(2.0)
    let requested = ceil(
      -Double(expectedItems) * log(falsePositiveRate) / (logTwo * logTwo)
    )
    guard requested.isFinite, requested <= Double(Int.max) else {
      throw BloomFilterError.allocationLimitExceeded(
        requestedBits: Int.max,
        maximumBits: BloomFilterLimits.maximumBitCount
      )
    }
    let bitCount = Int(requested)
    try validate(bitCount: bitCount)
    return bitCount
  }

  public static func optimalHashCount(
    bitCount: Int,
    expectedItems: Int
  ) throws -> Int {
    try validate(bitCount: bitCount)
    try validate(expectedItems: expectedItems)
    let requested = max(
      1,
      Int((Double(bitCount) / Double(expectedItems) * log(2.0)).rounded())
    )
    try validate(hashCount: requested)
    return requested
  }

  public static func capacity(
    memoryBytes: Int,
    falsePositiveRate: Double
  ) throws -> Int {
    guard memoryBytes > 0, memoryBytes <= BloomFilterLimits.maximumBitCount / 8 else {
      throw BloomFilterError.invalidMemoryBytes(memoryBytes)
    }
    try validate(falsePositiveRate: falsePositiveRate)
    let bitCount = Double(memoryBytes * 8)
    let logTwo = log(2.0)
    return Int(floor(-bitCount * logTwo * logTwo / log(falsePositiveRate)))
  }

  private func hashIndices(for bytes: [UInt8]) -> [Int] {
    let first = UInt64(fmix32(fnv1a32(bytes)))
    let second = UInt64(fmix32(UInt32(truncatingIfNeeded: djb2(bytes))) | 1)
    let modulus = UInt64(bitCount)
    return (0..<hashCount).map { index in
      Int((first &+ UInt64(index) &* second) % modulus)
    }
  }

  private static func validate(expectedItems: Int) throws {
    guard expectedItems > 0 else {
      throw BloomFilterError.invalidExpectedItems(expectedItems)
    }
  }

  private static func validate(falsePositiveRate: Double) throws {
    guard falsePositiveRate.isFinite,
      falsePositiveRate > 0,
      falsePositiveRate < 1
    else {
      throw BloomFilterError.invalidFalsePositiveRate(falsePositiveRate)
    }
  }

  private static func validate(bitCount: Int) throws {
    guard bitCount > 0 else { throw BloomFilterError.invalidBitCount(bitCount) }
    guard bitCount <= BloomFilterLimits.maximumBitCount else {
      throw BloomFilterError.allocationLimitExceeded(
        requestedBits: bitCount,
        maximumBits: BloomFilterLimits.maximumBitCount
      )
    }
  }

  private static func validate(hashCount: Int) throws {
    guard hashCount > 0, hashCount <= BloomFilterLimits.maximumHashCount else {
      throw BloomFilterError.invalidHashCount(hashCount)
    }
  }
}

private func fmix32(_ input: UInt32) -> UInt32 {
  var value = input
  value ^= value >> 16
  value = value &* 0x85EB_CA6B
  value ^= value >> 13
  value = value &* 0xC2B2_AE35
  value ^= value >> 16
  return value
}
