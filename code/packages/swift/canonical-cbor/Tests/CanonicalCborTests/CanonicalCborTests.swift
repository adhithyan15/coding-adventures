// ============================================================================
// CanonicalCborTests.swift -- CVE Hardening, Property Tests & Mutation Fuzzing
// ============================================================================

import CanonicalCbor
import XCTest

final class CanonicalCborTests: XCTestCase {

  // ========================================================================
  // Messages, Enums, and Types Unit Tests
  // ========================================================================

  func testMessagesAndPlaceholders() {
    XCTAssertEqual(Messages.redactedPlaceholder, "<redacted>")
    XCTAssertEqual(Messages.invalidPlaceholder, "<invalid>")
    XCTAssertEqual(Messages.unknownErrorIdMessage, "canonical-cbor: unknown error id")
    XCTAssertEqual(Messages.allCborErrorIds.count, 14)
    XCTAssertEqual(Messages.allDiagnosticMessages.count, 14)

    for id in Messages.allCborErrorIds {
      let msg = Messages.message(for: id)
      XCTAssertTrue(msg.hasPrefix("canonical-cbor: "), "Diagnostic prefix missing for \(id)")
    }

    let unknownMsg = Messages.message(for: .unknown)
    XCTAssertEqual(unknownMsg, Messages.unknownErrorIdMessage)

    let err = CborError(.unexpectedEof)
    XCTAssertEqual(err.id, .unexpectedEof)
    XCTAssertTrue(err.description.contains("unexpected-eof"))
  }

  func testSpanAndTypes() {
    let validSpan = Span(start: 5, finish: 15)
    XCTAssertNotNil(validSpan)
    XCTAssertEqual(validSpan?.start, 5)
    XCTAssertEqual(validSpan?.finish, 15)
    XCTAssertEqual(validSpan?.length, 10)
    XCTAssertEqual(validSpan?.description, "[5, 15)")

    let invalidSpan = Span(start: 15, finish: 5)
    XCTAssertNil(invalidSpan)

    let entry = CborMapEntry(key: .unsigned(1), value: .text("hello"))
    XCTAssertEqual(entry.description, "1: \"hello\"")

    // Value descriptions
    XCTAssertEqual(CborValue.unsigned(42).description, "42")
    XCTAssertEqual(CborValue.negative(9).description, "-1-9")
    XCTAssertEqual(CborValue.bytes([1, 2, 3]).description, "bytes(3)")
    XCTAssertEqual(CborValue.text("abc").description, "\"abc\"")
    XCTAssertEqual(CborValue.array([.unsigned(1), .unsigned(2)]).description, "[1, 2]")
    XCTAssertEqual(
      CborValue.map([CborMapEntry(key: .unsigned(1), value: .unsigned(2))]).description,
      "{1: 2}"
    )
    XCTAssertEqual(CborValue.tag(42, .unsigned(1)).description, "tag(42, 1)")
    XCTAssertEqual(CborValue.boolean(true).description, "true")
    XCTAssertEqual(CborValue.boolean(false).description, "false")
    XCTAssertEqual(CborValue.null.description, "null")
  }

  func testResultWrappers() {
    let val = CborValue.unsigned(10)
    let encRes = encodeCheckedResult(val)
    guard case .success(let encBytes) = encRes else {
      XCTFail("encodeCheckedResult failed")
      return
    }
    XCTAssertEqual(encBytes, [0x0A])

    let badEnc = encodeCheckedResult(FixtureLoader.buildGenerated("nested-array:129"))
    guard case .failure(let encErr) = badEnc else {
      XCTFail("Expected failure on nested array 129")
      return
    }
    XCTAssertEqual(encErr.id, .encodeTooDeep)

    let decRes = decodeResult(encBytes)
    guard case .success(let decVal) = decRes else {
      XCTFail("decodeResult failed")
      return
    }
    XCTAssertEqual(decVal, val)

    let badDec = decodeResult([])
    guard case .failure(let err) = badDec else {
      XCTFail("Expected failure on empty bytes")
      return
    }
    XCTAssertEqual(err.id, .unexpectedEof)

    let spanRes = decodeMapSpansResult([0xA0])
    guard case .success(let mapResult) = spanRes else {
      XCTFail("decodeMapSpansResult failed")
      return
    }
    XCTAssertEqual(mapResult, .map([]))

    let badSpanRes = decodeMapSpansResult([])
    guard case .failure(let spanErr) = badSpanRes else {
      XCTFail("Expected failure on empty bytes for decodeMapSpansResult")
      return
    }
    XCTAssertEqual(spanErr.id, .unexpectedEof)
  }

