import Foundation
import XCTest

import HashMap

private struct FixtureFailure: Error, CustomStringConvertible {
  let description: String
}

private final class FixtureReferenceValue {
  var field: Int

  init(field: Int) {
    self.field = field
  }
}

final class DT18FixtureTests: XCTestCase {
  private func fixture() throws -> [String: Any] {
    var repositoryRoot = URL(fileURLWithPath: #filePath)
    for _ in 0..<7 {
      repositoryRoot.deleteLastPathComponent()
    }
    let path = repositoryRoot.appendingPathComponent(
      "code/specs/fixtures/hash-map-v1/cases.json"
    )
    let data = try Data(contentsOf: path)
    guard let result = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
      throw FixtureFailure(description: "DT18 cases.json must contain a JSON object")
    }
    return result
  }

  private func object(_ value: Any?, _ name: String) throws -> [String: Any] {
    guard let value = value as? [String: Any] else {
      throw FixtureFailure(description: "\(name) must be a JSON object")
    }
    return value
  }

  private func rows(_ value: Any?, _ name: String) throws -> [[String: Any]] {
    guard let value = value as? [[String: Any]] else {
      throw FixtureFailure(description: "\(name) must be a JSON array of objects")
    }
    return value
  }

  private func string(_ value: Any?, _ name: String) throws -> String {
    guard let value = value as? String else {
      throw FixtureFailure(description: "\(name) must be a string")
    }
    return value
  }

  private func integer(_ value: Any?, _ name: String) throws -> Int {
    guard let number = value as? NSNumber else {
      throw FixtureFailure(description: "\(name) must be an integer")
    }
    return number.intValue
  }

  private func require(_ condition: Bool, _ message: String) throws {
    guard condition else { throw FixtureFailure(description: message) }
  }

  private func bytes(_ hex: String) throws -> [UInt8] {
    try require(hex.count.isMultiple(of: 2), "hex string has odd length: \(hex.prefix(48))")
    try require(
      hex == hex.lowercased() && hex.allSatisfy(\.isHexDigit),
      "hex string is not lowercase hexadecimal: \(hex.prefix(48))"
    )
    var output: [UInt8] = []
    output.reserveCapacity(hex.count / 2)
    var cursor = hex.startIndex
    while cursor < hex.endIndex {
      let next = hex.index(cursor, offsetBy: 2)
      guard let byte = UInt8(hex[cursor..<next], radix: 16) else {
        throw FixtureFailure(description: "invalid hexadecimal byte")
      }
      output.append(byte)
      cursor = next
    }
    return output
  }

  private func hex(_ value: UInt64, width: Int) -> String {
    let digits = String(value, radix: 16)
    return String(repeating: "0", count: max(0, width - digits.count)) + digits
  }

  private func hashConfiguration(
    _ row: [String: Any]
  ) throws -> HashAlgorithm {
    switch try string(row["kind"], "hash configuration kind") {
    case "fnv1a32":
      return .fnv1a32
    case "murmur3":
      let seedText = try string(row["seed_hex"], "Murmur seed")
      guard let seed = UInt32(seedText, radix: 16) else {
        throw FixtureFailure(description: "Murmur seed is not a UInt32 hexadecimal value")
      }
      return .murmur3(seed: seed)
    case "sipHash24":
      return .sipHash24(key: try bytes(string(row["key_hex"], "SipHash key")))
    default:
      throw FixtureFailure(description: "unknown DT18 hash configuration")
    }
  }

  private func hashConfiguration(
    id: String,
    configurations: [String: [String: Any]]
  ) throws -> HashAlgorithm {
    guard let row = configurations[id] else {
      throw FixtureFailure(description: "missing hash configuration \(id)")
    }
    return try hashConfiguration(row)
  }

  private func hashConfigurations(_ fixture: [String: Any]) throws -> [String: [String: Any]] {
    var result: [String: [String: Any]] = [:]
    for row in try rows(fixture["hash_configurations"], "hash_configurations") {
      let id = try string(row["id"], "hash configuration id")
      try require(result[id] == nil, "duplicate hash configuration id \(id)")
      result[id] = row
    }
    return result
  }

  private func keyCases(_ fixture: [String: Any]) throws -> [String: [String: Any]] {
    var result: [String: [String: Any]] = [:]
    for row in try rows(fixture["key_cases"], "key_cases") {
      let id = try string(row["id"], "key id")
      try require(result[id] == nil, "duplicate key id \(id)")
      result[id] = row
    }
    return result
  }

  private func key(_ row: [String: Any]) throws -> HashMapKey {
    let payload = try bytes(string(row["payload_hex"], "key payload"))
    let result: HashMapKey
    switch try string(row["kind"], "key kind") {
    case "bytes":
      result = .bytes(payload)
    case "string":
      guard let value = String(bytes: payload, encoding: .utf8) else {
        throw FixtureFailure(description: "shared Swift key payload must be valid UTF-8")
      }
      result = .string(value)
    default:
      throw FixtureFailure(description: "unknown shared key kind")
    }
    try require(
      result.encodedBytes == (try bytes(string(row["encoded_hex"], "encoded key"))),
      "public key encoding differs for \(row["id"] ?? "unknown key")"
    )
    return result
  }

  private func keys(_ fixture: [String: Any]) throws -> [String: HashMapKey] {
    var result: [String: HashMapKey] = [:]
    for row in try rows(fixture["key_cases"], "key_cases") {
      result[try string(row["id"], "key id")] = try key(row)
    }
    return result
  }

