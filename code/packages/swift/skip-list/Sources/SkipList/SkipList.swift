/// Stable failures produced by ``SkipList`` configuration and bounded operations.
public enum SkipListError: Error, Equatable, Sendable {
  case invalidMaximumLevel(Int)
  case invalidMaximumCount(Int)
  case invalidGeneratedLevel(level: Int, maximum: Int)
  case capacityExceeded(maximum: Int)
  case invalidOutputLimit(Int)
  case outputLimitExceeded(limit: Int)
}

/// An owned structural snapshot of one key/value entry.
public struct SkipListEntry<Key, Value> {
  public let key: Key
  public let value: Value

  public init(key: Key, value: Value) {
    self.key = key
    self.value = value
  }
}

/// A mutable, span-augmented ordered map with caller-controlled node levels.
///
/// The comparator must define a stable strict weak order. Comparator-equivalent
/// keys identify the same entry. The level source is called exactly once for a
/// new key and never for an update. This type is intentionally not thread-safe.
public final class SkipList<Key, Value> {
  public static var absoluteMaximumLevel: Int { 32 }
  public static var absoluteMaximumCount: Int { 1_048_576 }
  public static var absoluteMaximumOutputCount: Int { 65_536 }

  private final class Node {
    var key: Key?
    var value: Value?
    var forward: [Node?]
    var spans: [Int]

    init(key: Key?, value: Value?, height: Int) {
      self.key = key
      self.value = value
      forward = [Node?](repeating: nil, count: height)
      spans = [Int](repeating: 0, count: height)
    }
  }

  private let comparator: (Key, Key) -> Bool
  private let levelSource: () -> Int
  public let maximumLevel: Int
  public let maximumCount: Int
  private let head: Node
  private var tail: Node?
  private var currentLevel = 1

  public private(set) var count = 0

  public var isEmpty: Bool { count == 0 }
  public var currentMaximumLevel: Int { currentLevel }

  public init(
    maximumLevel: Int = 32,
    maximumCount: Int = 1_048_576,
    comparator: @escaping (Key, Key) -> Bool,
    levelSource: @escaping () -> Int
  ) throws {
    guard (1...Self.absoluteMaximumLevel).contains(maximumLevel) else {
      throw SkipListError.invalidMaximumLevel(maximumLevel)
    }
    guard (1...Self.absoluteMaximumCount).contains(maximumCount) else {
      throw SkipListError.invalidMaximumCount(maximumCount)
    }

    self.maximumLevel = maximumLevel
    self.maximumCount = maximumCount
    self.comparator = comparator
    self.levelSource = levelSource
    head = Node(key: nil, value: nil, height: maximumLevel)
  }

  /// Inserts or updates a key. Returns `true` only when the count grows.
  @discardableResult
  public func insert(_ key: Key, value: Value) throws -> Bool {
    let path = searchPath(for: key)
    if let candidate = path.update[0].forward[0], keysEqual(candidateKey(candidate), key) {
      candidate.value = .some(value)
      return false
    }

    guard count < maximumCount else {
      throw SkipListError.capacityExceeded(maximum: maximumCount)
    }

    let newLevel = levelSource()
    guard (1...maximumLevel).contains(newLevel) else {
      throw SkipListError.invalidGeneratedLevel(level: newLevel, maximum: maximumLevel)
    }

    var update = path.update
    var ranks = path.ranks
    if newLevel > currentLevel {
      for level in currentLevel..<newLevel {
        ranks[level] = 0
        update[level] = head
        head.spans[level] = count
      }
      currentLevel = newLevel
    }

    let node = Node(key: .some(key), value: .some(value), height: newLevel)
    for level in 0..<newLevel {
      let distance = ranks[0] - ranks[level]
      node.forward[level] = update[level].forward[level]
      node.spans[level] = update[level].spans[level] - distance
      update[level].forward[level] = node
      update[level].spans[level] = distance + 1
    }
    if newLevel < currentLevel {
      for level in newLevel..<currentLevel {
        update[level].spans[level] += 1
      }
    }

    if node.forward[0] == nil {
      tail = node
    }
    count += 1
    return true
  }

  public func value(for key: Key) -> Value? {
    guard let node = node(for: key) else { return nil }
    return nodeValue(node)
  }

  public func contains(_ key: Key) -> Bool {
    node(for: key) != nil
  }

  @discardableResult
  public func remove(_ key: Key) -> Bool {
    let path = searchPath(for: key)
    guard let target = path.update[0].forward[0], keysEqual(candidateKey(target), key) else {
      return false
    }

    for level in 0..<currentLevel {
      if path.update[level].forward[level] === target {
        path.update[level].spans[level] += target.spans[level] - 1
        path.update[level].forward[level] = target.forward[level]
      } else {
        path.update[level].spans[level] -= 1
      }
    }

    if tail === target {
      tail = path.update[0] === head ? nil : path.update[0]
    }
    count -= 1
    while currentLevel > 1 && head.forward[currentLevel - 1] == nil {
      currentLevel -= 1
    }
    return true
  }

  /// Returns the zero-based rank of a key, or `nil` when it is absent.
  public func rank(of key: Key) -> Int? {
    var traversed = 0
    var current = head
    for level in stride(from: currentLevel - 1, through: 0, by: -1) {
      while let next = current.forward[level] {
        let nextKey = candidateKey(next)
        if comparator(key, nextKey) {
          break
        }
        traversed += current.spans[level]
        current = next
      }
    }
    guard current !== head, keysEqual(candidateKey(current), key) else { return nil }
    return traversed - 1
  }

