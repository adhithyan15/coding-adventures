import XCTest

@testable import HashMap

final class HashMapTests: XCTestCase {
  func testKeyEncodingIsCanonicalAndTypeTagged() {
    XCTAssertEqual(
      HashMapKey.string("a").encodedBytes,
      [0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x61]
    )
    XCTAssertEqual(
      HashMapKey.bytes([0x61]).encodedBytes,
      [0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x61]
    )
    XCTAssertNotEqual(HashMapKey.string("a"), HashMapKey.bytes([0x61]))
  }

  func testStringKeyEqualityUsesExactUTF8Bytes() {
    let composed = "\u{00E9}"
    let decomposed = "e\u{0301}"

    XCTAssertEqual(composed, decomposed)
    XCTAssertNotEqual(HashMapKey.string(composed), HashMapKey.string(decomposed))
    XCTAssertNotEqual(
      HashMapKey.string(composed).encodedBytes,
      HashMapKey.string(decomposed).encodedBytes
    )
  }

  func testCanonicallyEquivalentStringsCoexistAcrossStrategies() throws {
    let composed = HashMapKey.string("\u{00E9}")
    let decomposed = HashMapKey.string("e\u{0301}")

    for strategy in [CollisionStrategy.separateChaining, .linearProbing] {
      let empty = try HashMap<Int>(
        capacity: 8,
        strategy: strategy,
        hashAlgorithm: .fnv1a32
      )
      let composedOnly = try empty.setting(1, for: composed)
      XCTAssertEqual(composedOnly.value(forKey: composed), 1)
      XCTAssertNil(composedOnly.value(forKey: decomposed))

      let both = try composedOnly.setting(2, for: decomposed)
      XCTAssertEqual(both.count, 2)
      XCTAssertEqual(both.value(forKey: composed), 1)
      XCTAssertEqual(both.value(forKey: decomposed), 2)

      let overwritten = try both.setting(3, for: composed)
      XCTAssertEqual(overwritten.count, 2)
      XCTAssertEqual(overwritten.value(forKey: composed), 3)
      XCTAssertEqual(overwritten.value(forKey: decomposed), 2)

      let removed = overwritten.removingValue(for: decomposed)
      XCTAssertEqual(removed.count, 1)
      XCTAssertEqual(removed.value(forKey: composed), 3)
      XCTAssertNil(removed.value(forKey: decomposed))
    }
  }

  func testChainingCRUDOverwriteAndPersistence() throws {
    let empty = try HashMap<Int>(
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    )
    let one = try empty.setting(1, for: .string("a"))
    let two = try one.setting(2, for: .string("b"))
    let overwritten = try two.setting(20, for: .string("b"))
    let removed = overwritten.removingValue(for: .string("a"))

    XCTAssertEqual(empty.count, 0)
    XCTAssertFalse(empty.has(.string("a")))
    XCTAssertEqual(one.value(forKey: .string("a")), 1)
    XCTAssertNil(one.value(forKey: .string("b")))
    XCTAssertEqual(two.count, 2)
    XCTAssertEqual(two.value(forKey: .string("b")), 2)
    XCTAssertEqual(overwritten.count, 2)
    XCTAssertEqual(overwritten.value(forKey: .string("b")), 20)
    XCTAssertFalse(removed.has(.string("a")))
    XCTAssertEqual(removed.count, 1)
    XCTAssertEqual(overwritten.value(forKey: .string("a")), 1)
  }

  func testOpenAddressingCRUDAndPersistence() throws {
    let empty = try HashMap<String>(
      capacity: 8,
      strategy: .linearProbing,
      hashAlgorithm: .fnv1a32
    )
    let one = try empty.setting("left", for: .string("k4"))
    let two = try one.setting("right", for: .string("k14"))
    let removed = two.removingValue(for: .string("k4"))

    XCTAssertEqual(two.value(forKey: .string("k4")), "left")
    XCTAssertEqual(two.value(forKey: .string("k14")), "right")
    XCTAssertFalse(removed.has(.string("k4")))
    XCTAssertEqual(removed.value(forKey: .string("k14")), "right")
    XCTAssertEqual(one.count, 1)
    XCTAssertEqual(two.count, 2)
  }

  func testChainingResizesOnlyAboveLoadOne() throws {
    var map = try HashMap<Int>(
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    )
    for (index, key) in ["a", "b", "c", "d"].enumerated() {
      map = try map.setting(index, for: .string(key))
    }
    XCTAssertEqual(map.capacity, 4)
    XCTAssertEqual(map.loadFactor, 1.0)

    map = try map.setting(4, for: .string("e"))
    XCTAssertEqual(map.capacity, 8)
    XCTAssertEqual(map.count, 5)
    XCTAssertEqual(map.value(forKey: .string("e")), 4)
  }