  private func values(_ fixture: [String: Any]) throws -> [String: Int?] {
    var result: [String: Int?] = [:]
    for row in try rows(fixture["values"], "values") {
      let id = try string(row["id"], "value id")
      if row["value"] is NSNull {
        result[id] = .some(nil)
      } else {
        result[id] = .some(try integer(row["value"], "fixture value"))
      }
    }
    return result
  }

  private func optionalInteger(_ value: Any?, _ name: String) throws -> Int? {
    guard let value else { throw FixtureFailure(description: "missing \(name)") }
    if value is NSNull { return nil }
    return try integer(value, name)
  }

  private func fixtureValue(
    id: String,
    values: [String: Int?]
  ) throws -> Int? {
    guard let value = values[id] else {
      throw FixtureFailure(description: "missing fixture value id \(id)")
    }
    return value
  }

  private func strategy(_ value: Any?) throws -> CollisionStrategy {
    switch try string(value, "collision strategy") {
    case "separateChaining": return .separateChaining
    case "linearProbing": return .linearProbing
    default: throw FixtureFailure(description: "unknown collision strategy")
    }
  }

  private func assertState(
    _ map: HashMap<Int?>,
    expected: [String: Any],
    keys: [String: HashMapKey]
  ) throws {
    let count = try integer(expected["count"], "expected map count")
    let capacity = try integer(expected["capacity"], "expected map capacity")
    try require(map.count == count, "map count \(map.count), expected \(count)")
    try require(map.capacity == capacity, "map capacity \(map.capacity), expected \(capacity)")
    guard let loadRatio = expected["load_factor_ratio"] as? [NSNumber], loadRatio.count == 2 else {
      throw FixtureFailure(description: "load_factor_ratio must contain numerator and denominator")
    }
    let numerator = loadRatio[0].doubleValue
    let denominator = loadRatio[1].doubleValue
    try require(denominator > 0, "load_factor_ratio denominator must be positive")
    let expectedLoad = numerator / denominator
    try require(
      abs(map.loadFactor - expectedLoad) <= 1e-15,
      "map load factor \(map.loadFactor), expected \(expectedLoad)"
    )
    let expectedEntries = try rows(expected["entries"], "expected entries")
    try require(map.entries.count == expectedEntries.count, "entry count differs")
    for (actual, expectedEntry) in zip(map.entries, expectedEntries) {
      let keyID = try string(expectedEntry["key_id"], "expected entry key id")
      guard let expectedKey = keys[keyID] else {
        throw FixtureFailure(description: "missing expected key \(keyID)")
      }
      try require(actual.key == expectedKey, "entry key differs at physical index for \(keyID)")
      let expectedValue = try optionalInteger(expectedEntry["value"], "expected entry value")
      try require(actual.value == expectedValue, "entry value differs for key \(keyID)")
    }
  }

  private func assertConfiguration(
    _ map: HashMap<Int?>,
    expected: [String: Any],
    configurations: [String: [String: Any]]
  ) throws {
    try require(map.capacity == integer(expected["capacity"], "expected capacity"), "configuration capacity differs")
    try require(map.strategy == strategy(expected["strategy"]), "configuration strategy differs")
    let algorithmID = try string(expected["hash_configuration_id"], "hash configuration id")
    try require(
      map.hashAlgorithm == (try hashConfiguration(id: algorithmID, configurations: configurations)),
      "configuration hash algorithm differs for \(algorithmID)"
    )
  }

  private func assertObservation(
    _ map: HashMap<Int?>,
    key: HashMapKey,
    expected: [String: Any]
  ) throws {
    guard let expectedHas = expected["has"] as? Bool else {
      throw FixtureFailure(description: "observation has must be a boolean")
    }
    try require(map.has(key) == expectedHas, "public has observation differs")
    if let expectedValue = expected["value"] {
      if expectedHas {
        guard let stored = map.value(forKey: key) else {
          throw FixtureFailure(description: "present key has no outer optional value")
        }
        try require(stored == (try optionalInteger(expectedValue, "observation value")), "public value observation differs")
      } else {
        try require(map.value(forKey: key) == nil, "missing key returned a value")
        try require(expectedValue is NSNull, "missing-key observation has a non-null value")
      }
    }
  }

  private func compareHashAnswer(
    _ row: [String: Any],
    configurations: [String: [String: Any]],
    expectedOverride: String? = nil
  ) throws -> Bool {
    let configurationID = try string(row["configuration_id"], "hash answer configuration id")
    let algorithm = try hashConfiguration(id: configurationID, configurations: configurations)
    let data = try bytes(string(row["data_hex"], "hash answer input"))
    let expected = try expectedOverride ?? string(row["expected_hex"], "expected hash")
    let actual = hex(try algorithm.validatedHash(data), width: expected.count)
    return actual == expected
  }

