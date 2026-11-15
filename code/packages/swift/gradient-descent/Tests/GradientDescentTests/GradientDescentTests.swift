import XCTest

@testable import GradientDescent

/// Deterministic SplitMix64 PRNG: seeded, no wall clock, identical across runs.
private struct SplitMix64 {
  private var state: UInt64

  init(seed: UInt64) {
    state = seed
  }

  mutating func next() -> UInt64 {
    state &+= 0x9E37_79B9_7F4A_7C15
    var mixed = state
    mixed = (mixed ^ (mixed >> 30)) &* 0xBF58_476D_1CE4_E5B9
    mixed = (mixed ^ (mixed >> 27)) &* 0x94D0_49BB_1331_11EB
    return mixed ^ (mixed >> 31)
  }

  /// Uniform integer in `0..<bound`.
  mutating func nextInt(_ bound: Int) -> Int {
    Int(next() % UInt64(bound))
  }

  /// Uniform integer in `range`.
  mutating func nextInt(in range: ClosedRange<Int>) -> Int {
    range.lowerBound + nextInt(range.upperBound - range.lowerBound + 1)
  }
}

final class GradientDescentTests: XCTestCase {
  // MARK: Helpers

  private func assertThrows(
    _ expected: GradientDescentErrorCode,
    file: StaticString = #filePath,
    line: UInt = #line,
    _ body: () throws -> [Double]
  ) {
    do {
      let result = try body()
      XCTFail("expected \(expected.errorId), got \(result)", file: file, line: line)
    } catch let error as GradientDescentError {
      XCTAssertEqual(error.code, expected, file: file, line: line)
    } catch {
      XCTFail("unexpected error type \(type(of: error))", file: file, line: line)
    }
  }

  /// The unfused reference: two separately rounded operations.
  private func reference(_ weights: [Double], _ gradients: [Double], _ rate: Double) -> [Double] {
    zip(weights, gradients).map { weight, gradient in
      let step = rate * gradient
      return weight - step
    }
  }

  // MARK: Parity vectors

  func testSpecVectorWithinToleranceAndBitExact() throws {
    let weights = [1.0, -0.5, 2.0]
    let gradients = [0.1, -0.2, 0.0]
    let result = try GradientDescent.sgd(weights, gradients, learningRate: 0.1)
    let expected = [0.99, -0.48, 2.0]
    XCTAssertEqual(result.count, expected.count)
    for (actual, want) in zip(result, expected) {
      XCTAssertEqual(actual, want, accuracy: 1e-12)
    }
    let unfused = reference(weights, gradients, 0.1)
    XCTAssertEqual(result.map(\.bitPattern), unfused.map(\.bitPattern))
  }

  func testDyadicWitnessIsExact() throws {
    let result = try GradientDescent.sgd([1.0, -0.5, 2.0], [0.5, -0.25, 0.0], learningRate: 0.5)
    XCTAssertEqual(result, [0.75, -0.375, 2.0])
  }

  func testUnlabeledOverloadMatchesLabeled() throws {
    let weights = [1.0, -0.5, 2.0]
    let gradients = [0.1, -0.2, 0.0]
    let labeled = try GradientDescent.sgd(weights, gradients, learningRate: 0.1)
    let unlabeled = try GradientDescent.sgd(weights, gradients, 0.1)
    XCTAssertEqual(labeled.map(\.bitPattern), unlabeled.map(\.bitPattern))
  }

  func testUnlabeledOverloadPropagatesErrors() {
    assertThrows(.emptyInput) { try GradientDescent.sgd([], [], 0.1) }
  }

  func testZeroLearningRateIsIdentity() throws {
    let weights = [3.25, -7.5, 0.0]
    XCTAssertEqual(try GradientDescent.sgd(weights, [1.0, 2.0, 3.0], learningRate: 0.0), weights)
  }

