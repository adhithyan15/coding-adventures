import HashFunctions

/// The two collision-resolution strategies required by DT18.
public enum CollisionStrategy: String, Equatable, Sendable {
  case separateChaining
  case linearProbing
}

/// A closed selection of the DT17 hashes admitted by this package.
///
/// Callers that need hash-flooding resistance should supply their own secret
/// 16-byte SipHash key. The deterministic compatibility key used by the
/// convenience initializer is public and is not a security secret.
public enum HashAlgorithm: Equatable, Sendable {
  case fnv1a32
  case murmur3(seed: UInt32)
  case sipHash24(key: [UInt8])

  /// Hash bytes after validating algorithm-specific parameters.
  public func validatedHash(_ bytes: [UInt8]) throws -> UInt64 {
    switch self {
    case .fnv1a32:
      return UInt64(HashFunctions.fnv1a32(bytes))
    case .murmur3(let seed):
      return UInt64(HashFunctions.murmur3_32(bytes, seed: seed))
    case .sipHash24(let key):
      guard key.count == 16 else {
        throw HashMapError.invalidSipHashKeyLength(key.count)
      }
      return try HashFunctions.sipHash24(bytes, key: key)
    }
  }

  func hash(_ bytes: [UInt8]) -> UInt64 {
    // HashMap construction validates SipHash keys once. This helper keeps
    // every table operation non-throwing after that boundary.
    switch self {
    case .fnv1a32:
      return UInt64(HashFunctions.fnv1a32(bytes))
    case .murmur3(let seed):
      return UInt64(HashFunctions.murmur3_32(bytes, seed: seed))
    case .sipHash24(let key):
      // The key was checked by HashMap.init or validatedHash.
      return try! HashFunctions.sipHash24(bytes, key: key)
    }
  }
}

/// A finite, package-owned key domain with an exact cross-process encoding.
///
/// The leading byte distinguishes raw bytes from UTF-8 strings. It is followed
/// by an unsigned 64-bit little-endian payload length and then the payload.
/// String keys use exact UTF-8 identity. Swift's canonical-equivalence
/// `String` equality is deliberately not used because the hash input is not
/// normalized: composed and decomposed spellings remain distinct keys.
/// This avoids Swift's randomized `Hasher`, host object descriptions, Codable,
/// and caller-supplied executable encoders.
public enum HashMapKey: Equatable, Sendable {
  case bytes([UInt8])
  case string(String)

  public static func == (lhs: HashMapKey, rhs: HashMapKey) -> Bool {
    switch (lhs, rhs) {
    case (.bytes(let lhsBytes), .bytes(let rhsBytes)):
      lhsBytes == rhsBytes
    case (.string(let lhsString), .string(let rhsString)):
      lhsString.utf8.elementsEqual(rhsString.utf8)
    default:
      false
    }
  }

  public var encodedBytes: [UInt8] {
    let tag: UInt8
    let payload: [UInt8]
    switch self {
    case .bytes(let bytes):
      tag = 0
      payload = bytes
    case .string(let string):
      tag = 1
      payload = Array(string.utf8)
    }

    var encoded = [tag]
    let length = UInt64(payload.count)
    for shift in stride(from: 0, through: 56, by: 8) {
      encoded.append(UInt8(truncatingIfNeeded: length >> UInt64(shift)))
    }
    encoded.append(contentsOf: payload)
    return encoded
  }

  var payloadByteCount: Int {
    switch self {
    case .bytes(let bytes): bytes.count
    case .string(let string): string.utf8.count
    }
  }
}

/// Stable validation and bounded-work failures.
public enum HashMapError: Error, Equatable, Sendable {
  case invalidCapacity(Int)
  case invalidSipHashKeyLength(Int)
  case keyTooLong(actual: Int, maximum: Int)
  case capacityLimitExceeded
}

/// Fixed production limits keep capacity arithmetic and table work finite.
public enum HashMapLimits {
  public static let maximumCapacity = 1_048_576
  public static let maximumKeyByteCount = 1_048_576

  /// A public deterministic compatibility key matching the established DT18
  /// lane. It is deliberately not represented as secret material.
  public static let deterministicSipHashKey = Array("codex-dt18-key!!".utf8)
}

struct OperationalLimits: Equatable {
  let maximumCapacity: Int
  let maximumKeyByteCount: Int

  static let production = OperationalLimits(
    maximumCapacity: HashMapLimits.maximumCapacity,
    maximumKeyByteCount: HashMapLimits.maximumKeyByteCount
  )
}