  func testFixtureHashesAndKeyEncodingsUsePublicSwiftAPIs() throws {
    let document = try fixture()
    let configurations = try hashConfigurations(document)
    let allKeys = try keys(document)
    try require(allKeys.count == 19, "unexpected finalized shared key-case count")

    let answers = try rows(document["hash_answers"], "hash_answers")
    try require(answers.count == 4, "expected four known-answer hashes")
    for answer in answers {
      let passed = try compareHashAnswer(answer, configurations: configurations)
      try require(passed, "public hash answer differs: \(answer["id"] ?? "unknown")")
    }

    let encodedHashes = try rows(document["encoded_key_hashes"], "encoded_key_hashes")
    try require(encodedHashes.count == 76, "expected 76 encoded-key hash vectors")
    let keyCases = try keyCases(document)
    for row in encodedHashes {
      let keyID = try string(row["key_id"], "encoded hash key id")
      guard let keyCase = keyCases[keyID], let publicKey = allKeys[keyID] else {
        throw FixtureFailure(description: "missing encoded-hash key \(keyID)")
      }
      try require(
        publicKey.encodedBytes == (try bytes(string(row["input_hex"], "encoded hash input"))),
        "encoded hash input differs for \(keyID)"
      )
      let configurationID = try string(row["configuration_id"], "encoded hash configuration id")
      let algorithm = try hashConfiguration(id: configurationID, configurations: configurations)
      let expected = try string(row["expected_hex"], "encoded hash expected value")
      let actual = hex(try algorithm.validatedHash(publicKey.encodedBytes), width: expected.count)
      try require(actual == expected, "public encoded-key hash differs for \(keyID)/\(configurationID)")
      try require(keyCase["encoded_hex"] != nil, "encoded key case omitted its canonical encoding")
    }

    let control = try rows(document["negative_controls"], "negative_controls").first
    guard let control else { throw FixtureFailure(description: "missing hash negative control") }
    try require(control["well_formed"] as? Bool == true, "negative control must be well formed")
    let sourceID = try string(control["source_hash_answer_id"], "negative control source answer")
    guard let source = answers.first(where: { ($0["id"] as? String) == sourceID }) else {
      throw FixtureFailure(description: "negative control refers to a missing public hash answer")
    }
    let mutatedExpected = try string(control["mutated_expected_hex"], "mutated hash expectation")
    let sourceExpected = try string(source["expected_hex"], "source hash expectation")
    try require(mutatedExpected.count == sourceExpected.count, "mutated hash expectation has wrong width")
    try require(mutatedExpected != sourceExpected, "negative control did not change the expected hash")
    let corruptedPasses = try compareHashAnswer(
      source,
      configurations: configurations,
      expectedOverride: mutatedExpected
    )
    try require(!corruptedPasses, "public hash comparator accepted the mutated expected output")
  }

  func testConfigurationAndDirectedOperationCases() throws {
    let document = try fixture()
    let configurations = try hashConfigurations(document)
    let keys = try keys(document)
    let values = try values(document)

    for row in try rows(document["configuration_cases"], "configuration_cases") {
      let arguments = try object(row["arguments"], "configuration arguments")
      let expected = try object(row["expected"], "configuration expected")
      let map: HashMap<Int?>
      if arguments["hash_configuration_omitted"] as? Bool == true {
        map = try HashMap<Int?>()
      } else {
        map = try HashMap<Int?>(
          capacity: integer(arguments["capacity"], "configuration argument capacity"),
          strategy: strategy(arguments["strategy"]),
          hashAlgorithm: hashConfiguration(
            id: string(arguments["hash_configuration_id"], "configuration argument hash id"),
            configurations: configurations
          )
        )
      }
      try assertConfiguration(map, expected: expected, configurations: configurations)
    }

    let operationCases = try rows(document["operation_cases"], "operation_cases")
    try require(operationCases.count == 6, "expected six directed operation cases")
    for operationCase in operationCases {
      let id = try string(operationCase["id"], "operation case id")
      let configuration = try object(operationCase["configuration"], "operation configuration")
      let algorithmID = try string(configuration["hash_configuration_id"], "operation hash configuration id")
      var map: HashMap<Int?>
      if configuration["hash_configuration_omitted"] as? Bool == true {
        map = try HashMap<Int?>()
      } else {
        map = try HashMap<Int?>(
          capacity: integer(configuration["capacity"], "operation capacity"),
          strategy: strategy(configuration["strategy"]),
          hashAlgorithm: hashConfiguration(id: algorithmID, configurations: configurations)
        )
      }
      for step in try rows(operationCase["steps"], "operation steps") {
        let op = try string(step["op"], "operation name")
        let expected = try object(step["expected"], "operation expected result")
        if let error = expected["error_id"], !(error is NSNull) {
          throw FixtureFailure(description: "shared directed operation unexpectedly expects an error")
        }
        let keyID = try string(step["key_id"], "operation key id")
        guard let key = keys[keyID] else { throw FixtureFailure(description: "missing operation key \(keyID)") }
        switch op {
        case "set":
          let valueID = try string(step["value_id"], "operation value id")
          map = try map.setting(fixtureValue(id: valueID, values: values), for: key)
        case "remove":
          map = map.removingValue(for: key)
        case "get", "has":
          break
        default:
          throw FixtureFailure(description: "unknown directed operation \(op)")
        }
        let expectedState = try object(expected["state"], "operation expected state")
        try assertState(map, expected: expectedState, keys: keys)
        if let observationValue = expected["observation"] {
          try assertObservation(map, key: key, expected: object(observationValue, "operation observation"))
        }
      }
      try require(!id.isEmpty, "operation case id must not be empty")
    }
  }

