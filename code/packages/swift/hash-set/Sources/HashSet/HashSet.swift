import HashMap

/// A persistent DT19 hash set backed by the DT18 `HashMap` package.
///
/// A set needs only the key half of a map. The private `Bool` value below is a
/// sentinel: membership is always decided with `HashMap.has`, never by reading
/// that value. Every operation returns a new set, so older values remain valid.
public struct HashSet {
  private var storage: HashMap<Bool>

  /// Create an empty set with a finite, explicit hash-table profile.
  public init(
    capacity: Int = 16,
    strategy: CollisionStrategy = .separateChaining,
    hashAlgorithm: HashAlgorithm = .sipHash24(
      key: HashMapLimits.deterministicSipHashKey
    )
  ) throws {
    storage = try HashMap(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm
    )
  }

  private init(storage: HashMap<Bool>) {
    self.storage = storage
  }

  /// Build a set from keys, discarding duplicates.
  public static func fromElements<S: Sequence>(
    _ elements: S,
    capacity: Int = 16,
    strategy: CollisionStrategy = .separateChaining,
    hashAlgorithm: HashAlgorithm = .sipHash24(
      key: HashMapLimits.deterministicSipHashKey
    )
  ) throws -> Self where S.Element == HashMapKey {
    var result = try Self(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm
    )
    for element in elements {
      result = try result.adding(element)
    }
    return result
  }

  /// The current table profile. Capacity may grow as elements are inserted.
  public var configuration: HashMapConfiguration { storage.configuration }

  /// Number of distinct elements.
  public var count: Int { storage.count }

  public var isEmpty: Bool { count == 0 }

  /// Elements in the underlying map's deterministic physical storage order.
  ///
  /// This is not insertion or sorted order. It is stable only for an identical
  /// strategy, hash profile, initial capacity, and operation history.
  public var elements: [HashMapKey] { storage.keys }

  public func contains(_ element: HashMapKey) -> Bool {
    storage.has(element)
  }

  /// Return a new set containing `element`.
  public func adding(_ element: HashMapKey) throws -> Self {
    Self(storage: try storage.setting(true, for: element))
  }

  /// Return a new set without `element`; a missing element is a no-op.
  public func removing(_ element: HashMapKey) -> Self {
    Self(storage: storage.removingValue(for: element))
  }

  /// Python-style spelling for non-failing removal.
  public func discarding(_ element: HashMapKey) -> Self {
    removing(element)
  }

  // MARK: - Set algebra

  /// Return A ∪ B while retaining the receiver's table profile.
  public func union(_ other: Self) throws -> Self {
    var result = self
    for element in other.elements {
      result = try result.adding(element)
    }
    return result
  }

  /// Return A ∩ B. Iterate the smaller input but retain the receiver profile.
  public func intersection(_ other: Self) throws -> Self {
    let (smaller, larger) = count <= other.count ? (self, other) : (other, self)
    var result = try emptyWithReceiverProfile()
    for element in smaller.elements where larger.contains(element) {
      result = try result.adding(element)
    }
    return result
  }

  /// Return A − B.
  public func difference(_ other: Self) throws -> Self {
    var result = try emptyWithReceiverProfile()
    for element in elements where !other.contains(element) {
      result = try result.adding(element)
    }
    return result
  }

  /// Return keys in exactly one input, A △ B.
  public func symmetricDifference(_ other: Self) throws -> Self {
    var result = try emptyWithReceiverProfile()
    for element in elements where !other.contains(element) {
      result = try result.adding(element)
    }
    for element in other.elements where !contains(element) {
      result = try result.adding(element)
    }
    return result
  }

  // MARK: - Relations

  public func isSubset(of other: Self) -> Bool {
    guard count <= other.count else { return false }
    return elements.allSatisfy(other.contains)
  }

  public func isSuperset(of other: Self) -> Bool {
    other.isSubset(of: self)
  }

  public func isDisjoint(with other: Self) -> Bool {
    let (smaller, larger) = count <= other.count ? (self, other) : (other, self)
    return smaller.elements.allSatisfy { !larger.contains($0) }
  }

  public func equals(_ other: Self) -> Bool {
    count == other.count && isSubset(of: other)
  }

  /// Algebra creates fresh maps instead of exposing DT18 buckets. Starting
  /// with the receiver's current capacity avoids unnecessary growth while
  /// preserving its collision strategy and explicit hash parameters.
  private func emptyWithReceiverProfile() throws -> Self {
    let profile = configuration
    return try Self(
      capacity: profile.capacity,
      strategy: profile.strategy,
      hashAlgorithm: profile.hashAlgorithm
    )
  }
}