/// Copyable configuration sufficient for the downstream DT19 hash-set wrapper.
public struct HashMapConfiguration: Equatable, Sendable {
  public let capacity: Int
  public let strategy: CollisionStrategy
  public let hashAlgorithm: HashAlgorithm
}

/// A persistent hash map implementing separate chaining and linear probing.
///
/// Update methods return a new value. Keys and structural arrays use Swift
/// value semantics. Generic values are retained shallowly: reference-typed
/// values remain shared between callers and map versions.
public struct HashMap<Value> {
  private struct Entry {
    let key: HashMapKey
    let value: Value
  }

  private enum Slot {
    case empty
    case tombstone
    case occupied(Entry)
  }

  public private(set) var count: Int
  public private(set) var capacity: Int
  public let strategy: CollisionStrategy
  public let hashAlgorithm: HashAlgorithm

  private let limits: OperationalLimits
  private var chainBuckets: [[Entry]]
  private var slots: [Slot]

  public init(
    capacity: Int = 16,
    strategy: CollisionStrategy = .separateChaining,
    hashAlgorithm: HashAlgorithm = .sipHash24(
      key: HashMapLimits.deterministicSipHashKey
    )
  ) throws {
    try self.init(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm,
      limits: .production
    )
  }

  init(
    capacity: Int,
    strategy: CollisionStrategy,
    hashAlgorithm: HashAlgorithm,
    limits: OperationalLimits
  ) throws {
    guard capacity > 0 else { throw HashMapError.invalidCapacity(capacity) }
    guard capacity <= limits.maximumCapacity else {
      throw HashMapError.capacityLimitExceeded
    }
    if case .sipHash24(let key) = hashAlgorithm, key.count != 16 {
      throw HashMapError.invalidSipHashKeyLength(key.count)
    }

    self.count = 0
    self.capacity = capacity
    self.strategy = strategy
    self.hashAlgorithm = hashAlgorithm
    self.limits = limits
    switch strategy {
    case .separateChaining:
      self.chainBuckets = Array(repeating: [], count: capacity)
      self.slots = []
    case .linearProbing:
      self.chainBuckets = []
      self.slots = Array(repeating: .empty, count: capacity)
    }
  }

  public var configuration: HashMapConfiguration {
    HashMapConfiguration(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm
    )
  }

  public var loadFactor: Double {
    Double(count) / Double(capacity)
  }

  /// Return a new map containing `key -> value`.
  public func setting(_ value: Value, for key: HashMapKey) throws -> Self {
    try validate(key)
    var copy = self

    if copy.replaceExisting(value, for: key) {
      return copy
    }

    try copy.ensureCapacityForNewEntry()
    copy.insertNew(value, for: key)
    return copy
  }

  /// Return the value associated with `key`, or `nil` when it is absent.
  /// Use `has` to distinguish an absent key from a stored Optional.none.
  public func value(forKey key: HashMapKey) -> Value? {
    guard key.payloadByteCount <= limits.maximumKeyByteCount else { return nil }
    switch strategy {
    case .separateChaining:
      let bucket = chainBuckets[bucketIndex(for: key)]
      return bucket.first(where: { $0.key == key })?.value
    case .linearProbing:
      guard let index = occupiedSlotIndex(for: key) else { return nil }
      if case .occupied(let entry) = slots[index] { return entry.value }
      return nil
    }
  }

  public func has(_ key: HashMapKey) -> Bool {
    guard key.payloadByteCount <= limits.maximumKeyByteCount else { return false }
    switch strategy {
    case .separateChaining:
      return chainBuckets[bucketIndex(for: key)].contains { $0.key == key }
    case .linearProbing:
      return occupiedSlotIndex(for: key) != nil
    }
  }

  /// Return a new map without `key`; missing deletion is a no-op.
  public func removingValue(for key: HashMapKey) -> Self {
    guard key.payloadByteCount <= limits.maximumKeyByteCount else { return self }
    var copy = self
    switch strategy {
    case .separateChaining:
      let index = bucketIndex(for: key)
      guard
        let entryIndex = copy.chainBuckets[index].firstIndex(where: {
          $0.key == key
        })
      else { return self }
      copy.chainBuckets[index].remove(at: entryIndex)
    case .linearProbing:
      guard let index = occupiedSlotIndex(for: key) else { return self }
      copy.slots[index] = .tombstone
    }
    copy.count -= 1
    return copy
  }

  /// Entries in deterministic physical storage order for this exact table
  /// profile (strategy, hash parameters, capacity, and operation history).
  public var entries: [(key: HashMapKey, value: Value)] {
    switch strategy {
    case .separateChaining:
      return chainBuckets.flatMap { bucket in
        bucket.map { (key: $0.key, value: $0.value) }
      }
    case .linearProbing:
      return slots.compactMap { slot in
        guard case .occupied(let entry) = slot else { return nil }
        return (key: entry.key, value: entry.value)
      }
    }
  }