  func testFromEntriesMergeAndOwnershipCases() throws {
    let document = try fixture()
    let configurations = try hashConfigurations(document)
    let keys = try keys(document)
    let values = try values(document)

    let fromEntriesRows = try rows(document["from_entries_cases"], "from_entries_cases")
    try require(fromEntriesRows.count == 1, "expected one fromEntries case")
    for row in fromEntriesRows {
      let config = try object(row["configuration"], "fromEntries configuration")
      let source = try rows(row["source_entries"], "fromEntries source entries")
      let pairs = try source.map { entry -> (HashMapKey, Int?) in
        let keyID = try string(entry["key_id"], "fromEntries key id")
        let valueID = try string(entry["value_id"], "fromEntries value id")
        guard let key = keys[keyID] else { throw FixtureFailure(description: "missing fromEntries key \(keyID)") }
        return (key, try fixtureValue(id: valueID, values: values))
      }
      let map = try HashMap<Int?>.fromEntries(
        pairs,
        capacity: integer(config["capacity"], "fromEntries capacity"),
        strategy: strategy(config["strategy"]),
        hashAlgorithm: hashConfiguration(
          id: string(config["hash_configuration_id"], "fromEntries hash id"),
          configurations: configurations
        )
      )
      try assertState(map, expected: object(row["expected_state"], "fromEntries expected state"), keys: keys)
    }

    let mergeCase = try rows(document["merge_cases"], "merge_cases").first
    guard let mergeCase else { throw FixtureFailure(description: "missing merge case") }
    func makeMap(_ side: [String: Any]) throws -> HashMap<Int?> {
      let config = try object(side["configuration"], "merge configuration")
      let source = try rows(side["source_entries"], "merge source entries")
      let pairs = try source.map { entry -> (HashMapKey, Int?) in
        let keyID = try string(entry["key_id"], "merge key id")
        let valueID = try string(entry["value_id"], "merge value id")
        guard let key = keys[keyID] else { throw FixtureFailure(description: "missing merge key \(keyID)") }
        return (key, try fixtureValue(id: valueID, values: values))
      }
      return try HashMap<Int?>.fromEntries(
        pairs,
        capacity: integer(config["capacity"], "merge capacity"),
        strategy: strategy(config["strategy"]),
        hashAlgorithm: hashConfiguration(
          id: string(config["hash_configuration_id"], "merge hash id"),
          configurations: configurations
        )
      )
    }
    let left = try makeMap(object(mergeCase["left"], "merge left"))
    let right = try makeMap(object(mergeCase["right"], "merge right"))
    let leftBefore = left
    let rightBefore = right
    guard let expectedRightIDs = mergeCase["expected_right_physical_order"] as? [String] else {
      throw FixtureFailure(description: "expected_right_physical_order must be an array")
    }
    try require(
      right.entries.map(\.key) == expectedRightIDs.compactMap { keys[$0] },
      "right merge input physical order differs"
    )
    let result = try left.merging(right)
    let expectedResult = try object(mergeCase["expected_result"], "expected merge result")
    try assertConfiguration(result, expected: object(expectedResult["configuration"], "merge result configuration"), configurations: configurations)
    try assertState(result, expected: object(expectedResult["state"], "merge result state"), keys: keys)
    try require(left.count == leftBefore.count && left.entries.map(\.key) == leftBefore.entries.map(\.key), "merge mutated left input")
    try require(right.count == rightBefore.count && right.entries.map(\.key) == rightBefore.entries.map(\.key), "merge mutated right input")
    try assertState(left, expected: object(object(mergeCase["left"], "merge left")["expected_state"], "left expected state"), keys: keys)
    try assertState(right, expected: object(object(mergeCase["right"], "merge right")["expected_state"], "right expected state"), keys: keys)

    let ownership = try rows(document["ownership_cases"], "ownership_cases")
    try require(ownership.count == 5, "expected five ownership cases")
    for row in ownership {
      let id = try string(row["id"], "ownership case id")
      switch id {
      case "byte-key-copies-caller-list":
        var source = try bytes(string(row["source_payload_hex"], "source key payload"))
        let keyID = try string(row["key_id"], "ownership key id")
        let expected = try bytes(string(row["expected_encoded_hex"], "expected encoded key"))
        let owned = HashMapKey.bytes(source)
        let mutation = try object(row["mutation"], "key mutation")
        source[try integer(mutation["index"], "mutation index")] =
          UInt8(try integer(mutation["new_value"], "mutation value"))
        try require(owned.encodedBytes == expected, "byte key changed after caller buffer mutation for \(keyID)")
      case "encoded-key-getter-returns-detached-list":
        let keyID = try string(row["key_id"], "getter key id")
        guard let key = keys[keyID] else { throw FixtureFailure(description: "missing getter key \(keyID)") }
        var returned = key.encodedBytes
        let mutation = try object(row["mutation"], "encoded getter mutation")
        returned[try integer(mutation["index"], "mutation index")] = UInt8(try integer(mutation["new_value"], "mutation value"))
        try require(key.encodedBytes == (try bytes(string(row["expected_encoded_hex"], "expected encoding"))), "encoded key getter exposed mutable storage")
      case "siphash-configuration-copies-caller-list":
        var source = try bytes(string(row["source_key_hex"], "SipHash source key"))
        let algorithm = HashAlgorithm.sipHash24(key: source)
        let mutation = try object(row["mutation"], "SipHash key mutation")
        source[try integer(mutation["index"], "mutation index")] = UInt8(try integer(mutation["new_value"], "mutation value"))
        let probe = try bytes(string(row["probe_data_hex"], "SipHash ownership probe"))
        let expected = try string(row["expected_hash_hex"], "SipHash ownership expected")
        try require(hex(try algorithm.validatedHash(probe), width: expected.count) == expected, "SipHash configuration retained mutated caller key")
      case "entry-key-and-values-snapshots-are-detached":
        let source = try rows(row["source_entries"], "ownership source entries")
        let pairs = try source.map { entry -> (HashMapKey, Int?) in
          let keyID = try string(entry["key_id"], "ownership entry key")
          guard let key = keys[keyID] else { throw FixtureFailure(description: "missing ownership key \(keyID)") }
          return (key, try integer(entry["value"], "ownership entry value"))
        }
        let map = try HashMap<Int?>.fromEntries(pairs, capacity: 4, strategy: .linearProbing, hashAlgorithm: .fnv1a32)
        var entries = map.entries
        var keysSnapshot = map.keys
        var valuesSnapshot = map.values
        if case .bytes(var payload) = entries[0].key {
          payload[0] = 255
          entries[0].key = .bytes(payload)
        }
        entries[0].value = 999
        keysSnapshot[0] = .bytes([255])
        valuesSnapshot[0] = 999
        let expectedEntries = try rows(row["expected_entries"], "expected owned entries")
        try require(map.entries.count == expectedEntries.count, "entry getter mutated map count")
        for (actual, expected) in zip(map.entries, expectedEntries) {
          let keyID = try string(expected["key_id"], "expected owned key")
          try require(actual.key == keys[keyID], "entry getter mutated stored key")
          try require(actual.value == (try integer(expected["value"], "expected owned value")), "entry getter mutated stored value")
        }
      case "reference-values-are-shallow":
        let objectValue = try object(row["initial_object"], "reference object")
        let initial = try integer(objectValue["field"], "reference initial field")
        let mutated = try integer(object(row["mutation"], "reference mutation")["field"], "reference mutated field")
        let box = FixtureReferenceValue(field: initial)
        let empty = try HashMap<FixtureReferenceValue>(capacity: 2, strategy: .separateChaining, hashAlgorithm: .fnv1a32)
        let old = try empty.setting(box, for: keys["k0"]!)
        let new = try old.setting(box, for: keys["k1"]!)
        box.field = mutated
        let expected = try integer(row["expected_old_and_new_map_value_field"], "expected shallow field")
        try require(old.value(forKey: keys["k0"]!)?.field == expected, "old map did not retain shared reference value")
        try require(new.value(forKey: keys["k1"]!)?.field == expected, "new map did not retain shared reference value")
      default:
        throw FixtureFailure(description: "unknown shared ownership case \(id)")
      }
    }
  }

