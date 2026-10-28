import Foundation
import HashFunctions

/// Stable validation failures for the DT21 sketch.
public enum HyperLogLogError: Error, Equatable, Sendable {
  case invalidPrecision(Int)
  case precisionMismatch(left: Int, right: Int)
  case invalidDesiredError(Double)
}

/// Fixed production limits for precision and register allocation.
public enum HyperLogLogLimits {
  public static let defaultPrecision = 14
  public static let minimumPrecision = 4
  public static let maximumPrecision = 16
}

/// A persistent HyperLogLog cardinality sketch.
///
/// Values are hashed with DT17 FNV-1a 64-bit followed by the fixed MurmurHash3
/// finalizer. Every update and merge returns a new value, leaving the receiver
/// unchanged. The package accepts only exact bytes or UTF-8 strings; it never
/// uses Swift's randomized `Hasher` or a caller-provided executable encoder.
public struct HyperLogLog: Equatable, Sendable {
  public let precision: Int
  private let registers: [UInt8]

  public init(precision: Int = HyperLogLogLimits.defaultPrecision) throws {
    guard
      (HyperLogLogLimits.minimumPrecision...HyperLogLogLimits.maximumPrecision)
        .contains(precision)
    else {
      throw HyperLogLogError.invalidPrecision(precision)
    }
    self.precision = precision
    self.registers = [UInt8](repeating: 0, count: 1 << precision)
  }

  private init(precision: Int, registers: [UInt8]) {
    self.precision = precision
    self.registers = registers
  }

  /// Return a new sketch that has observed these exact bytes.
  public func adding(_ bytes: [UInt8]) -> Self {
    var updated = registers
    let hash = fmix64(fnv1a64(bytes))
    let bucket = Int(hash >> UInt64(64 - precision))
    let remainingBits = 64 - precision
    let mask = (UInt64(1) << UInt64(remainingBits)) - 1
    let remaining = hash & mask
    let leadingZeros =
      remaining == 0
      ? remainingBits
      : remaining.leadingZeroBitCount - precision
    let rank = UInt8(leadingZeros + 1)
    if rank > updated[bucket] {
      updated[bucket] = rank
    }
    return Self(precision: precision, registers: updated)
  }

  /// Return a new sketch that has observed the string's exact UTF-8 bytes.
  public func adding(_ text: String) -> Self {
    adding(Array(text.utf8))
  }

  /// Estimate the number of distinct values observed by this sketch.
  public func count() -> UInt64 {
    let m = Double(numRegisters)
    let harmonicSum = registers.reduce(0.0) { partial, register in
      partial + pow(2.0, -Double(register))
    }
    var estimate = alpha(for: numRegisters) * m * m / harmonicSum

    if estimate <= 2.5 * m {
      let emptyRegisters = registers.reduce(0) { count, register in
        count + (register == 0 ? 1 : 0)
      }
      if emptyRegisters > 0 {
        estimate = m * log(m / Double(emptyRegisters))
      }
    }

    let twoTo64 = 18_446_744_073_709_551_616.0
    if estimate > twoTo64 / 30.0 {
      let ratio = 1.0 - estimate / twoTo64
      if ratio > 0 {
        estimate = -twoTo64 * log(ratio)
      }
    }

    guard estimate.isFinite, estimate > 0 else { return 0 }
    if estimate >= twoTo64 { return UInt64.max }
    return UInt64(estimate.rounded())
  }

  /// Return the register-wise union of two equal-precision sketches.
  public func merged(with other: Self) throws -> Self {
    guard precision == other.precision else {
      throw HyperLogLogError.precisionMismatch(
        left: precision,
        right: other.precision
      )
    }
    let mergedRegisters = zip(registers, other.registers).map { Swift.max($0, $1) }
    return Self(precision: precision, registers: mergedRegisters)
  }

  public var numRegisters: Int { registers.count }
  public var isEmpty: Bool { registers.allSatisfy { $0 == 0 } }
  public var errorRate: Double { 1.04 / sqrt(Double(numRegisters)) }

  public static func errorRate(for precision: Int) throws -> Double {
    try validate(precision: precision)
    return 1.04 / sqrt(Double(1 << precision))
  }

  public static func memoryBytes(for precision: Int) throws -> Int {
    try validate(precision: precision)
    return ((1 << precision) * 6) / 8
  }

  public static func optimalPrecision(for desiredError: Double) throws -> Int {
    guard desiredError.isFinite, desiredError > 0 else {
      throw HyperLogLogError.invalidDesiredError(desiredError)
    }
    let requiredRegisters = pow(1.04 / desiredError, 2.0)
    if !requiredRegisters.isFinite {
      return HyperLogLogLimits.maximumPrecision
    }
    let precision = Int(ceil(log2(requiredRegisters)))
    return min(
      HyperLogLogLimits.maximumPrecision,
      max(HyperLogLogLimits.minimumPrecision, precision)
    )
  }

  private static func validate(precision: Int) throws {
    guard
      (HyperLogLogLimits.minimumPrecision...HyperLogLogLimits.maximumPrecision)
        .contains(precision)
    else {
      throw HyperLogLogError.invalidPrecision(precision)
    }
  }

  internal init(testingPrecision precision: Int, registers: [UInt8]) {
    self.init(precision: precision, registers: registers)
  }

  internal var registerValuesForTesting: [UInt8] { registers }
}

private func alpha(for registerCount: Int) -> Double {
  switch registerCount {
  case 16: 0.673
  case 32: 0.697
  case 64: 0.709
  default: 0.7213 / (1.0 + 1.079 / Double(registerCount))
  }
}

private func fmix64(_ input: UInt64) -> UInt64 {
  var value = input
  value ^= value >> 33
  value = value &* 0xFF51_AFD7_ED55_8CCD
  value ^= value >> 33
  value = value &* 0xC4CE_B9FE_1A85_EC53
  value ^= value >> 33
  return value
}