  // ========================================================================
  // CVE-1: Integer Overflow Traps (UInt64 -> Int Bounds Defense)
  // ========================================================================

  func testCVE1HostileIntegerLengths() {
    // Enormous byte string lengths (>= 2^63) must not crash Int conversion with fatal error
    let hostileLengths: [[UInt8]] = [
      // Major 2: Bytes
      [0x5B, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
      [0x5B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      // Major 3: Text
      [0x7B, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
      [0x7B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      // Major 4: Array
      [0x9B, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
      [0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      // Major 5: Map
      [0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
      [0xBB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    ]

    for wire in hostileLengths {
      do {
        _ = try decode(wire)
        XCTFail("Expected lengthTooLarge for hostile wire length")
      } catch let err as CborError {
        XCTAssertEqual(err.id, .lengthTooLarge)
      } catch {
        XCTFail("Unexpected error: \(error)")
      }
    }
  }

  // ========================================================================
  // CVE-2: Recursion Limits (Nesting Depth = 128)
  // ========================================================================

  func testCVE2RecursionLimits() throws {
    // Array nesting 128 succeeds, 129 fails
    let arr128 = FixtureLoader.buildGenerated("nested-array:128")
    let wire128 = try encodeChecked(arr128)
    let dec128 = try decode(wire128)
    XCTAssertEqual(dec128, arr128)

    let arr129 = FixtureLoader.buildGenerated("nested-array:129")
    XCTAssertThrowsError(try encodeChecked(arr129)) { err in
      XCTAssertEqual((err as? CborError)?.id, .encodeTooDeep)
    }

    var wire129 = [UInt8](repeating: 0x81, count: 129)
    wire129.append(0xF6)
    XCTAssertThrowsError(try decode(wire129)) { err in
      XCTAssertEqual((err as? CborError)?.id, .tooDeep)
    }

    // Truncated array wire (128 wrappers with 0 remaining bytes) rejects with lengthTooLarge before tooDeep
    let truncated128 = [UInt8](repeating: 0x81, count: 128)
    XCTAssertThrowsError(try decode(truncated128)) { err in
      XCTAssertEqual((err as? CborError)?.id, .lengthTooLarge)
    }

    // Tag nesting
    var tag128 = CborValue.unsigned(0)
    for _ in 0..<128 {
      tag128 = .tag(1, tag128)
    }
    let encTag128 = try encodeChecked(tag128)
    let decTag128 = try decode(encTag128)
    XCTAssertEqual(decTag128, tag128)

    let tag129 = CborValue.tag(1, tag128)
    XCTAssertThrowsError(try encodeChecked(tag129)) { err in
      XCTAssertEqual((err as? CborError)?.id, .encodeTooDeep)
    }

    // Map nesting
    var map128 = CborValue.unsigned(0)
    for _ in 0..<128 {
      map128 = .map([CborMapEntry(key: .unsigned(1), value: map128)])
    }
    let encMap128 = try encodeChecked(map128)
    let decMap128 = try decode(encMap128)
    XCTAssertEqual(decMap128, map128)

    let map129 = CborValue.map([CborMapEntry(key: .unsigned(1), value: map128)])
    XCTAssertThrowsError(try encodeChecked(map129)) { err in
      XCTAssertEqual((err as? CborError)?.id, .encodeTooDeep)
    }
  }

  // ========================================================================
  // CVE-3: Strict UTF-8 Validation
  // ========================================================================

  func testCVE3StrictUtf8Validation() {
    let invalidUtf8Cases: [[UInt8]] = [
      [0x61, 0xFF],  // Invalid byte 0xFF
      [0x61, 0x80],  // Dangling continuation byte
      [0x62, 0xC0, 0xAF],  // Overlong 2-byte sequence
      [0x62, 0xC0, 0x80],  // Overlong NUL
      [0x63, 0xE0, 0x80, 0xAF],  // Overlong 3-byte sequence
      [0x64, 0xF0, 0x80, 0x80, 0xAF],  // Overlong 4-byte sequence
      [0x63, 0xED, 0xA0, 0x80],  // Surrogate U+D800
      [0x63, 0xED, 0xBF, 0xBF],  // Surrogate U+DFFF
    ]

    for wire in invalidUtf8Cases {
      XCTAssertThrowsError(try decode(wire)) { err in
        XCTAssertEqual((err as? CborError)?.id, .invalidUtf8)
      }
    }
  }

  // ========================================================================
  // CVE-4: Float Rejection Before Minimal Width
  // ========================================================================

  func testCVE4FloatRejection() {
    let floatCases: [[UInt8]] = [
      [0xF9, 0x00, 0x00],  // Float 16
      [0xFA, 0x00, 0x00, 0x00, 0x00],  // Float 32
      [0xFB, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],  // Float 64
    ]
    for wire in floatCases {
      XCTAssertThrowsError(try decode(wire)) { err in
        XCTAssertEqual((err as? CborError)?.id, .floatNotSupported)
      }
    }

    // Simple value tests
    XCTAssertThrowsError(try decode([0xF8, 0x17])) { err in
      XCTAssertEqual((err as? CborError)?.id, .nonMinimalInteger)
    }
    XCTAssertThrowsError(try decode([0xF8, 0x20])) { err in
      XCTAssertEqual((err as? CborError)?.id, .unsupportedSimple)
    }
    XCTAssertThrowsError(try decode([0xFC])) { err in
      XCTAssertEqual((err as? CborError)?.id, .reserved)
    }
    XCTAssertThrowsError(try decode([0xFF])) { err in
      XCTAssertEqual((err as? CborError)?.id, .indefinite)
    }
  }

  // ========================================================================
  // CVE-5: Mirror & Debug Redaction (Standing Rule #20)
  // ========================================================================

  func testCVE5MirrorAndStringRedaction() {
    guard let span = Span(start: 10, finish: 25) else {
      XCTFail("Failed to create span")
      return
    }
    let entry = SpannedMapEntry(
      key: .text("secret_api_key"),
      value: .text("super_confidential_token"),
      valueSpan: span
    )

    let desc = entry.description
    XCTAssertTrue(desc.contains("<redacted>"))
    XCTAssertFalse(desc.contains("secret_api_key"))
    XCTAssertFalse(desc.contains("super_confidential_token"))

    let debugDesc = entry.debugDescription
    XCTAssertTrue(debugDesc.contains("<redacted>"))
    XCTAssertFalse(debugDesc.contains("secret_api_key"))
    XCTAssertFalse(debugDesc.contains("super_confidential_token"))

    let mirror = Mirror(reflecting: entry)
    for child in mirror.children {
      if child.label == "key" || child.label == "value" {
        XCTAssertEqual(child.value as? String, "<redacted>")
      }
    }
  }

  // ========================================================================
  // CVE-6: Non-Minimal Integer Encodings Across All Majors
  // ========================================================================

  func testCVE6NonMinimalIntegers() {
    let nonMinimalCases: [[UInt8]] = [
      [0x18, 0x17],  // major 0 AI 24 val 23
      [0x38, 0x17],  // major 1 AI 24 val 23
      [0x58, 0x17],  // major 2 AI 24 val 23
      [0x78, 0x17],  // major 3 AI 24 val 23
      [0x98, 0x17],  // major 4 AI 24 val 23
      [0xB8, 0x17],  // major 5 AI 24 val 23
      [0xD8, 0x17],  // major 6 AI 24 val 23
      [0x19, 0x00, 0xFF],  // 16-bit encoding of 255
      [0x1A, 0x00, 0x00, 0xFF, 0xFF],  // 32-bit encoding of 65535
      [0x1B, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF],  // 64-bit encoding of 2^32 - 1
    ]

    for wire in nonMinimalCases {
      XCTAssertThrowsError(try decode(wire)) { err in
        XCTAssertEqual((err as? CborError)?.id, .nonMinimalInteger)
      }
    }
  }

  // ========================================================================
  // CVE-7: Memory Bomb & Atomic Buffer Rollback
  // ========================================================================

  func testCVE7MemoryLimitsAndRollback() throws {
    var buffer: [UInt8] = [0x01, 0x02, 0x03]
    let badVal = CborValue.map([
      CborMapEntry(key: .text("same"), value: .unsigned(1)),
      CborMapEntry(key: .text("same"), value: .unsigned(2)),
    ])

    XCTAssertThrowsError(try encodeIntoChecked(badVal, buffer: &buffer)) { err in
      XCTAssertEqual((err as? CborError)?.id, .duplicateMapKey)
    }
    XCTAssertEqual(buffer, [0x01, 0x02, 0x03], "Buffer was not cleanly rolled back on error")

    // Exceeding max encoded size limit
    let hugeBytes = [UInt8](repeating: 0, count: 1_048_572)
    XCTAssertThrowsError(try encodeChecked(.bytes(hugeBytes))) { err in
      XCTAssertEqual((err as? CborError)?.id, .encodeTooLarge)
    }

    // Appending a max-size item onto a non-empty destination buffer succeeds (item budget is independent of prefix)
    var prefixBuffer: [UInt8] = [0xAA, 0xBB, 0xCC]
    let maxItem = CborValue.bytes([UInt8](repeating: 0, count: 1_048_571))
    try encodeIntoChecked(maxItem, buffer: &prefixBuffer)
    XCTAssertEqual(prefixBuffer.count, 3 + 1_048_576)

    // Multi-key shared budget exhaustion in maps
    let key1 = CborValue.bytes([UInt8](repeating: 1, count: 600_000))
    let key2 = CborValue.bytes([UInt8](repeating: 2, count: 600_000))
    let multiKeyMap = CborValue.map([
      CborMapEntry(key: key1, value: .unsigned(1)),
      CborMapEntry(key: key2, value: .unsigned(2)),
    ])
    XCTAssertThrowsError(try encodeChecked(multiKeyMap)) { err in
      XCTAssertEqual((err as? CborError)?.id, .encodeTooLarge)
    }
  }

  // ========================================================================
  // Map Spans Decoding Tests
  // ========================================================================

  func testDecodeMapSpans() throws {
    // Wire: a2 01 02 03 04 -> map { 1: 2, 3: 4 }
    let wire: [UInt8] = [0xA2, 0x01, 0x02, 0x03, 0x04]
    let result = try decodeMapSpans(wire)
    guard case .map(let entries) = result else {
      XCTFail("Expected .map")
      return
    }
    XCTAssertEqual(entries.count, 2)
    XCTAssertEqual(entries[0].key, .unsigned(1))
    XCTAssertEqual(entries[0].value, .unsigned(2))
    XCTAssertEqual(entries[0].valueSpan.start, 2)
    XCTAssertEqual(entries[0].valueSpan.finish, 3)

    XCTAssertEqual(entries[1].key, .unsigned(3))
    XCTAssertEqual(entries[1].value, .unsigned(4))
    XCTAssertEqual(entries[1].valueSpan.start, 4)
    XCTAssertEqual(entries[1].valueSpan.finish, 5)

    // Non-map wire returns .nonMap
    let nonMapResult = try decodeMapSpans([0x81, 0x01])
    XCTAssertEqual(nonMapResult, .nonMap)

    // Trailing bytes on map throws trailingBytes
    XCTAssertThrowsError(try decodeMapSpans([0xA0, 0x00])) { err in
      XCTAssertEqual((err as? CborError)?.id, .trailingBytes)
    }

    // Empty bytes throws unexpectedEof
    XCTAssertThrowsError(try decodeMapSpans([])) { err in
      XCTAssertEqual((err as? CborError)?.id, .unexpectedEof)
    }

    // Hostile map length in decodeMapSpans
    XCTAssertThrowsError(
      try decodeMapSpans([0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
    ) { err in
      XCTAssertEqual((err as? CborError)?.id, .lengthTooLarge)
    }

    // Out of order map keys in decodeMapSpans
    XCTAssertThrowsError(try decodeMapSpans([0xA2, 0x02, 0x00, 0x01, 0x00])) { err in
      XCTAssertEqual((err as? CborError)?.id, .nonCanonicalMapOrder)
    }

    // Duplicate key in decodeMapSpans
    XCTAssertThrowsError(try decodeMapSpans([0xA2, 0x01, 0x00, 0x01, 0x00])) { err in
      XCTAssertEqual((err as? CborError)?.id, .nonCanonicalMapOrder)
    }

    // Truncated map entry where remaining bytes < minBytesPerMapEntry
    XCTAssertThrowsError(try decodeMapSpans([0xA1])) { err in
      XCTAssertEqual((err as? CborError)?.id, .lengthTooLarge)
    }

    // Truncated key argument inside map
    XCTAssertThrowsError(try decodeMapSpans([0xA1, 0x1A, 0x00])) { err in
      XCTAssertEqual((err as? CborError)?.id, .unexpectedEof)
    }
  }

  // ========================================================================
  // Deterministic Property Tests & Map Permutation Invariance
  // ========================================================================

  private struct LCG {
    var state: UInt64
    init(seed: UInt64) { self.state = seed }
    mutating func next() -> UInt64 {
      state = state &* 6_364_136_223_846_793_005 &+ 1_442_695_040_888_963_407
      return state
    }
    mutating func nextInt(bound: Int) -> Int {
      Int(next() % UInt64(bound))
    }
  }

  private func randomCborValue(rng: inout LCG, depth: Int = 0) -> CborValue {
    if depth >= 4 {
      let choice = rng.nextInt(bound: 4)
      switch choice {
      case 0: return .unsigned(rng.next() % 1000)
      case 1: return .negative(rng.next() % 1000)
      case 2: return .boolean(rng.next() % 2 == 0)
      default: return .null
      }
    }

    let choice = rng.nextInt(bound: 9)
    switch choice {
    case 0:
      return .unsigned(rng.next() % 100_000)
    case 1:
      return .negative(rng.next() % 100_000)
    case 2:
      let len = rng.nextInt(bound: 16)
      var bytes: [UInt8] = []
      for _ in 0..<len { bytes.append(UInt8(rng.next() % 256)) }
      return .bytes(bytes)
    case 3:
      let len = rng.nextInt(bound: 8)
      var str = ""
      for _ in 0..<len {
        let code = 0x61 + (rng.next() % 26)
        if let scalar = UnicodeScalar(UInt32(code)) {
          str.unicodeScalars.append(scalar)
        }
      }
      return .text(str)
    case 4:
      let count = rng.nextInt(bound: 4)
      var arr: [CborValue] = []
      for _ in 0..<count {
        arr.append(randomCborValue(rng: &rng, depth: depth + 1))
      }
      return .array(arr)
    case 5:
      let count = rng.nextInt(bound: 3)
      var entries: [CborMapEntry] = []
      for i in 0..<count {
        let k = CborValue.unsigned(UInt64(i * 10) + (rng.next() % 5))
        let v = randomCborValue(rng: &rng, depth: depth + 1)
        entries.append(CborMapEntry(key: k, value: v))
      }
      return .map(entries)
    case 6:
      return .tag(rng.next() % 50, randomCborValue(rng: &rng, depth: depth + 1))
    case 7:
      return .boolean(rng.next() % 2 == 0)
    default:
      return .null
    }
  }

  func testPropertyRoundtripsAndMapPermutations() throws {
    var rng = LCG(seed: 0xCAFE_BABE_DEAD_BEEF)

    for _ in 0..<500 {
      let val = randomCborValue(rng: &rng)
      do {
        let wire = try encodeChecked(val)
        let decoded = try decode(wire)
        XCTAssertEqual(decoded, val)
        let reEncoded = try encodeChecked(decoded)
        XCTAssertEqual(reEncoded, wire)
      } catch let err as CborError where err.id == .duplicateMapKey {
        // If random map generation happened to produce duplicate key, that is validly rejected
        continue
      }
    }

    // Map Permutation Invariance: shuffling entries produces identical wire bytes
    let mapEntries: [CborMapEntry] = [
      CborMapEntry(key: .unsigned(50), value: .text("fifty")),
      CborMapEntry(key: .text("alpha"), value: .unsigned(1)),
      CborMapEntry(key: .unsigned(2), value: .boolean(true)),
      CborMapEntry(key: .bytes([0xFF]), value: .null),
      CborMapEntry(key: .negative(10), value: .unsigned(100)),
    ]
    let originalMap = CborValue.map(mapEntries)
    let originalWire = try encodeChecked(originalMap)

    for seed in [1, 42, 99, 1337, 8888] {
      var permRng = LCG(seed: UInt64(seed))
      var shuffled = mapEntries
      for i in 0..<shuffled.count {
        let j = permRng.nextInt(bound: shuffled.count)
        shuffled.swapAt(i, j)
      }
      let permutedWire = try encodeChecked(CborValue.map(shuffled))
      XCTAssertEqual(
        permutedWire,
        originalWire,
        "Map permutation altered canonical wire encoding"
      )
    }
  }

  // ========================================================================
  // Adversarial Mutation Fuzz Loop (10,000 iterations)
  // ========================================================================

  func testAdversarialMutationFuzzing() throws {
    let seeds: [CborValue] = [
      .unsigned(0),
      .unsigned(23),
      .unsigned(24),
      .unsigned(65536),
      .negative(0),
      .bytes([0x01, 0x02, 0x03]),
      .text("canonical"),
      .array([.unsigned(1), .text("two"), .boolean(true)]),
      .map([
        CborMapEntry(key: .unsigned(1), value: .text("one")),
        CborMapEntry(key: .text("k"), value: .unsigned(99)),
      ]),
      .tag(10, .text("tagged")),
      .boolean(false),
      .null,
    ]

    var wireTemplates: [[UInt8]] = []
    for s in seeds {
      wireTemplates.append(try encodeChecked(s))
    }

    var rng = LCG(seed: 0x1234_5678_9ABC_DEF0)

    for _ in 0..<10_000 {
      let templateIndex = rng.nextInt(bound: wireTemplates.count)
      var mutated = wireTemplates[templateIndex]

      let mutationType = rng.nextInt(bound: 5)
      switch mutationType {
      case 0:
        // Bit flip
        if !mutated.isEmpty {
          let byteIdx = rng.nextInt(bound: mutated.count)
          let bitIdx = rng.nextInt(bound: 8)
          mutated[byteIdx] ^= (1 << bitIdx)
        }
      case 1:
        // Byte replace
        if !mutated.isEmpty {
          let byteIdx = rng.nextInt(bound: mutated.count)
          mutated[byteIdx] = UInt8(rng.next() % 256)
        }
      case 2:
        // Truncation
        if !mutated.isEmpty {
          let dropCount = 1 + rng.nextInt(bound: mutated.count)
          mutated.removeLast(dropCount)
        }
      case 3:
        // Byte insertion
        let insertIdx = rng.nextInt(bound: mutated.count + 1)
        mutated.insert(UInt8(rng.next() % 256), at: insertIdx)
      default:
        // Multi-byte corruption
        if mutated.count >= 2 {
          let start = rng.nextInt(bound: mutated.count - 1)
          mutated[start] = UInt8(rng.next() % 256)
          mutated[start + 1] = UInt8(rng.next() % 256)
        }
      }

      // Unconditionally decode: MUST NOT CRASH
      do {
        let decoded = try decode(mutated)
        // If it decoded successfully, it MUST be valid canonical CBOR and roundtrip byte-for-byte!
        let reEncoded = try encodeChecked(decoded)
        XCTAssertEqual(reEncoded, mutated, "Mutated wire accepted non-canonical encoding")
        let reDecoded = try decode(reEncoded)
        XCTAssertEqual(reDecoded, decoded)
      } catch is CborError {
        // Expected canonical rejection
      } catch {
        XCTFail("Unexpected exception thrown: \(error)")
      }
    }
  }
}