  func testNegativeLearningRateAscends() throws {
    XCTAssertEqual(try GradientDescent.sgd([1.0], [2.0], learningRate: -0.5), [2.0])
  }

  func testSingleElement() throws {
    XCTAssertEqual(try GradientDescent.sgd([4.0], [1.0], learningRate: 1.0), [3.0])
  }

  func testSubnormalAndSignedZeroInputsAreAccepted() throws {
    let tiny = Double.leastNonzeroMagnitude
    let result = try GradientDescent.sgd([tiny, -0.0], [tiny, 0.0], learningRate: 0.5)
    XCTAssertEqual(result.count, 2)
    XCTAssertTrue(result.allSatisfy(\.isFinite))
  }

  // MARK: Class 1 — validation precedence

  func testMismatchBeatsEmpty() {
    assertThrows(.lengthMismatch) { try GradientDescent.sgd([], [1.0], learningRate: 0.1) }
    assertThrows(.lengthMismatch) { try GradientDescent.sgd([1.0], [], learningRate: 0.1) }
  }

  func testMismatchBeatsTooLarge() {
    let huge = [Double](repeating: 0.0, count: GradientDescentConstants.maxVectorLength + 1)
    assertThrows(.lengthMismatch) { try GradientDescent.sgd(huge, [1.0], learningRate: 0.1) }
  }

  func testEmptyBeatsNonFiniteLearningRate() {
    assertThrows(.emptyInput) { try GradientDescent.sgd([], [], learningRate: .nan) }
  }

  func testTooLargeBeatsNonFiniteLearningRate() {
    let huge = [Double](repeating: .nan, count: GradientDescentConstants.maxVectorLength + 1)
    assertThrows(.inputTooLarge) { try GradientDescent.sgd(huge, huge, learningRate: .nan) }
  }

  func testNonFiniteLearningRateBeatsNonFiniteInput() {
    assertThrows(.nonFiniteLearningRate) {
      try GradientDescent.sgd([.nan], [.nan], learningRate: .nan)
    }
  }

  func testNonFiniteInputBeatsOverflowViaFullPrePass() {
    // Index 0 would overflow, but index 1 is NaN: the pre-pass must win.
    assertThrows(.nonFiniteInput) {
      try GradientDescent.sgd([1e308, .nan], [-1e308, 1.0], learningRate: 1.0)
    }
  }

  // MARK: Class 2 — CWE-400 resource ceiling

  func testOneOverCeilingIsRejected() {
    let length = GradientDescentConstants.maxVectorLength + 1
    XCTAssertEqual(length, 1_048_577)
    let vector = [Double](repeating: 1.0, count: length)
    assertThrows(.inputTooLarge) { try GradientDescent.sgd(vector, vector, learningRate: 0.1) }
  }

  func testExactlyAtCeilingIsAccepted() throws {
    let length = GradientDescentConstants.maxVectorLength
    XCTAssertEqual(length, 1_048_576)
    let weights = [Double](repeating: 1.0, count: length)
    let gradients = [Double](repeating: 0.5, count: length)
    let result = try GradientDescent.sgd(weights, gradients, learningRate: 0.5)
    XCTAssertEqual(result.count, length)
    XCTAssertEqual(result.first, 0.75)
    XCTAssertEqual(result.last, 0.75)
  }

  // MARK: Class 3 — CWE-682 non-finite poisoning and overflow

  func testNonFiniteLearningRatesRejected() {
    for rate in [Double.nan, .infinity, -.infinity] {
      assertThrows(.nonFiniteLearningRate) {
        try GradientDescent.sgd([1.0], [1.0], learningRate: rate)
      }
    }
  }

  func testNonFiniteWeightsRejected() {
    for bad in [Double.nan, .infinity, -.infinity] {
      assertThrows(.nonFiniteInput) {
        try GradientDescent.sgd([1.0, bad], [1.0, 1.0], learningRate: 0.1)
      }
    }
  }

