import XCTest

import HashMap
@testable import HashSet

final class HashSetTests: XCTestCase {
  private let leftKeys: [HashMapKey] = [
    .string("alpha"), .string("beta"), .string("gamma"),
  ]

  func testConstructionDiscardsDuplicatesAndPreservesExactIdentity() throws {
    let composed = HashMapKey.string("\u{00E9}")
    let decomposed = HashMapKey.string("e\u{0301}")
    let set = try HashSet.fromElements([
      .string("alpha"), .string("alpha"), composed, decomposed,
      .bytes(Array("alpha".utf8)),
    ])

    XCTAssertEqual(set.count, 4)
    XCTAssertTrue(set.contains(.string("alpha")))
    XCTAssertTrue(set.contains(.bytes(Array("alpha".utf8))))
    XCTAssertTrue(set.contains(composed))
    XCTAssertTrue(set.contains(decomposed))
  }

  func testAddingRemovingAndDiscardingArePersistent() throws {
    let empty = try HashSet(
      capacity: 4,
      strategy: .linearProbing,
      hashAlgorithm: .fnv1a32
    )
    let one = try empty.adding(.string("alpha"))
    let duplicate = try one.adding(.string("alpha"))
    let two = try duplicate.adding(.string("beta"))
    let removed = two.removing(.string("alpha"))
    let discardedMissing = removed.discarding(.string("missing"))

    XCTAssertTrue(empty.isEmpty)
    XCTAssertFalse(empty.contains(.string("alpha")))
    XCTAssertEqual(one.count, 1)
    XCTAssertEqual(duplicate.count, 1)
    XCTAssertEqual(two.count, 2)
    XCTAssertTrue(two.contains(.string("alpha")))
    XCTAssertFalse(removed.contains(.string("alpha")))
    XCTAssertEqual(discardedMissing.count, 1)
  }

  func testElementsUseUnderlyingDeterministicStorageOrder() throws {
    let set = try HashSet.fromElements(
      [.string("b"), .string("d"), .string("a"), .string("c")],
      capacity: 4,
      strategy: .separateChaining,
      hashAlgorithm: .fnv1a32
    )

    XCTAssertEqual(
      set.elements,
      [.string("a"), .string("b"), .string("c"), .string("d")]
    )
    XCTAssertEqual(set.configuration.strategy, .separateChaining)
    XCTAssertEqual(set.configuration.hashAlgorithm, .fnv1a32)
  }

  func testUnionIsPersistentAndKeepsReceiverProfile() throws {
    let left = try HashSet.fromElements(
      leftKeys,
      capacity: 4,
      strategy: .linearProbing,
      hashAlgorithm: .murmur3(seed: 7)
    )
    let right = try HashSet.fromElements([
      .string("beta"), .string("delta"),
    ])
    let union = try left.union(right)

    XCTAssertEqual(union.count, 4)
    for key in leftKeys + [.string("delta")] {
      XCTAssertTrue(union.contains(key))
    }
    XCTAssertEqual(left.count, 3)
    XCTAssertEqual(right.count, 2)
    XCTAssertEqual(union.configuration.strategy, .linearProbing)
    XCTAssertEqual(union.configuration.hashAlgorithm, .murmur3(seed: 7))
  }

  func testIntersectionDifferenceAndSymmetricDifference() throws {
    let left = try HashSet.fromElements(leftKeys)
    let right = try HashSet.fromElements([
      .string("beta"), .string("delta"),
    ])

    let intersection = try left.intersection(right)
    XCTAssertEqual(intersection.count, 1)
    XCTAssertTrue(intersection.contains(.string("beta")))

    let difference = try left.difference(right)
    XCTAssertEqual(difference.count, 2)
    XCTAssertTrue(difference.contains(.string("alpha")))
    XCTAssertTrue(difference.contains(.string("gamma")))

    let symmetric = try left.symmetricDifference(right)
    XCTAssertEqual(symmetric.count, 3)
    XCTAssertTrue(symmetric.contains(.string("alpha")))
    XCTAssertTrue(symmetric.contains(.string("gamma")))
    XCTAssertTrue(symmetric.contains(.string("delta")))
    XCTAssertFalse(symmetric.contains(.string("beta")))
  }

  func testRelationsIncludeEmptyAndReflexiveCases() throws {
    let empty = try HashSet()
    let left = try HashSet.fromElements([
      .string("alpha"), .string("beta"),
    ])
    let sameDifferentOrder = try HashSet.fromElements([
      .string("beta"), .string("alpha"),
    ])
    let larger = try left.adding(.string("gamma"))
    let disjoint = try HashSet.fromElements([.string("delta")])

    XCTAssertTrue(empty.isSubset(of: left))
    XCTAssertTrue(empty.isSubset(of: empty))
    XCTAssertTrue(left.isSubset(of: larger))
    XCTAssertFalse(larger.isSubset(of: left))
    XCTAssertTrue(larger.isSuperset(of: left))
    XCTAssertTrue(left.isDisjoint(with: disjoint))
    XCTAssertFalse(left.isDisjoint(with: larger))
    XCTAssertTrue(left.equals(sameDifferentOrder))
    XCTAssertFalse(left.equals(larger))
  }

  func testEmptySetAlgebraLaws() throws {
    let empty = try HashSet()
    let populated = try HashSet.fromElements(leftKeys)

    XCTAssertTrue(try empty.union(populated).equals(populated))
    XCTAssertTrue(try populated.union(empty).equals(populated))
    XCTAssertTrue(try empty.intersection(populated).isEmpty)
    XCTAssertTrue(try empty.difference(populated).isEmpty)
    XCTAssertTrue(try populated.difference(empty).equals(populated))
    XCTAssertTrue(try empty.symmetricDifference(populated).equals(populated))
  }

  func testBothCollisionStrategiesSurviveRemovalAndAlgebra() throws {
    for strategy in [CollisionStrategy.separateChaining, .linearProbing] {
      let set = try HashSet.fromElements(
        [.string("k4"), .string("k14"), .string("k21")],
        capacity: 8,
        strategy: strategy,
        hashAlgorithm: .fnv1a32
      )
      let removed = set.removing(.string("k4"))
      let restored = try removed.union(
        HashSet.fromElements([.string("k4")])
      )

      XCTAssertTrue(removed.contains(.string("k14")))
      XCTAssertTrue(removed.contains(.string("k21")))
      XCTAssertFalse(removed.contains(.string("k4")))
      XCTAssertTrue(restored.equals(set))
    }
  }

  func testInvalidConstructionAndOversizedElementsUseMapErrors() throws {
    XCTAssertThrowsError(try HashSet(capacity: 0)) {
      XCTAssertEqual($0 as? HashMapError, .invalidCapacity(0))
    }
    XCTAssertThrowsError(
      try HashSet(hashAlgorithm: .sipHash24(key: [0]))
    ) {
      XCTAssertEqual($0 as? HashMapError, .invalidSipHashKeyLength(1))
    }

    let set = try HashSet()
    let oversized = HashMapKey.bytes(
      Array(repeating: 0, count: HashMapLimits.maximumKeyByteCount + 1)
    )
    XCTAssertThrowsError(try set.adding(oversized)) {
      XCTAssertEqual(
        $0 as? HashMapError,
        .keyTooLong(
          actual: HashMapLimits.maximumKeyByteCount + 1,
          maximum: HashMapLimits.maximumKeyByteCount
        )
      )
    }
    XCTAssertFalse(set.contains(oversized))
    XCTAssertEqual(set.removing(oversized).count, 0)
  }
}