  public var keys: [HashMapKey] { entries.map(\.key) }
  public var values: [Value] { entries.map(\.value) }

  public static func fromEntries(
    _ entries: [(HashMapKey, Value)],
    capacity: Int = 16,
    strategy: CollisionStrategy = .separateChaining,
    hashAlgorithm: HashAlgorithm = .sipHash24(
      key: HashMapLimits.deterministicSipHashKey
    )
  ) throws -> Self {
    var map = try Self(
      capacity: capacity,
      strategy: strategy,
      hashAlgorithm: hashAlgorithm
    )
    for (key, value) in entries {
      map = try map.setting(value, for: key)
    }
    return map
  }

  /// Return a right-biased merge. Both input maps remain unchanged.
  public func merging(_ other: Self) throws -> Self {
    var result = self
    for entry in other.entries {
      result = try result.setting(entry.value, for: entry.key)
    }
    return result
  }

  private func validate(_ key: HashMapKey) throws {
    let actual = key.payloadByteCount
    guard actual <= limits.maximumKeyByteCount else {
      throw HashMapError.keyTooLong(
        actual: actual,
        maximum: limits.maximumKeyByteCount
      )
    }
  }

  private func bucketIndex(for key: HashMapKey, capacity: Int? = nil) -> Int {
    let tableCapacity = capacity ?? self.capacity
    return Int(hashAlgorithm.hash(key.encodedBytes) % UInt64(tableCapacity))
  }

  private func occupiedSlotIndex(for key: HashMapKey) -> Int? {
    let start = bucketIndex(for: key)
    for offset in 0..<capacity {
      let index = (start + offset) % capacity
      switch slots[index] {
      case .empty:
        return nil
      case .tombstone:
        continue
      case .occupied(let entry):
        if entry.key == key { return index }
      }
    }
    return nil
  }

  private mutating func replaceExisting(_ value: Value, for key: HashMapKey) -> Bool {
    switch strategy {
    case .separateChaining:
      let index = bucketIndex(for: key)
      guard
        let entryIndex = chainBuckets[index].firstIndex(where: {
          $0.key == key
        })
      else { return false }
      chainBuckets[index][entryIndex] = Entry(key: key, value: value)
      return true
    case .linearProbing:
      guard let index = occupiedSlotIndex(for: key) else { return false }
      slots[index] = .occupied(Entry(key: key, value: value))
      return true
    }
  }

  private mutating func ensureCapacityForNewEntry() throws {
    let shouldGrow: Bool
    switch strategy {
    case .separateChaining:
      shouldGrow = count == capacity
    case .linearProbing:
      // (count + 1) / capacity > 3 / 4, expressed with bounded integers.
      shouldGrow = (count + 1) * 4 > capacity * 3
    }
    guard shouldGrow else { return }

    let (doubled, overflow) = capacity.multipliedReportingOverflow(by: 2)
    guard !overflow else { throw HashMapError.capacityLimitExceeded }
    let newCapacity = min(doubled, limits.maximumCapacity)
    guard newCapacity > capacity else {
      throw HashMapError.capacityLimitExceeded
    }
    rehash(to: newCapacity)
  }

  private mutating func rehash(to newCapacity: Int) {
    let existingEntries = entries
    capacity = newCapacity
    count = 0
    switch strategy {
    case .separateChaining:
      chainBuckets = Array(repeating: [], count: newCapacity)
    case .linearProbing:
      slots = Array(repeating: .empty, count: newCapacity)
    }
    for entry in existingEntries {
      insertNew(entry.value, for: entry.key)
    }
  }

  private mutating func insertNew(_ value: Value, for key: HashMapKey) {
    let entry = Entry(key: key, value: value)
    switch strategy {
    case .separateChaining:
      chainBuckets[bucketIndex(for: key)].append(entry)
    case .linearProbing:
      let start = bucketIndex(for: key)
      var firstTombstone: Int?
      for offset in 0..<capacity {
        let index = (start + offset) % capacity
        switch slots[index] {
        case .empty:
          slots[firstTombstone ?? index] = .occupied(entry)
          count += 1
          return
        case .tombstone:
          if firstTombstone == nil { firstTombstone = index }
        case .occupied:
          continue
        }
      }
      // A tombstone-only table still has a valid insertion position.
      if let firstTombstone {
        slots[firstTombstone] = .occupied(entry)
        count += 1
        return
      }
      preconditionFailure("validated load threshold left no insertion slot")
    }
    count += 1
  }
}