  func testNonFiniteGradientsRejected() {
    for bad in [Double.nan, .infinity, -.infinity] {
      assertThrows(.nonFiniteInput) {
        try GradientDescent.sgd([1.0, 1.0], [bad, 1.0], learningRate: 0.1)
      }
    }
  }

  func testUpdateOverflowRejected() {
    assertThrows(.numericOverflow) {
      try GradientDescent.sgd([1e308], [-1e308], learningRate: 1.0)
    }
  }

  func testNegativeUpdateOverflowRejected() {
    assertThrows(.numericOverflow) {
      try GradientDescent.sgd([-1e308], [1e308], learningRate: 1.0)
    }
  }

  func testStepOverflowRejected() {
    assertThrows(.numericOverflow) {
      try GradientDescent.sgd([0.0], [1e308], learningRate: 10.0)
    }
  }

  // MARK: Class 4 — alias safety and purity

  func testAliasedInputsAreCorrectAndUnchanged() throws {
    let values = [1.0, -2.0, 0.5]
    let snapshot = values
    let result = try GradientDescent.sgd(values, values, learningRate: 0.5)
    XCTAssertEqual(result, [0.5, -1.0, 0.25])
    XCTAssertEqual(values, snapshot)
  }

  func testMutatingResultDoesNotAffectInput() throws {
    let weights = [1.0, 2.0]
    var result = try GradientDescent.sgd(weights, [0.0, 0.0], learningRate: 0.1)
    result[0] = 99.0
    XCTAssertEqual(weights, [1.0, 2.0])
  }

  func testMutatingInputDoesNotAffectResult() throws {
    var weights = [1.0, 2.0]
    var gradients = [0.5, 0.5]
    let result = try GradientDescent.sgd(weights, gradients, learningRate: 1.0)
    weights[0] = -100.0
    gradients[1] = 100.0
    XCTAssertEqual(result, [0.5, 1.5])
  }

  // MARK: Class 5 — CWE-209 payload-blind errors

  func testErrorsArePayloadBlind() {
    let length = 7
    var weights = [Double](repeating: 13_371_337.0, count: length)
    weights[3] = .nan
    let gradients = [Double](repeating: 13_371_337.0, count: length)
    let rate = 0.4242
    do {
      _ = try GradientDescent.sgd(weights, gradients, learningRate: rate)
      XCTFail("expected an error")
    } catch let error as GradientDescentError {
      XCTAssertEqual(error.code, .nonFiniteInput)
      let surfaces = [
        error.message, error.errorId, error.description,
        String(describing: error), String(reflecting: error), "\(error)",
        error.localizedDescription,
      ]
      let sentinels = ["13371337", "1.3371337", "0.4242", "4242", String(length), "nan", "NaN"]
      for surface in surfaces {
        for sentinel in sentinels {
          XCTAssertFalse(surface.contains(sentinel), "\(surface) leaks \(sentinel)")
        }
      }
    } catch {
      XCTFail("unexpected error type")
    }
  }

  // MARK: Error surface

  func testErrorIdsAndMessagesMatchContract() {
    let expected: [(GradientDescentErrorCode, String, String)] = [
      (.lengthMismatch, "length-mismatch", "weights and gradients must have the same length"),
      (.emptyInput, "empty-input", "weights and gradients must not be empty"),
      (
        .inputTooLarge, "input-too-large",
        "input vectors exceed the maximum supported length"
      ),
      (
        .nonFiniteLearningRate, "non-finite-learning-rate",
        "learning rate must be a finite number"
      ),
      (.nonFiniteInput, "non-finite-input", "weights and gradients must be finite numbers"),
      (.numericOverflow, "numeric-overflow", "updated weight is not a finite number"),
    ]
    XCTAssertEqual(GradientDescentErrorCode.allCases, expected.map { $0.0 })
    for (code, errorId, message) in expected {
      let error = GradientDescentError(code)
      XCTAssertEqual(code.errorId, errorId)
      XCTAssertEqual(code.message, message)
      XCTAssertEqual(error.code, code)
      XCTAssertEqual(error.errorId, errorId)
      XCTAssertEqual(error.message, message)
      XCTAssertEqual(error.description, errorId + ": " + message)
    }
  }

