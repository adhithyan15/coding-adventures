import Testing
@testable import SkipList

private final class LevelScript {
  private var levels: [Int]
  private(set) var calls = 0

  init(_ levels: [Int]) {
    self.levels = levels
  }

  func next() -> Int {
    calls += 1
    return levels.isEmpty ? 1 : levels.removeFirst()
  }
}

@Test func configurationAndEmptyBehavior() throws {
  #expect(throws: SkipListError.invalidMaximumLevel(0)) {
    try SkipList<Int, String>(maximumLevel: 0, comparator: <, levelSource: { 1 })
  }
  #expect(throws: SkipListError.invalidMaximumLevel(33)) {
    try SkipList<Int, String>(maximumLevel: 33, comparator: <, levelSource: { 1 })
  }
  #expect(throws: SkipListError.invalidMaximumCount(0)) {
    try SkipList<Int, String>(maximumCount: 0, comparator: <, levelSource: { 1 })
  }

  let list = try SkipList<Int, String>(comparator: <, levelSource: { 1 })
  #expect(list.isEmpty)
  #expect(list.count == 0)
  #expect(list.currentMaximumLevel == 1)
  #expect(list.value(for: 1) == nil)
  #expect(!list.contains(1))
  #expect(list.rank(of: 1) == nil)
  #expect(list.entry(at: 0) == nil)
  #expect(list.entry(at: -1) == nil)
  #expect(list.minimum() == nil)
  #expect(list.maximum() == nil)
  #expect(try list.orderedEntries().isEmpty)
  #expect(list.validatesStructure())
}

@Test func exactTowerSpansRanksAndUpsert() throws {
  let levels = LevelScript([1, 3, 2])
  let list = try SkipList<Int, String>(maximumLevel: 4, comparator: <, levelSource: levels.next)
  #expect(try list.insert(10, value: "ten"))
  #expect(try list.insert(20, value: "twenty"))
  #expect(try list.insert(30, value: "thirty"))
  #expect(!(try list.insert(20, value: "TWENTY")))
  #expect(levels.calls == 3)
  #expect(list.count == 3)
  #expect(list.value(for: 20) == "TWENTY")
  #expect(list.rank(of: 10) == 0)
  #expect(list.rank(of: 20) == 1)
  #expect(list.rank(of: 30) == 2)
  #expect(list.rank(of: 99) == nil)
  #expect(list.entry(at: 1)?.key == 20)
  #expect(list.entry(at: 1)?.value == "TWENTY")
  #expect(list.entry(at: 3) == nil)

  let snapshot = list.debugLevelSnapshot()
  #expect(snapshot.count == 3)
  #expect(snapshot[0].map(\.destination) == [10, 20, 30])
  #expect(snapshot[0].map(\.span) == [1, 1, 1])
  #expect(snapshot[1].map(\.destination) == [20, 30])
  #expect(snapshot[1].map(\.span) == [2, 1])
  #expect(snapshot[2].map(\.destination) == [20])
  #expect(snapshot[2].map(\.span) == [2])
  #expect(list.validatesStructure())
}

@Test func invalidLevelAndCapacityFailuresAreAtomic() throws {
  let levels = LevelScript([2, 0, 1])
  let list = try SkipList<Int, String>(
    maximumLevel: 3,
    maximumCount: 2,
    comparator: <,
    levelSource: levels.next
  )
  #expect(try list.insert(1, value: "one"))
  #expect(throws: SkipListError.invalidGeneratedLevel(level: 0, maximum: 3)) {
    try list.insert(2, value: "two")
  }
  #expect(list.count == 1)
  #expect(!list.contains(2))
  #expect(try list.insert(2, value: "two"))
  #expect(throws: SkipListError.capacityExceeded(maximum: 2)) {
    try list.insert(3, value: "three")
  }
  #expect(levels.calls == 3)
  #expect(!(try list.insert(2, value: "TWO")))
  #expect(levels.calls == 3)
  #expect(list.value(for: 2) == "TWO")
  #expect(list.validatesStructure())
}