  private func expectedErrorID(_ error: HashMapError) -> String {
    switch error {
    case .invalidCapacity(_): return "invalid_capacity"
    case .invalidSipHashKeyLength(_): return "invalid_sip_hash_key_length"
    case .keyTooLong(_, _): return "key_too_long"
    case .capacityLimitExceeded: return "capacity_limit_exceeded"
    }
  }

  private func mapForErrorCase(
    _ row: [String: Any],
    keys: [String: HashMapKey]
  ) throws -> HashMap<Int?> {
    let configuration = try object(row["configuration"], "error-case configuration")
    var map = try HashMap<Int?>(
      capacity: integer(configuration["capacity"], "error-case capacity"),
      strategy: strategy(configuration["strategy"]),
      hashAlgorithm: hashConfiguration(object(configuration["hash_configuration"], "error-case hash configuration"))
    )
    for entry in try rows(row["initial_entries"], "error-case initial entries") {
      let keyID = try string(entry["key_id"], "initial error-case key id")
      guard let key = keys[keyID] else { throw FixtureFailure(description: "missing initial error-case key \(keyID)") }
      let value = try optionalInteger(entry["value"], "initial error-case value")
      map = try map.setting(value, for: key)
    }
    return map
  }

  func testSharedErrorCasesReachNativePublicAPI() throws {
    let document = try fixture()
    let rows = try rows(document["error_cases"], "error_cases")
    let keys = try keys(document)
    try require(rows.count == 5, "expected five shared error cases")
    for row in rows {
      let id = try string(row["id"], "error case id")
      if id == "oversized-key-lookup-and-removal-are-absent-no-ops" {
        try require(row["expected_state_unchanged"] as? Bool == true, "oversized lookup case must pin unchanged state")
        let original = try mapForErrorCase(row, keys: keys)
        let expectedState = try object(row["expected_state"], "oversized lookup expected state")
        try assertState(original, expected: expectedState, keys: keys)
        let recipe = try object(row["key_recipe"], "oversized lookup key recipe")
        let oversized = HashMapKey.bytes(Array(repeating: UInt8(try integer(recipe["byte"], "oversized lookup byte")), count: try integer(recipe["length"], "oversized lookup length")))
        let observation = try object(row["expected_observation"], "oversized lookup expectation")
        try require(observation["remove_result"] as? Bool == false, "oversized removal must expect remove_result=false")
        try require(!original.has(oversized), "oversized key unexpectedly present")
        try require(original.value(forKey: oversized) == nil, "oversized lookup unexpectedly returned a value")
        try assertObservation(original, key: oversized, expected: observation)
        let removed = original.removingValue(for: oversized)
        try require(removed.count == original.count && removed.capacity == original.capacity, "oversized removal changed state")
        try require(!removed.has(oversized), "oversized removal unexpectedly left the key present")
        try assertState(removed, expected: expectedState, keys: keys)
        try require(original.count == 1, "oversized no-op case must begin with a nonempty map")
        try require(row["expected_state_unchanged"] as? Bool == true, "oversized lookup case must pin unchanged state")
        continue
      }
      let expected = try object(row["expected_error"], "expected native error")
      let expectedID = try string(expected["id"], "expected native error id")
      var atomicOriginal: HashMap<Int?>?
      do {
        switch id {
        case "nonpositive-capacity-precedes-invalid-sip-key",
          "capacity-above-production-limit", "invalid-siphash-key-length":
          let configuration = try object(row["configuration"], "constructor error configuration")
          let algorithm = try hashConfiguration(object(configuration["hash_configuration"], "inline hash configuration"))
          _ = try HashMap<Int?>(
            capacity: integer(configuration["capacity"], "constructor error capacity"),
            strategy: strategy(configuration["strategy"]),
            hashAlgorithm: algorithm
          )
          throw FixtureFailure(description: "constructor error case unexpectedly succeeded: \(id)")
        case "oversized-key-setting-is-atomic":
          try require(row["original_map_unchanged"] as? Bool == true, "oversized set case must pin unchanged original map")
          let original = try mapForErrorCase(row, keys: keys)
          atomicOriginal = original
          try assertState(original, expected: object(row["before_state"], "oversized set before state"), keys: keys)
          let recipe = try object(row["key_recipe"], "oversized key recipe")
          let oversized = HashMapKey.bytes(Array(repeating: UInt8(try integer(recipe["byte"], "oversized key byte")), count: try integer(recipe["length"], "oversized key length")))
          _ = try original.setting(7, for: oversized)
          throw FixtureFailure(description: "oversized key setting unexpectedly succeeded")
        default:
          throw FixtureFailure(description: "unknown shared error case \(id)")
        }
      } catch let error as HashMapError {
        try require(expectedErrorID(error) == expectedID, "native error \(expectedErrorID(error)), expected \(expectedID) for \(id)")
        switch error {
        case .invalidCapacity(let capacity):
          try require(capacity == integer(object(row["configuration"], "capacity error config")["capacity"], "expected bad capacity"), "invalid capacity error field differs")
        case .invalidSipHashKeyLength(let actual):
          try require(actual == integer(object(expected["fields"], "error fields")["actual_length"], "expected invalid key length"), "invalid SipHash key length field differs")
        case .keyTooLong(let actual, let maximum):
          let fields = try object(expected["fields"], "oversized key error fields")
          try require(actual == integer(fields["actual"], "actual key size") && maximum == integer(fields["maximum"], "maximum key size"), "key-too-long error fields differ")
          guard let original = atomicOriginal else { throw FixtureFailure(description: "atomic update case did not preserve its original map") }
          try assertState(original, expected: object(row["before_state"], "oversized set before state"), keys: keys)
        case .capacityLimitExceeded:
          break
        }
      }
    }
  }