  func testOpenAddressingResizesAboveThreeQuarters() throws {
    var map = try HashMap<Int>(
      capacity: 4,
      strategy: .linearProbing,
      hashAlgorithm: .fnv1a32
    )
    for (index, key) in ["a", "b", "c"].enumerated() {
      map = try map.setting(index, for: .string(key))
    }
    XCTAssertEqual(map.capacity, 4)
    XCTAssertEqual(map.loadFactor, 0.75)

    map = try map.setting(3, for: .string("d"))
    XCTAssertEqual(map.capacity, 8)
    XCTAssertEqual(map.count, 4)
    XCTAssertEqual(map.value(forKey: .string("a")), 0)
    XCTAssertEqual(map.value(forKey: .string("d")), 3)
  }

  func testTombstonePreservesProbeChainAndIsReused() throws {
    var map = try HashMap<Int>(
      capacity: 8,
      strategy: .linearProbing,
      hashAlgorithm: .fnv1a32
    )
    map = try map.setting(4, for: .string("k4"))
    map = try map.setting(14, for: .string("k14"))
    map = try map.setting(21, for: .string("k21"))
    map = map.removingValue(for: .string("k4"))

    XCTAssertEqual(map.value(forKey: .string("k14")), 14)
    XCTAssertEqual(map.value(forKey: .string("k21")), 21)

    map = try map.setting(140, for: .string("k14"))
    XCTAssertEqual(map.count, 2)
    XCTAssertEqual(map.value(forKey: .string("k14")), 140)

    map = try map.setting(29, for: .string("k29"))
    XCTAssertEqual(map.count, 3)
    XCTAssertEqual(map.value(forKey: .string("k29")), 29)
    XCTAssertEqual(
      map.entries.map(\.key),
      [
        .string("k29"), .string("k14"), .string("k21"),
      ])
  }

  func testStoredOptionalNilIsDistinctFromMissing() throws {
    let map = try HashMap<Int?>(
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    ).setting(nil, for: .string("nil"))

    XCTAssertTrue(map.has(.string("nil")))
    switch map.value(forKey: .string("nil")) {
    case .some(let stored): XCTAssertNil(stored)
    case .none: XCTFail("stored Optional.none was reported as a missing key")
    }
    XCTAssertFalse(map.has(.string("missing")))
    if case .some = map.value(forKey: .string("missing")) {
      XCTFail("missing key unexpectedly produced an outer Optional.some")
    }
  }

  func testStorageOrderIsDeterministicForFixedProfile() throws {
    var chaining = try HashMap<Int>(
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    )
    var probing = try HashMap<Int>(
      capacity: 4,
      strategy: .linearProbing,
      hashAlgorithm: .fnv1a32
    )
    for (index, key) in ["b", "d", "a", "c"].enumerated() {
      chaining = try chaining.setting(index, for: .string(key))
      probing = try probing.setting(index, for: .string(key))
    }

    XCTAssertEqual(
      chaining.keys,
      [
        .string("a"), .string("b"), .string("c"), .string("d"),
      ])
    // The fourth insertion grows the open-addressed table to capacity 8.
    // Its deterministic order therefore differs from the chaining profile.
    XCTAssertEqual(
      probing.keys,
      [
        .string("c"), .string("d"), .string("a"), .string("b"),
      ])
    XCTAssertEqual(chaining.entries.map(\.value), [2, 0, 3, 1])
    XCTAssertEqual(probing.values, [3, 1, 2, 0])
  }

  func testFromEntriesAndMergeArePersistentAndRightBiased() throws {
    let left = try HashMap<Int>.fromEntries(
      [(.string("a"), 1), (.string("b"), 2), (.string("b"), 20)],
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    )
    let right = try HashMap<Int>.fromEntries(
      [(.string("b"), 99), (.string("c"), 3)],
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    )
    let merged = try left.merging(right)

    XCTAssertEqual(left.count, 2)
    XCTAssertEqual(left.value(forKey: .string("b")), 20)
    XCTAssertEqual(right.count, 2)
    XCTAssertEqual(merged.count, 3)
    XCTAssertEqual(merged.value(forKey: .string("a")), 1)
    XCTAssertEqual(merged.value(forKey: .string("b")), 99)
    XCTAssertEqual(merged.value(forKey: .string("c")), 3)
  }

  func testClosedHashAlgorithmsReachExpectedLiteralBuckets() throws {
    let key = HashMapKey.string("a")
    XCTAssertEqual(try HashAlgorithm.fnv1a32.validatedHash(key.encodedBytes), 0x280C_D7A4)
    XCTAssertEqual(try HashAlgorithm.murmur3(seed: 0).validatedHash([]), 0)
    XCTAssertEqual(HashAlgorithm.fnv1a32.hash(key.encodedBytes), 0x280C_D7A4)
    XCTAssertEqual(HashAlgorithm.murmur3(seed: 0).hash([]), 0)
    XCTAssertEqual(
      try HashAlgorithm.sipHash24(key: Array(0...15)).validatedHash([]),
      0x726F_DB47_DD0E_0E31
    )
    XCTAssertThrowsError(
      try HashAlgorithm.sipHash24(key: [0]).validatedHash([])
    ) { XCTAssertEqual($0 as? HashMapError, .invalidSipHashKeyLength(1)) }

    let defaultMap = try HashMap<Int>().setting(1, for: key)
    XCTAssertEqual(defaultMap.value(forKey: key), 1)
  }