  /// Returns the entry at a zero-based rank, or `nil` when out of range.
  public func entry(at rank: Int) -> SkipListEntry<Key, Value>? {
    guard rank >= 0 && rank < count else { return nil }
    let target = rank + 1
    var traversed = 0
    var current = head
    for level in stride(from: currentLevel - 1, through: 0, by: -1) {
      while current.forward[level] != nil && traversed + current.spans[level] <= target {
        traversed += current.spans[level]
        current = current.forward[level]!
      }
    }
    guard traversed == target else { return nil }
    return makeEntry(current)
  }

  public func minimum() -> SkipListEntry<Key, Value>? {
    guard let first = head.forward[0] else { return nil }
    return makeEntry(first)
  }

  public func maximum() -> SkipListEntry<Key, Value>? {
    guard let tail else { return nil }
    return makeEntry(tail)
  }

  /// Returns entries in comparator order within the requested bounds.
  public func entries(
    from lowerBound: Key,
    through upperBound: Key,
    includingLower: Bool = true,
    includingUpper: Bool = true,
    limit: Int = 65_536
  ) throws -> [SkipListEntry<Key, Value>] {
    try validateOutputLimit(limit)
    if comparator(upperBound, lowerBound) {
      return []
    }

    var current = head
    for level in stride(from: currentLevel - 1, through: 0, by: -1) {
      while let next = current.forward[level] {
        let key = candidateKey(next)
        let beforeLower = comparator(key, lowerBound)
        let equalLower = keysEqual(key, lowerBound)
        if beforeLower || (!includingLower && equalLower) {
          current = next
        } else {
          break
        }
      }
    }

    var result: [SkipListEntry<Key, Value>] = []
    var candidate = current.forward[0]
    while let node = candidate {
      let key = candidateKey(node)
      let beyondUpper = comparator(upperBound, key)
      let equalUpper = keysEqual(key, upperBound)
      if beyondUpper || (!includingUpper && equalUpper) {
        break
      }
      guard result.count < limit else {
        throw SkipListError.outputLimitExceeded(limit: limit)
      }
      result.append(makeEntry(node))
      candidate = node.forward[0]
    }
    return result
  }

  /// Returns a structurally independent ordered snapshot.
  public func orderedEntries(limit: Int = 65_536) throws -> [SkipListEntry<Key, Value>] {
    try validateOutputLimit(limit)
    guard count <= limit else {
      throw SkipListError.outputLimitExceeded(limit: limit)
    }
    var result: [SkipListEntry<Key, Value>] = []
    result.reserveCapacity(count)
    var current = head.forward[0]
    while let node = current {
      result.append(makeEntry(node))
      current = node.forward[0]
    }
    return result
  }

  private func validateOutputLimit(_ limit: Int) throws {
    guard (1...Self.absoluteMaximumOutputCount).contains(limit) else {
      throw SkipListError.invalidOutputLimit(limit)
    }
  }

  private func node(for key: Key) -> Node? {
    let predecessor = searchPath(for: key).update[0]
    guard let candidate = predecessor.forward[0], keysEqual(candidateKey(candidate), key) else {
      return nil
    }
    return candidate
  }

  private func searchPath(for key: Key) -> (update: [Node], ranks: [Int]) {
    var update = [Node](repeating: head, count: maximumLevel)
    var ranks = [Int](repeating: 0, count: maximumLevel)
    var current = head
    for level in stride(from: currentLevel - 1, through: 0, by: -1) {
      ranks[level] = level == currentLevel - 1 ? 0 : ranks[level + 1]
      while let next = current.forward[level], comparator(candidateKey(next), key) {
        ranks[level] += current.spans[level]
        current = next
      }
      update[level] = current
    }
    return (update, ranks)
  }

  private func keysEqual(_ left: Key, _ right: Key) -> Bool {
    !comparator(left, right) && !comparator(right, left)
  }

  private func candidateKey(_ node: Node) -> Key {
    guard case .some(let key) = node.key else {
      preconditionFailure("sentinel nodes are not entries")
    }
    return key
  }

  private func nodeValue(_ node: Node) -> Value {
    guard case .some(let value) = node.value else {
      preconditionFailure("sentinel nodes do not contain values")
    }
    return value
  }

  private func makeEntry(_ node: Node) -> SkipListEntry<Key, Value> {
    SkipListEntry(key: candidateKey(node), value: nodeValue(node))
  }
}

struct SkipListDebugLink<Key> {
  let destination: Key
  let span: Int
}

extension SkipList {
  func debugLevelSnapshot() -> [[SkipListDebugLink<Key>]] {
    (0..<currentLevel).map { level in
      var links: [SkipListDebugLink<Key>] = []
      var current = head
      while let next = current.forward[level] {
        links.append(SkipListDebugLink(destination: candidateKey(next), span: current.spans[level]))
        current = next
      }
      return links
    }
  }

  func validatesStructure() -> Bool {
    var bottomRanks: [ObjectIdentifier: Int] = [ObjectIdentifier(head): 0]
    var bottom = head.forward[0]
    var expectedRank = 1
    var previousKey: Key?
    while let node = bottom {
      let key = candidateKey(node)
      if let previousKey, !comparator(previousKey, key) {
        return false
      }
      guard node.forward.count == node.spans.count else { return false }
      bottomRanks[ObjectIdentifier(node)] = expectedRank
      expectedRank += 1
      previousKey = key
      bottom = node.forward[0]
    }
    guard expectedRank - 1 == count else { return false }
    guard (count == 0 && tail == nil) || (count > 0 && tail != nil) else { return false }

    for level in 0..<currentLevel {
      var current = head
      while let next = current.forward[level] {
        guard
          let currentRank = bottomRanks[ObjectIdentifier(current)],
          let nextRank = bottomRanks[ObjectIdentifier(next)],
          current.spans[level] == nextRank - currentRank
        else { return false }
        current = next
      }
    }
    return currentLevel == 1 || head.forward[currentLevel - 1] != nil
  }
}