  private struct ModelOperation {
    let op: String
    let keyID: String
    let valueID: String?
  }

  private struct ModelEntry: Equatable {
    let keyID: String
    var valueID: String
  }

  private func baseNineDigits(rank: Int, length: Int) -> [Int] {
    guard length > 0 else { return [] }
    var value = rank
    var result = Array(repeating: 0, count: length)
    for index in stride(from: length - 1, through: 0, by: -1) {
      result[index] = value % 9
      value /= 9
    }
    return result
  }

  private func baseNineRank(_ digits: [Int]) -> Int {
    digits.reduce(0) { $0 * 9 + $1 }
  }

  private func enumeratedRankMatches(_ expectedRank: Int, length: Int, digits: [Int]) -> Bool {
    digits.count == length && digits.allSatisfy { (0..<9).contains($0) }
      && baseNineRank(digits) == expectedRank
  }

  private func intPower(_ base: Int, _ exponent: Int) -> Int {
    (0..<exponent).reduce(1) { value, _ in value * base }
  }

  private func associationListSymbol(_ model: [ModelEntry], keyOrder: [String]) throws -> Int {
    var symbol = 0
    var weight = 1
    for keyID in keyOrder {
      let valueID = model.first(where: { $0.keyID == keyID })?.valueID
      let digit: Int
      switch valueID {
      case nil: digit = 0
      case "v0": digit = 1
      case "v1": digit = 2
      default: throw FixtureFailure(description: "association-list model has unsupported value id")
      }
      symbol += digit * weight
      weight *= 3
    }
    try require(model.count == Set(model.map(\.keyID)).count, "association-list model contains duplicate keys")
    return symbol
  }

  private func digit(_ byte: UInt8, alphabet: [UInt8], label: String) throws -> Int {
    guard let index = alphabet.firstIndex(of: byte) else {
      throw FixtureFailure(description: "invalid \(label) stream character \(byte)")
    }
    return index
  }