@Test func deletionRepairsSpansAndCollapsesTopLevels() throws {
  let levels = LevelScript([1, 4, 2, 1])
  let list = try SkipList<Int, Int>(maximumLevel: 4, comparator: <, levelSource: levels.next)
  for key in [10, 20, 30, 40] {
    try list.insert(key, value: key * 10)
  }
  #expect(list.currentMaximumLevel == 4)
  #expect(!list.remove(99))
  #expect(list.remove(20))
  #expect(list.currentMaximumLevel == 2)
  #expect(list.rank(of: 30) == 1)
  #expect(list.entry(at: 2)?.key == 40)
  #expect(list.minimum()?.key == 10)
  #expect(list.maximum()?.key == 40)
  #expect(list.validatesStructure())
  #expect(list.remove(40))
  #expect(list.maximum()?.key == 30)
  #expect(list.remove(10))
  #expect(list.remove(30))
  #expect(list.isEmpty)
  #expect(list.currentMaximumLevel == 1)
  #expect(list.validatesStructure())
}

@Test func rangesAreBoundedAndRespectEndpoints() throws {
  let list = try SkipList<Int, Int>(comparator: <, levelSource: { 1 })
  for key in [5, 12, 20, 37, 42, 50, 55, 63] {
    try list.insert(key, value: key * 10)
  }
  #expect(try list.entries(from: 20, through: 55).map(\.key) == [20, 37, 42, 50, 55])
  #expect(
    try list.entries(from: 20, through: 55, includingLower: false, includingUpper: false)
      .map(\.key) == [37, 42, 50]
  )
  #expect(try list.entries(from: 42, through: 42).map(\.key) == [42])
  #expect(try list.entries(from: 55, through: 20).isEmpty)
  #expect(try list.entries(from: 100, through: 200).isEmpty)
  #expect(throws: SkipListError.invalidOutputLimit(0)) {
    try list.orderedEntries(limit: 0)
  }
  #expect(throws: SkipListError.invalidOutputLimit(65_537)) {
    try list.entries(from: 0, through: 100, limit: 65_537)
  }
  #expect(throws: SkipListError.outputLimitExceeded(limit: 2)) {
    try list.entries(from: 5, through: 63, limit: 2)
  }
  #expect(throws: SkipListError.outputLimitExceeded(limit: 3)) {
    try list.orderedEntries(limit: 3)
  }
}

@Test func optionalValuesAndCustomComparatorsPreservePresence() throws {
  let descending = try SkipList<Int, String?>(comparator: >, levelSource: { 1 })
  try descending.insert(10, value: nil)
  try descending.insert(30, value: "thirty")
  try descending.insert(20, value: "twenty")
  #expect(descending.contains(10))
  #expect(descending.value(for: 10) != nil)
  #expect(descending.value(for: 10)! == nil)
  #expect(try descending.orderedEntries().map(\.key) == [30, 20, 10])
  #expect(try descending.entries(from: 30, through: 10).map(\.key) == [30, 20, 10])

  struct Key {
    let group: Int
    let label: String
  }
  let grouped = try SkipList<Key, String>(
    comparator: { $0.group < $1.group },
    levelSource: { 1 }
  )
  #expect(try grouped.insert(Key(group: 1, label: "first"), value: "old"))
  #expect(!(try grouped.insert(Key(group: 1, label: "replacement"), value: "new")))
  #expect(grouped.count == 1)
  #expect(grouped.minimum()?.key.label == "first")
  #expect(grouped.minimum()?.value == "new")
}

@Test func deterministicTraceMatchesSortedReference() throws {
  var nextKey = 0
  let list = try SkipList<Int, Int>(
    maximumLevel: 8,
    comparator: <,
    levelSource: {
      defer { nextKey += 1 }
      return min(nextKey.trailingZeroBitCount + 1, 8)
    }
  )
  let insertionOrder = (0..<200).map { ($0 * 73) % 200 }
  for key in insertionOrder {
    try list.insert(key, value: key * key)
  }
  #expect(list.count == 200)
  #expect(list.validatesStructure())
  for rank in 0..<200 {
    #expect(list.entry(at: rank)?.key == rank)
    #expect(list.rank(of: rank) == rank)
  }

  for key in stride(from: 0, to: 200, by: 3) {
    #expect(list.remove(key))
  }
  let expected = (0..<200).filter { $0 % 3 != 0 }
  #expect(try list.orderedEntries().map(\.key) == expected)
  #expect(list.validatesStructure())
  for (rank, key) in expected.enumerated() {
    #expect(list.rank(of: key) == rank)
    #expect(list.entry(at: rank)?.key == key)
  }
}