  func testInvalidConstructionAndSipHashKeyAreStableErrors() {
    XCTAssertThrowsError(
      try HashMap<Int>(
        capacity: 0,
        strategy: .separateChaining,
        hashAlgorithm: .fnv1a32
      )
    ) { XCTAssertEqual($0 as? HashMapError, .invalidCapacity(0)) }
    XCTAssertThrowsError(
      try HashMap<Int>(
        capacity: HashMapLimits.maximumCapacity + 1,
        strategy: .separateChaining,
        hashAlgorithm: .fnv1a32
      )
    ) { XCTAssertEqual($0 as? HashMapError, .capacityLimitExceeded) }
    XCTAssertThrowsError(
      try HashMap<Int>(
        capacity: 4,
        strategy: .linearProbing,
        hashAlgorithm: .sipHash24(key: [0])
      )
    ) { XCTAssertEqual($0 as? HashMapError, .invalidSipHashKeyLength(1)) }
  }

  func testCheckedGrowthAndKeyBudgetFailWithoutChangingOldMap() throws {
    var map = try HashMap<Int>(
      capacity: 2,
      strategy: .linearProbing,
      hashAlgorithm: .fnv1a32,
      limits: .init(maximumCapacity: 2, maximumKeyByteCount: 2)
    )
    map = try map.setting(1, for: .string("a"))
    let beforeFailure = map

    XCTAssertThrowsError(try map.setting(2, for: .string("b"))) {
      XCTAssertEqual($0 as? HashMapError, .capacityLimitExceeded)
    }
    XCTAssertThrowsError(try map.setting(3, for: .string("abc"))) {
      XCTAssertEqual($0 as? HashMapError, .keyTooLong(actual: 3, maximum: 2))
    }
    XCTAssertEqual(beforeFailure.count, 1)
    XCTAssertEqual(beforeFailure.value(forKey: .string("a")), 1)

    let oversized = HashMapKey.string("abc")
    XCTAssertNil(map.value(forKey: oversized))
    XCTAssertFalse(map.has(oversized))
    XCTAssertEqual(map.removingValue(for: oversized).count, 1)
  }

  func testExportsDoNotAliasStructuralStorageButReferenceValuesRemainShallow() throws {
    final class Box {
      var value: Int
      init(_ value: Int) { self.value = value }
    }
    let box = Box(1)
    let map = try HashMap<Box>(
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    ).setting(box, for: .bytes([1, 2]))

    var exportedKeys = map.keys
    exportedKeys[0] = .bytes([9])
    XCTAssertTrue(map.has(.bytes([1, 2])))
    XCTAssertFalse(map.has(.bytes([9])))

    let exportedBox = try XCTUnwrap(map.value(forKey: .bytes([1, 2])))
    XCTAssertTrue(exportedBox === box)
    exportedBox.value = 7
    XCTAssertEqual(map.value(forKey: .bytes([1, 2]))?.value, 7)
  }

  func testConfigurationCanSeedDownstreamHashSetWrapper() throws {
    struct SetWrapper {
      private var storage: HashMap<Bool>

      init(configuration: HashMapConfiguration) throws {
        storage = try HashMap(
          capacity: configuration.capacity,
          strategy: configuration.strategy,
          hashAlgorithm: configuration.hashAlgorithm
        )
      }

      func inserting(_ key: HashMapKey) throws -> Self {
        var copy = self
        copy.storage = try storage.setting(true, for: key)
        return copy
      }

      func contains(_ key: HashMapKey) -> Bool { storage.has(key) }
      var elements: [HashMapKey] { storage.keys }
    }

    let map = try HashMap<Int>(
      capacity: 8,
      strategy: .linearProbing,
      hashAlgorithm: .murmur3(seed: 7)
    )
    let set = try SetWrapper(configuration: map.configuration)
      .inserting(.string("member"))
    XCTAssertTrue(set.contains(.string("member")))
    XCTAssertEqual(set.elements, [.string("member")])
  }

  func testDeterministicOperationTraceMatchesSemanticReference() throws {
    for strategy in [CollisionStrategy.separateChaining, .linearProbing] {
      var map = try HashMap<Int>(
        capacity: 4,
        strategy: strategy,
        hashAlgorithm: .murmur3(seed: 0xC0DE)
      )
      var reference: [String: Int] = [:]

      for step in 0..<300 {
        let key = "key-\((step &* 37 &+ 11) % 41)"
        if step % 7 == 0 {
          map = map.removingValue(for: .string(key))
          reference.removeValue(forKey: key)
        } else {
          let value = step &* 13
          map = try map.setting(value, for: .string(key))
          reference[key] = value
        }

        XCTAssertEqual(map.count, reference.count)
        for probe in 0..<41 {
          let probeKey = "key-\(probe)"
          XCTAssertEqual(map.has(.string(probeKey)), reference[probeKey] != nil)
          XCTAssertEqual(map.value(forKey: .string(probeKey)), reference[probeKey])
        }
      }
    }
  }
}