  private func assertExhaustiveSnapshot(
    _ map: HashMap<Int?>,
    model: [ModelEntry],
    ordinal: Int,
    strategyName: String,
    recipe: [String: Any],
    keys: [String: HashMapKey],
    values: [String: Int?],
    semanticStreams: [String: String],
    physicalStreams: [String: String],
    capacityStreams: [String: String],
    physicalTable: [String: [String]],
    keyOrder: [String],
    semanticAlphabet: [UInt8],
    capacityAlphabet: [UInt8]
  ) throws {
    guard let semantic = semanticStreams[strategyName],
      let physical = physicalStreams[strategyName],
      let capacity = capacityStreams[strategyName]
    else { throw FixtureFailure(description: "missing history streams for \(strategyName)") }
    let semanticSymbol = try digit(semantic.utf8[semantic.utf8.index(semantic.utf8.startIndex, offsetBy: ordinal)], alphabet: semanticAlphabet, label: "semantic")
    let physicalID = String(UnicodeScalar(physical.utf8[physical.utf8.index(physical.utf8.startIndex, offsetBy: ordinal)]))
    let expectedCapacity = try digit(capacity.utf8[capacity.utf8.index(capacity.utf8.startIndex, offsetBy: ordinal)], alphabet: capacityAlphabet, label: "capacity")
    try require(map.capacity == expectedCapacity, "\(strategyName) capacity stream mismatch at ordinal \(ordinal)")

    var expectedState: [String: String] = [:]
    var remaining = semanticSymbol
    for keyID in keyOrder {
      let valueDigit = remaining % 3
      remaining /= 3
      if valueDigit == 1 { expectedState[keyID] = "v0" }
      if valueDigit == 2 { expectedState[keyID] = "v1" }
    }
    try require(remaining == 0, "semantic state symbol exceeds its three-key domain")
    let modelDictionary = Dictionary(uniqueKeysWithValues: model.map { ($0.keyID, $0.valueID) })
    try require(modelDictionary == expectedState, "independent association-list model differs from semantic stream at ordinal \(ordinal)")
    try require(try associationListSymbol(model, keyOrder: keyOrder) == semanticSymbol, "association-list encoding differs at ordinal \(ordinal)")
    try require(map.count == expectedState.count, "\(strategyName) semantic count differs at ordinal \(ordinal)")
    let order = physicalTable[physicalID]
    guard let order else { throw FixtureFailure(description: "physical order id \(physicalID) is absent") }
    try require(map.entries.count == order.count, "physical order stream count differs at ordinal \(ordinal)")
    for (entry, keyID) in zip(map.entries, order) {
      guard let expectedKey = keys[keyID] else { throw FixtureFailure(description: "physical table references missing key \(keyID)") }
      try require(entry.key == expectedKey, "physical order differs at ordinal \(ordinal), key \(keyID)")
      guard let valueID = expectedState[keyID] else {
        throw FixtureFailure(description: "physical table contains semantically absent key \(keyID)")
      }
      try require(entry.value == (try fixtureValue(id: valueID, values: values)), "physical entry value differs at ordinal \(ordinal)")
    }
    let ratio = Double(map.count) / Double(map.capacity)
    try require(abs(map.loadFactor - ratio) <= 1e-15, "load factor differs at ordinal \(ordinal)")
    _ = recipe
  }

