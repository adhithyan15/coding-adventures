// ============================================================================
// Codec.swift -- RFC 8949 Canonical CBOR Encoder & Decoder (CBR01)
// ============================================================================

// Comparison outcome for map keys
private enum KeyOrdering {
  case less
  case equal
  case greater
}

private func compareWireKeys(_ a: [UInt8], _ b: [UInt8]) -> KeyOrdering {
  if a.count < b.count { return .less }
  if a.count > b.count { return .greater }
  for i in 0..<a.count {
    if a[i] < b[i] { return .less }
    if a[i] > b[i] { return .greater }
  }
  return .equal
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Encodes a CBOR value into a canonical RFC 8949 byte representation.
/// Enforces max encoded size (1,048,576 bytes) and max nesting depth (128).
public func encodeChecked(_ value: CborValue) throws -> [UInt8] {
  var buffer: [UInt8] = []
  buffer.reserveCapacity(Constants.defaultBufferCapacity)
  try encodeIntoChecked(value, buffer: &buffer)
  return buffer
}

/// Result-returning wrapper for encodeChecked.
public func encodeCheckedResult(_ value: CborValue) -> Result<[UInt8], CborError> {
  do {
    return .success(try encodeChecked(value))
  } catch let error as CborError {
    return .failure(error)
  } catch {
    return .failure(CborError(.unknown))
  }
}

/// Encodes a CBOR value appending into an existing buffer with atomic rollback on failure.
/// The item itself is strictly budgeted to Constants.maxEncodedBytes.
public func encodeIntoChecked(_ value: CborValue, buffer: inout [UInt8]) throws {
  let initialCount = buffer.count
  var remaining = Constants.maxEncodedBytes
  do {
    try encodeValue(value, buffer: &buffer, depth: Constants.rootDepth, remaining: &remaining)
  } catch {
    if buffer.count > initialCount {
      buffer.removeSubrange(initialCount...)
    }
    throw error
  }
}

/// Decodes exactly one canonical CBOR data item from wire bytes.
public func decode(_ bytes: [UInt8]) throws -> CborValue {
  guard !bytes.isEmpty else {
    throw CborError(.unexpectedEof)
  }
  var offset = 0
  let value = try decodeValue(bytes, offset: &offset, depth: Constants.rootDepth)
  if offset < bytes.count {
    throw CborError(.trailingBytes)
  }
  return value
}

/// Result-returning wrapper for decode.
public func decodeResult(_ bytes: [UInt8]) -> Result<CborValue, CborError> {
  do {
    return .success(try decode(bytes))
  } catch let error as CborError {
    return .failure(error)
  } catch {
    return .failure(CborError(.unknown))
  }
}

/// Decodes top-level map entries with value byte spans, or validates non-map values.
public func decodeMapSpans(_ bytes: [UInt8]) throws -> DecodeMapSpansResult {
  guard !bytes.isEmpty else {
    throw CborError(.unexpectedEof)
  }
  let firstByte = bytes[0]
  let major = (firstByte & Constants.majorTypeMask) >> Constants.majorTypeShift
  if major == Constants.majorMap {
    var offset = 0
    let entries = try decodeSpannedMap(bytes, offset: &offset)
    if offset < bytes.count {
      throw CborError(.trailingBytes)
    }
    return .map(entries)
  } else {
    _ = try decode(bytes)
    return .nonMap
  }
}

/// Result-returning wrapper for decodeMapSpans.
public func decodeMapSpansResult(_ bytes: [UInt8]) -> Result<DecodeMapSpansResult, CborError> {
  do {
    return .success(try decodeMapSpans(bytes))
  } catch let error as CborError {
    return .failure(error)
  } catch {
    return .failure(CborError(.unknown))
  }
}

// ============================================================================
// Encoding Internals
// ============================================================================

private func appendBytes(_ slice: [UInt8], buffer: inout [UInt8], remaining: inout Int) throws {
  let count = slice.count
  guard remaining >= count else {
    throw CborError(.encodeTooLarge)
  }
  let (_, overflow) = buffer.count.addingReportingOverflow(count)
  guard !overflow else {
    throw CborError(.encodeTooLarge)
  }
  remaining -= count
  buffer.append(contentsOf: slice)
}

private func appendByte(_ byte: UInt8, buffer: inout [UInt8], remaining: inout Int) throws {
  guard remaining >= 1 else {
    throw CborError(.encodeTooLarge)
  }
  let (_, overflow) = buffer.count.addingReportingOverflow(1)
  guard !overflow else {
    throw CborError(.encodeTooLarge)
  }
  remaining -= 1
  buffer.append(byte)
}

private func encodeHeader(major: UInt8, arg: UInt64, buffer: inout [UInt8], remaining: inout Int)
  throws
{
  let majorShifted = major << Constants.majorTypeShift
  if arg <= UInt64(Constants.aiDirectMax) {
    try appendByte(majorShifted | UInt8(arg), buffer: &buffer, remaining: &remaining)
  } else if arg <= Constants.maxUint8 {
    try appendBytes(
      [majorShifted | Constants.aiOneByte, UInt8(arg)], buffer: &buffer, remaining: &remaining)
  } else if arg <= Constants.maxUint16 {
    try appendBytes(
      [
        majorShifted | Constants.aiTwoBytes,
        UInt8((arg >> Constants.shift8) & Constants.byteMask),
        UInt8(arg & Constants.byteMask),
      ], buffer: &buffer, remaining: &remaining)
  } else if arg <= Constants.maxUint32 {
    try appendBytes(
      [
        majorShifted | Constants.aiFourBytes,
        UInt8((arg >> Constants.shift24) & Constants.byteMask),
        UInt8((arg >> Constants.shift16) & Constants.byteMask),
        UInt8((arg >> Constants.shift8) & Constants.byteMask),
        UInt8(arg & Constants.byteMask),
      ], buffer: &buffer, remaining: &remaining)
  } else {
    try appendBytes(
      [
        majorShifted | Constants.aiEightBytes,
        UInt8((arg >> Constants.shift56) & Constants.byteMask),
        UInt8((arg >> Constants.shift48) & Constants.byteMask),
        UInt8((arg >> Constants.shift40) & Constants.byteMask),
        UInt8((arg >> Constants.shift32) & Constants.byteMask),
        UInt8((arg >> Constants.shift24) & Constants.byteMask),
        UInt8((arg >> Constants.shift16) & Constants.byteMask),
        UInt8((arg >> Constants.shift8) & Constants.byteMask),
        UInt8(arg & Constants.byteMask),
      ], buffer: &buffer, remaining: &remaining)
  }
}

private func encodeValue(
  _ value: CborValue, buffer: inout [UInt8], depth: Int, remaining: inout Int
) throws {
  if depth > Constants.maxNestingDepth {
    throw CborError(.encodeTooDeep)
  }

  switch value {
  case .unsigned(let u):
    try encodeHeader(major: Constants.majorUnsigned, arg: u, buffer: &buffer, remaining: &remaining)

  case .negative(let n):
    try encodeHeader(major: Constants.majorNegative, arg: n, buffer: &buffer, remaining: &remaining)

  case .bytes(let b):
    let count = UInt64(b.count)
    guard UInt64(remaining) >= count else {
      throw CborError(.encodeTooLarge)
    }
    try encodeHeader(
      major: Constants.majorBytes, arg: count, buffer: &buffer, remaining: &remaining)
    try appendBytes(b, buffer: &buffer, remaining: &remaining)

  case .text(let s):
    // O(1) UTF-8 count check on native Swift String without temporary allocation
    let utf8Count = s.utf8.count
    guard remaining >= utf8Count else {
      throw CborError(.encodeTooLarge)
    }
    try encodeHeader(
      major: Constants.majorText, arg: UInt64(utf8Count), buffer: &buffer, remaining: &remaining)
    guard remaining >= utf8Count else {
      throw CborError(.encodeTooLarge)
    }
    let (_, overflow) = buffer.count.addingReportingOverflow(utf8Count)
    guard !overflow else {
      throw CborError(.encodeTooLarge)
    }
    remaining -= utf8Count
    buffer.append(contentsOf: s.utf8)

  case .array(let items):
    let itemCount = UInt64(items.count)
    // Pre-flight minimal size budget
    let (minPayload, overflow) = itemCount.multipliedReportingOverflow(
      by: Constants.minBytesPerArrayItem)
    if overflow || UInt64(remaining) < minPayload {
      throw CborError(.encodeTooLarge)
    }
    try encodeHeader(
      major: Constants.majorArray, arg: itemCount, buffer: &buffer, remaining: &remaining)
    for item in items {
      try encodeValue(item, buffer: &buffer, depth: depth + 1, remaining: &remaining)
    }

  case .map(let entries):
    let entryCount = UInt64(entries.count)
    let (minPayload, overflow) = entryCount.multipliedReportingOverflow(
      by: Constants.minBytesPerMapEntry)
    if overflow || UInt64(remaining) < minPayload {
      throw CborError(.encodeTooLarge)
    }

    // Threaded key encoding and canonical sorting
    struct PreparedEntry {
      let keyWire: [UInt8]
      let value: CborValue
    }

    var prepared: [PreparedEntry] = []
    prepared.reserveCapacity(entries.count)

    for entry in entries {
      var keyBuffer: [UInt8] = []
      keyBuffer.reserveCapacity(Constants.keyBufferCapacity)
      try encodeValue(entry.key, buffer: &keyBuffer, depth: depth + 1, remaining: &remaining)
      prepared.append(PreparedEntry(keyWire: keyBuffer, value: entry.value))
    }

    // Sort entries by RFC 8949 Section 4.2.3 length-first canonical ordering
    prepared.sort { a, b in
      compareWireKeys(a.keyWire, b.keyWire) == .less
    }

    // Check for duplicate keys in sorted array
    if prepared.count > 1 {
      for i in 0..<(prepared.count - 1) {
        if compareWireKeys(prepared[i].keyWire, prepared[i + 1].keyWire) == .equal {
          throw CborError(.duplicateMapKey)
        }
      }
    }

    try encodeHeader(
      major: Constants.majorMap, arg: entryCount, buffer: &buffer, remaining: &remaining)
    for p in prepared {
      // keyWire bytes were already deducted from remaining during key encoding
      let (_, overflow) = buffer.count.addingReportingOverflow(p.keyWire.count)
      guard !overflow else {
        throw CborError(.encodeTooLarge)
      }
      buffer.append(contentsOf: p.keyWire)
      try encodeValue(p.value, buffer: &buffer, depth: depth + 1, remaining: &remaining)
    }

  case .tag(let tagVal, let inner):
    try encodeHeader(major: Constants.majorTag, arg: tagVal, buffer: &buffer, remaining: &remaining)
    try encodeValue(inner, buffer: &buffer, depth: depth + 1, remaining: &remaining)

  case .boolean(let b):
    try appendByte(
      b ? Constants.wireByteTrue : Constants.wireByteFalse, buffer: &buffer, remaining: &remaining)

  case .null:
    try appendByte(Constants.wireByteNull, buffer: &buffer, remaining: &remaining)
  }
}

// ============================================================================
// Decoding Internals
// ============================================================================

private func readArgument(ai: UInt8, bytes: [UInt8], offset: inout Int) throws -> UInt64 {
  if ai <= Constants.aiDirectMax {
    return UInt64(ai)
  } else if ai == Constants.aiOneByte {
    guard offset < bytes.count else { throw CborError(.unexpectedEof) }
    let val = UInt64(bytes[offset])
    offset += 1
    if val <= UInt64(Constants.aiDirectMax) {
      throw CborError(.nonMinimalInteger)
    }
    return val
  } else if ai == Constants.aiTwoBytes {
    guard offset + Constants.argBytes2 <= bytes.count else { throw CborError(.unexpectedEof) }
    let b0 = UInt64(bytes[offset])
    let b1 = UInt64(bytes[offset + 1])
    offset += Constants.argBytes2
    let val = (b0 << Constants.shift8) | b1
    if val <= Constants.maxUint8 {
      throw CborError(.nonMinimalInteger)
    }
    return val
  } else if ai == Constants.aiFourBytes {
    guard offset + Constants.argBytes4 <= bytes.count else { throw CborError(.unexpectedEof) }
    let b0 = UInt64(bytes[offset])
    let b1 = UInt64(bytes[offset + 1])
    let b2 = UInt64(bytes[offset + 2])
    let b3 = UInt64(bytes[offset + 3])
    offset += Constants.argBytes4
    let val =
      (b0 << Constants.shift24) | (b1 << Constants.shift16)
      | (b2 << Constants.shift8) | b3
    if val <= Constants.maxUint16 {
      throw CborError(.nonMinimalInteger)
    }
    return val
  } else if ai == Constants.aiEightBytes {
    guard offset + Constants.argBytes8 <= bytes.count else { throw CborError(.unexpectedEof) }
    let b0 = UInt64(bytes[offset])
    let b1 = UInt64(bytes[offset + 1])
    let b2 = UInt64(bytes[offset + 2])
    let b3 = UInt64(bytes[offset + 3])
    let b4 = UInt64(bytes[offset + 4])
    let b5 = UInt64(bytes[offset + 5])
    let b6 = UInt64(bytes[offset + 6])
    let b7 = UInt64(bytes[offset + 7])
    offset += Constants.argBytes8
    let val =
      (b0 << Constants.shift56) | (b1 << Constants.shift48)
      | (b2 << Constants.shift40) | (b3 << Constants.shift32)
      | (b4 << Constants.shift24) | (b5 << Constants.shift16)
      | (b6 << Constants.shift8) | b7
    if val <= Constants.maxUint32 {
      throw CborError(.nonMinimalInteger)
    }
    return val
  } else if ai >= Constants.aiReserved28 && ai <= Constants.aiReserved30 {
    throw CborError(.reserved)
  } else if ai == Constants.aiIndefinite {
    throw CborError(.indefinite)
  } else {
    throw CborError(.reserved)
  }
}

private func decodeMajor7(ai: UInt8, bytes: [UInt8], offset: inout Int) throws -> CborValue {
  if ai == Constants.simpleFalse {
    return .boolean(false)
  } else if ai == Constants.simpleTrue {
    return .boolean(true)
  } else if ai == Constants.simpleNull {
    return .null
  } else if ai == Constants.aiFloat16 || ai == Constants.aiFloat32 || ai == Constants.aiFloat64 {
    throw CborError(.floatNotSupported)
  } else if ai >= Constants.aiReserved28 && ai <= Constants.aiReserved30 {
    throw CborError(.reserved)
  } else if ai == Constants.aiIndefinite {
    throw CborError(.indefinite)
  } else if ai == Constants.aiOneByte {
    guard offset < bytes.count else { throw CborError(.unexpectedEof) }
    let val = bytes[offset]
    offset += 1
    if val <= Constants.aiDirectMax {
      throw CborError(.nonMinimalInteger)
    }
    throw CborError(.unsupportedSimple)
  } else {
    throw CborError(.unsupportedSimple)
  }
}

private func decodeMapEntries<T>(
  bytes: [UInt8],
  offset: inout Int,
  count: Int,
  depth: Int,
  makeEntry: (_ key: CborValue, _ val: CborValue, _ valStart: Int, _ valEnd: Int) -> T
) throws -> [T] {
  var entries: [T] = []
  entries.reserveCapacity(min(count, Constants.collectionInitialCapacity))
  var prevKeyWire: [UInt8]? = nil

  for _ in 0..<count {
    let keyStart = offset
    let key = try decodeValue(bytes, offset: &offset, depth: depth)
    let keyEnd = offset
    let keyWire = Array(bytes[keyStart..<keyEnd])

    if let prev = prevKeyWire {
      let ordering = compareWireKeys(prev, keyWire)
      if ordering != .less {
        throw CborError(.nonCanonicalMapOrder)
      }
    }
    prevKeyWire = keyWire

    let valStart = offset
    let val = try decodeValue(bytes, offset: &offset, depth: depth)
    let valEnd = offset

    entries.append(makeEntry(key, val, valStart, valEnd))
  }
  return entries
}

private func decodeValue(_ bytes: [UInt8], offset: inout Int, depth: Int) throws -> CborValue {
  if depth > Constants.maxNestingDepth {
    throw CborError(.tooDeep)
  }
  guard offset < bytes.count else {
    throw CborError(.unexpectedEof)
  }

  let b0 = bytes[offset]
  offset += 1

  let major = (b0 & Constants.majorTypeMask) >> Constants.majorTypeShift
  let ai = b0 & Constants.additionalInfoMask

  if major == Constants.majorSimple {
    return try decodeMajor7(ai: ai, bytes: bytes, offset: &offset)
  }

  let arg = try readArgument(ai: ai, bytes: bytes, offset: &offset)
  let remainingBytes = UInt64(bytes.count - offset)

  switch major {
  case Constants.majorUnsigned:
    return .unsigned(arg)

  case Constants.majorNegative:
    return .negative(arg)

  case Constants.majorBytes:
    if arg > remainingBytes {
      throw CborError(.lengthTooLarge)
    }
    let len = Int(arg)
    let payload = Array(bytes[offset..<(offset + len)])
    offset += len
    return .bytes(payload)

  case Constants.majorText:
    if arg > remainingBytes {
      throw CborError(.lengthTooLarge)
    }
    let len = Int(arg)
    let slice = bytes[offset..<(offset + len)]
    offset += len

    var utf8Codec = UTF8()
    var iter = slice.makeIterator()
    var scalars: [UnicodeScalar] = []
    scalars.reserveCapacity(len)

    decodingLoop: while true {
      switch utf8Codec.decode(&iter) {
      case .scalarValue(let scalar):
        scalars.append(scalar)
      case .emptyInput:
        break decodingLoop
      case .error:
        throw CborError(.invalidUtf8)
      }
    }

    var resultString = ""
    resultString.unicodeScalars.append(contentsOf: scalars)
    return .text(resultString)

  case Constants.majorArray:
    let (minBytes, overflow) = arg.multipliedReportingOverflow(by: Constants.minBytesPerArrayItem)
    if overflow || minBytes > remainingBytes {
      throw CborError(.lengthTooLarge)
    }
    let count = Int(arg)
    var items: [CborValue] = []
    items.reserveCapacity(min(count, Constants.collectionInitialCapacity))
    for _ in 0..<count {
      let item = try decodeValue(bytes, offset: &offset, depth: depth + 1)
      items.append(item)
    }
    return .array(items)

  case Constants.majorMap:
    let (minBytes, overflow) = arg.multipliedReportingOverflow(by: Constants.minBytesPerMapEntry)
    if overflow || minBytes > remainingBytes {
      throw CborError(.lengthTooLarge)
    }
    let count = Int(arg)
    let entries = try decodeMapEntries(
      bytes: bytes, offset: &offset, count: count, depth: depth + 1
    ) { key, val, _, _ in
      CborMapEntry(key: key, value: val)
    }
    return .map(entries)

  case Constants.majorTag:
    let inner = try decodeValue(bytes, offset: &offset, depth: depth + 1)
    return .tag(arg, inner)

  default:
    throw CborError(.unsupportedSimple)
  }
}

private func decodeSpannedMap(_ bytes: [UInt8], offset: inout Int) throws -> [SpannedMapEntry] {
  guard offset < bytes.count else {
    throw CborError(.unexpectedEof)
  }

  let b0 = bytes[offset]
  offset += 1

  let ai = b0 & Constants.additionalInfoMask
  let arg = try readArgument(ai: ai, bytes: bytes, offset: &offset)
  let remainingBytes = UInt64(bytes.count - offset)

  let (minBytes, overflow) = arg.multipliedReportingOverflow(by: Constants.minBytesPerMapEntry)
  if overflow || minBytes > remainingBytes {
    throw CborError(.lengthTooLarge)
  }

  let count = Int(arg)
  return try decodeMapEntries(
    bytes: bytes, offset: &offset, count: count, depth: Constants.initialMapDepth
  ) { key, val, valStart, valEnd in
    SpannedMapEntry(
      key: key,
      value: val,
      valueSpan: Span(uncheckedStart: UInt64(valStart), finish: UInt64(valEnd))
    )
  }
}