  func testErrorEquality() {
    XCTAssertEqual(GradientDescentError(.emptyInput), GradientDescentError(.emptyInput))
    XCTAssertNotEqual(GradientDescentError(.emptyInput), GradientDescentError(.lengthMismatch))
  }

  func testPlaceholdersAndConstants() {
    XCTAssertEqual(GradientDescentMessages.redactedPlaceholder, "<redacted>")
    XCTAssertEqual(GradientDescentMessages.invalidPlaceholder, "<invalid>")
    XCTAssertEqual(GradientDescentConstants.maxVectorLength, 1_048_576)
    XCTAssertEqual(GradientDescentConstants.version, "0.1.0")
  }

  // MARK: Properties

  func testDyadicMultiStepLinearity() throws {
    var rng = SplitMix64(seed: 0x4D4C_3032)
    for _ in 0..<1_000 {
      let length = rng.nextInt(in: 1...8)
      let weights = (0..<length).map { _ in Double(rng.nextInt(in: -64...64)) / 16.0 }
      let gradients = (0..<length).map { _ in Double(rng.nextInt(in: -64...64)) / 16.0 }
      let rateA = Double(rng.nextInt(in: -8...8)) / 8.0
      let rateB = Double(rng.nextInt(in: -8...8)) / 8.0
      let twice = try GradientDescent.sgd(
        try GradientDescent.sgd(weights, gradients, learningRate: rateA),
        gradients,
        learningRate: rateB
      )
      let once = try GradientDescent.sgd(weights, gradients, learningRate: rateA + rateB)
      XCTAssertEqual(twice.map(\.bitPattern), once.map(\.bitPattern))
    }
  }

  func testAdversarialMutationFuzz() {
    let specials: [Double] = [
      .nan, .infinity, -.infinity, 0.0, -0.0, 1e308, -1e308,
      .greatestFiniteMagnitude, -.greatestFiniteMagnitude,
      .leastNonzeroMagnitude, -.leastNonzeroMagnitude, .leastNormalMagnitude,
      1.0, -1.0, 0.5,
    ]
    var rng = SplitMix64(seed: 0x00C0_FFEE)
    func draw() -> Double {
      if rng.nextInt(2) == 0 {
        return specials[rng.nextInt(specials.count)]
      }
      return Double(rng.nextInt(in: -1_000...1_000)) / 7.0
    }
    let knownCodes = Set(GradientDescentErrorCode.allCases)
    var successes = 0
    var failures = 0
    for _ in 0..<10_000 {
      let weightCount = rng.nextInt(in: 0...8)
      let gradientCount = rng.nextInt(4) == 0 ? rng.nextInt(in: 0...8) : weightCount
      let weights = (0..<weightCount).map { _ in draw() }
      let gradients = (0..<gradientCount).map { _ in draw() }
      let rate = draw()
      do {
        let result = try GradientDescent.sgd(weights, gradients, learningRate: rate)
        XCTAssertEqual(result.count, weights.count)
        XCTAssertTrue(result.allSatisfy(\.isFinite))
        let expected = reference(weights, gradients, rate)
        XCTAssertEqual(result.map(\.bitPattern), expected.map(\.bitPattern))
        successes += 1
      } catch let error as GradientDescentError {
        XCTAssertTrue(knownCodes.contains(error.code))
        failures += 1
      } catch {
        XCTFail("unexpected error type \(type(of: error))")
      }
    }
    XCTAssertGreaterThan(successes, 0)
    XCTAssertGreaterThan(failures, 0)
  }
}