  func testExhaustiveHistoriesMatchIndependentModelAndEveryPrefixVersion() throws {
    let document = try fixture()
    let keys = try keys(document)
    let values = try values(document)
    let recipe = try object(document["history_recipe"], "history_recipe")
    guard let historyLengths = recipe["lengths"] as? [Int] else {
      throw FixtureFailure(description: "history lengths must be integers")
    }
    let operations = try rows(recipe["operations"], "history operations").map { row -> ModelOperation in
      ModelOperation(
        op: try string(row["op"], "history op"),
        keyID: try string(row["key_id"], "history key id"),
        valueID: row["value_id"] as? String
      )
    }
    try require(historyLengths == [0, 1, 2, 3, 4, 5], "unexpected exhaustive history lengths")
    try require(operations.count == 9, "expected nine exhaustive operation choices")
    try require(try integer(recipe["history_count"], "history_count") == 66_430, "history count changed")
    let semanticAlphabet = Array(try string(object(recipe["state_encoding"], "state encoding")["alphabet"], "semantic alphabet").utf8)
    let capacityAlphabet = Array(try string(recipe["capacity_alphabet"], "capacity alphabet").utf8)
    let keyOrder = try (object(recipe["state_encoding"], "state encoding")["key_order"] as? [String]) ?? []
    try require(keyOrder == ["k0", "k1", "k2"], "unexpected state key ordering")
    let physicalRows = try rows(recipe["physical_order_table"], "physical_order_table")
    var physicalTable: [String: [String]] = [:]
    for row in physicalRows {
      let id = try string(row["id"], "physical order id")
      guard let ids = row["key_ids"] as? [String] else {
        throw FixtureFailure(description: "physical order key_ids must be strings")
      }
      physicalTable[id] = ids
    }
    try require(physicalTable.count == 16, "expected all 16 physical-order table entries")
    let semanticStreams = try object(recipe["expected_semantic_streams"], "semantic streams").mapValues { $0 as? String ?? "" }
    let physicalStreams = try object(recipe["expected_physical_order_streams"], "physical streams").mapValues { $0 as? String ?? "" }
    let capacityStreams = try object(recipe["expected_capacity_streams"], "capacity streams").mapValues { $0 as? String ?? "" }
    let expectedHistoryCount = try integer(recipe["history_count"], "history_count")
    try require(
      !enumeratedRankMatches(1, length: 2, digits: [0, 0]),
      "history enumeration negative control accepted a duplicate digit vector"
    )
    for strategyName in ["separateChaining", "linearProbing"] {
      for (label, streams) in [
        ("semantic", semanticStreams), ("physical", physicalStreams), ("capacity", capacityStreams),
      ] {
        guard let stream = streams[strategyName] else { throw FixtureFailure(description: "missing \(label) stream for \(strategyName)") }
        try require(stream.utf8.count == expectedHistoryCount, "\(strategyName) \(label) stream length differs")
      }
    }

    let configuration = try hashConfiguration(id: string(recipe["hash_configuration_id"], "history hash configuration"), configurations: hashConfigurations(document))
    let semanticBase = Array("0123456789abcdefghijklmnopq".utf8)
    try require(semanticAlphabet == semanticBase, "semantic alphabet differs from the frozen DT18 encoding")
    let capacityBase = capacityAlphabet
    var totalHistories = 0
    var totalPublicUpdates = 0
    var totalPrefixChecks = 0
    var totalRetainedVersionChecks = 0
    let offsets = historyLengths.map { length in
      (0..<length).reduce(0) { $0 + intPower(9, $1) }
    }

    for strategyName in ["separateChaining", "linearProbing"] {
      let collisionStrategy: CollisionStrategy = strategyName == "separateChaining" ? .separateChaining : .linearProbing
      for length in historyLengths {
        let historyCount = intPower(9, length)
        for rank in 0..<historyCount {
          let digits = baseNineDigits(rank: rank, length: length)
          try require(
            enumeratedRankMatches(rank, length: length, digits: digits),
            "base-nine enumerator did not bijectively encode rank \(rank) at length \(length)"
          )
          let traceOrdinal = offsets[length] + baseNineRank(digits)
          try require(traceOrdinal == offsets[length] + rank, "history ordinal differs from enumeration rank")
          var map = try HashMap<Int?>(
            capacity: integer(recipe["initial_capacity"], "history initial capacity"),
            strategy: collisionStrategy,
            hashAlgorithm: configuration
          )
          var model: [ModelEntry] = []
          var mapVersions = [map]
          var modelVersions = [model]
          try assertExhaustiveSnapshot(
            map, model: model, ordinal: offsets[0] + 0,
            strategyName: strategyName, recipe: recipe, keys: keys, values: values,
            semanticStreams: semanticStreams, physicalStreams: physicalStreams,
            capacityStreams: capacityStreams, physicalTable: physicalTable,
            keyOrder: keyOrder, semanticAlphabet: semanticBase, capacityAlphabet: capacityBase
          )
          totalPrefixChecks += 1
          for prefixLength in 0..<length {
            let operation = operations[digits[prefixLength]]
            guard let operationKey = keys[operation.keyID] else {
              throw FixtureFailure(description: "history references missing key \(operation.keyID)")
            }
            switch operation.op {
            case "set":
              guard let valueID = operation.valueID else { throw FixtureFailure(description: "history set lacks value id") }
              map = try map.setting(fixtureValue(id: valueID, values: values), for: operationKey)
              if let existing = model.firstIndex(where: { $0.keyID == operation.keyID }) {
                model[existing].valueID = valueID
              } else {
                model.append(ModelEntry(keyID: operation.keyID, valueID: valueID))
              }
              totalPublicUpdates += 1
            case "remove":
              map = map.removingValue(for: operationKey)
              model.removeAll(where: { $0.keyID == operation.keyID })
              totalPublicUpdates += 1
            default:
              throw FixtureFailure(description: "unknown exhaustive operation \(operation.op)")
            }
            mapVersions.append(map)
            modelVersions.append(model)
            let prefixDigits = Array(digits.prefix(prefixLength + 1))
            let ordinal = offsets[prefixLength + 1] + baseNineRank(prefixDigits)
            try assertExhaustiveSnapshot(
              map, model: model, ordinal: ordinal,
              strategyName: strategyName, recipe: recipe, keys: keys, values: values,
              semanticStreams: semanticStreams, physicalStreams: physicalStreams,
              capacityStreams: capacityStreams, physicalTable: physicalTable,
              keyOrder: keyOrder, semanticAlphabet: semanticBase, capacityAlphabet: capacityBase
            )
            totalPrefixChecks += 1
            for priorPrefix in 0..<(prefixLength + 1) {
              let priorDigits = Array(digits.prefix(priorPrefix))
              let priorRank = baseNineRank(priorDigits)
              try require(
                enumeratedRankMatches(priorRank, length: priorPrefix, digits: priorDigits),
                "prior-prefix digits are not a valid base-nine rank"
              )
              let priorOrdinal = offsets[priorPrefix] + priorRank
              try assertExhaustiveSnapshot(
                mapVersions[priorPrefix], model: modelVersions[priorPrefix], ordinal: priorOrdinal,
                strategyName: strategyName, recipe: recipe, keys: keys, values: values,
                semanticStreams: semanticStreams, physicalStreams: physicalStreams,
                capacityStreams: capacityStreams, physicalTable: physicalTable,
                keyOrder: keyOrder, semanticAlphabet: semanticBase, capacityAlphabet: capacityBase
              )
              totalRetainedVersionChecks += 1
            }
          }
          totalHistories += 1
        }
      }
    }
    let expectedPerStrategy = historyLengths.reduce(0) { $0 + intPower(9, $1) }
    let expectedChecksPerStrategy = historyLengths.reduce(0) { $0 + intPower(9, $1) * ($1 + 1) }
    let expectedUpdatesPerStrategy = historyLengths.reduce(0) { $0 + intPower(9, $1) * $1 }
    let expectedPriorChecksPerStrategy = historyLengths.reduce(0) {
      $0 + intPower(9, $1) * ($1 * ($1 + 1) / 2)
    }
    try require(totalHistories == 2 * expectedPerStrategy, "exhaustive history enumeration count differs")
    try require(totalHistories == 132_860, "unexpected two-strategy history count")
    try require(totalPrefixChecks == 2 * expectedChecksPerStrategy, "prefix check count differs")
    try require(totalRetainedVersionChecks == 2 * expectedPriorChecksPerStrategy, "retained-version recheck count differs")
    try require(totalPublicUpdates == 2 * expectedUpdatesPerStrategy, "public update count differs")
    try require(totalPrefixChecks == 780_554, "unexpected exhaustive prefix check count")
    try require(totalRetainedVersionChecks == 1_911_942, "unexpected prior-version recheck count")
    try require(totalPublicUpdates == 647_694, "unexpected exhaustive public update count")
    try require(totalPrefixChecks + totalRetainedVersionChecks == 2_692_496, "unexpected exhaustive state check count")
    print(
      "DT18 Swift public history receipt: strategies=2 histories=\(totalHistories) " +
        "public_updates=\(totalPublicUpdates) prefix_checks=\(totalPrefixChecks) " +
        "retained_version_rechecks=\(totalRetainedVersionChecks) total_state_checks=\(totalPrefixChecks + totalRetainedVersionChecks)"
    )
  }
}
