// WasmLeb128.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - LEB128: Little Endian Base 128 Encoding
// ============================================================================
//
// LEB128 is a variable-length integer encoding used throughout the WebAssembly
// binary format. It encodes integers using 7 bits per byte, with the high bit
// (bit 7) acting as a "continuation" flag:
//
//   bit 7 = 1  →  more bytes follow
//   bit 7 = 0  →  this is the last byte
//
// ============================================================================
// Why LEB128?
// ============================================================================
//
// Fixed-width encodings waste space for small values. A 32-bit integer always
// takes 4 bytes, even for the value 0. LEB128 uses only as many bytes as
// needed:
//
//   Value     Fixed (32-bit)    LEB128
//   -----     --------------    ------
//   0         00 00 00 00       00
//   1         01 00 00 00       01
//   127       7F 00 00 00       7F
//   128       80 00 00 00       80 01
//   16384     00 40 00 00       80 80 01
//
// WebAssembly uses LEB128 everywhere: type indices, function indices, memory
// sizes, instruction immediates, and more. A typical .wasm file is 20-30%
// smaller than it would be with fixed-width encoding.
//
// ============================================================================
// Unsigned LEB128 Encoding Algorithm
// ============================================================================
//
// To encode an unsigned integer:
//   1. Take the lowest 7 bits of the value
//   2. Shift the value right by 7
//   3. If more bits remain, set the continuation bit (bit 7) and repeat
//   4. Otherwise, emit the final byte without continuation bit
//
// Example: encode 624485 (0x98765)
//
//   Binary: 0000 1001 1000 0111 0110 0101
//
//   Step 1: lowest 7 bits = 110 0101 = 0x65, remaining = 0x4C3B >> 7
//           More bits remain → emit 0xE5 (0x65 | 0x80)
//   Step 2: lowest 7 bits = 000 1110 = 0x0E, remaining = 0x0262 >> 7
//           More bits remain → emit 0x8E (0x0E | 0x80)
//   Step 3: lowest 7 bits = 010 0110 = 0x26, remaining = 0
//           No more bits → emit 0x26
//
//   Result: [0xE5, 0x8E, 0x26]
//
// ============================================================================
// Signed LEB128 Encoding Algorithm
// ============================================================================
//
// Signed LEB128 uses two's complement representation. The sign bit of the
// last byte is the sign of the entire value:
//
//   - For positive values: if bit 6 of the last byte is set, emit an extra
//     0x00 byte to indicate the value is positive
//   - For negative values: if bit 6 of the last byte is clear, emit an extra
//     0x7F byte to indicate the value is negative
//
// Example: encode -123456
//
//   Two's complement (64-bit): ...1111 1111 1110 0001 1101 1100 0000 0000
//   (but we only care about significant bits)
//
//   Step 1: lowest 7 bits = 100 0000 = 0x40, shift right (arithmetic)
//           More significant bits differ → emit 0xC0 (0x40 | 0x80)
//   Step 2: lowest 7 bits = 011 1000 = 0x38, shift right
//           → emit 0xB8 (0x38 | 0x80)
//   Step 3: lowest 7 bits = 000 0111 = 0x07, shift right
//           → emit 0x87 (0x07 | 0x80)
//   Step 4: lowest 7 bits = 111 1000 = 0x78, remaining = all 1s
//           Sign bit (bit 6) is set, matching the negative sign → emit 0x78
//
//   Result: [0xC0, 0xB8, 0x87, 0x78]
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - Error Types
// ============================================================================

/// Errors that can occur during LEB128 encoding or decoding.
///
/// LEB128 is a simple format, but several things can go wrong:
/// - The input bytes may run out before the value is complete
/// - The encoded value may be too large for the target integer type
/// - The encoded value may use more bytes than the format allows
public enum LEB128Error: Error, Equatable {
    /// The input ended before a complete LEB128 value was found.
    /// This means we saw continuation bits (bit 7 = 1) but ran out of bytes
    /// before seeing a terminating byte (bit 7 = 0).
    case unexpectedEnd

    /// The encoded value would overflow the target integer type.
    /// For example, a 6-byte unsigned LEB128 value cannot fit in a UInt32
    /// (which needs at most 5 bytes: ceil(32/7) = 5).
    case overflow

    /// The encoded value uses more bytes than allowed for the target type.
    /// WebAssembly limits unsigned 32-bit LEB128 to 5 bytes and unsigned
    /// 64-bit LEB128 to 10 bytes.
    case tooManyBytes
}

// ============================================================================
// MARK: - LEB128 Decoder
// ============================================================================

/// A stateful decoder that reads LEB128-encoded values from a byte buffer.
///
/// The decoder maintains a position cursor that advances as values are read,
/// making it easy to decode multiple consecutive values from a byte stream
/// (which is exactly what a WebAssembly parser needs to do).
///
/// Usage:
///
///     var decoder = LEB128Decoder(data: [0x80, 0x01, 0x05])
///     let first = try decoder.decodeUnsigned32()   // 128
///     let second = try decoder.decodeUnsigned32()   // 5
///     print(decoder.position)                        // 3
///
public struct LEB128Decoder {
    /// The byte buffer we are reading from.
    public let data: [UInt8]

    /// Current read position in the buffer. Advances after each decode call.
    public var position: Int

    /// Creates a decoder that reads from the given byte array.
    ///
    /// - Parameter data: The bytes containing LEB128-encoded values.
    public init(data: [UInt8]) {
        self.data = data
        self.position = 0
    }

    /// Creates a decoder starting at a specific offset.
    ///
    /// - Parameters:
    ///   - data: The bytes containing LEB128-encoded values.
    ///   - offset: The starting position in the byte array.
    public init(data: [UInt8], offset: Int) {
        self.data = data
        self.position = offset
    }

    /// Returns true if there are more bytes available to read.
    public var hasMore: Bool {
        return position < data.count
    }

    /// Returns the number of bytes remaining in the buffer.
    public var remaining: Int {
        return data.count - position
    }

    // ========================================================================
    // MARK: Unsigned Decoding
    // ========================================================================

    /// Decodes an unsigned 32-bit LEB128 value.
    ///
    /// WebAssembly limits unsigned 32-bit LEB128 to at most 5 bytes.
    /// The 5th byte may only use the lowest 4 bits (since 4*7 = 28 bits
    /// from the first 4 bytes, plus 4 more bits = 32).
    ///
    /// - Returns: The decoded UInt32 value.
    /// - Throws: `LEB128Error.unexpectedEnd` if the buffer ends mid-value,
    ///           `LEB128Error.overflow` if the value exceeds UInt32 range,
    ///           `LEB128Error.tooManyBytes` if more than 5 bytes are used.
    public mutating func decodeUnsigned32() throws -> UInt32 {
        var result: UInt32 = 0
        var shift: UInt32 = 0
        var bytesRead = 0

        while true {
            guard position < data.count else {
                throw LEB128Error.unexpectedEnd
            }

            let byte = data[position]
            position += 1
            bytesRead += 1

            // A UInt32 needs at most 5 bytes (ceil(32/7) = 5).
            // The 5th byte can only use the low 4 bits.
            if bytesRead > 5 {
                throw LEB128Error.tooManyBytes
            }

            // Extract the 7 payload bits and shift them into position.
            let payload = UInt32(byte & 0x7F)

            // Check for overflow: on the 5th byte, only the low 4 bits
            // may be set (bits 4-6 must be zero).
            if bytesRead == 5 && payload > 0x0F {
                throw LEB128Error.overflow
            }

            result |= payload << shift
            shift += 7

            // If the continuation bit is clear, we're done.
            if byte & 0x80 == 0 {
                return result
            }
        }
    }

    /// Decodes an unsigned 64-bit LEB128 value.
    ///
    /// WebAssembly limits unsigned 64-bit LEB128 to at most 10 bytes.
    /// The 10th byte may only use the lowest bit.
    ///
    /// - Returns: The decoded UInt64 value.
    /// - Throws: `LEB128Error` on malformed input.
    public mutating func decodeUnsigned64() throws -> UInt64 {
        var result: UInt64 = 0
        var shift: UInt64 = 0
        var bytesRead = 0

        while true {
            guard position < data.count else {
                throw LEB128Error.unexpectedEnd
            }

            let byte = data[position]
            position += 1
            bytesRead += 1

            if bytesRead > 10 {
                throw LEB128Error.tooManyBytes
            }

            let payload = UInt64(byte & 0x7F)

            // On the 10th byte, only the lowest bit may be set.
            if bytesRead == 10 && payload > 0x01 {
                throw LEB128Error.overflow
            }

            result |= payload << shift
            shift += 7

            if byte & 0x80 == 0 {
                return result
            }
        }
    }

    // ========================================================================
    // MARK: Signed Decoding
    // ========================================================================

    /// Decodes a signed 32-bit LEB128 value.
    ///
    /// Signed LEB128 uses two's complement. The sign is determined by bit 6
    /// of the final byte: if it's set, the value is negative and we need to
    /// sign-extend the result.
    ///
    /// - Returns: The decoded Int32 value.
    /// - Throws: `LEB128Error` on malformed input.
    public mutating func decodeSigned32() throws -> Int32 {
        var result: Int32 = 0
        var shift: Int32 = 0
        var bytesRead = 0
        var byte: UInt8 = 0

        while true {
            guard position < data.count else {
                throw LEB128Error.unexpectedEnd
            }

            byte = data[position]
            position += 1
            bytesRead += 1

            if bytesRead > 5 {
                throw LEB128Error.tooManyBytes
            }

            let payload = Int32(byte & 0x7F)
            result |= payload << shift
            shift += 7

            if byte & 0x80 == 0 {
                break
            }
        }

        // Sign extension: if the sign bit (bit 6) of the last byte is set
        // and we haven't filled all 32 bits, extend the sign.
        //
        // Why bit 6? Because each byte contributes 7 bits (0-6), and the
        // topmost of those (bit 6) is the sign bit in two's complement.
        let signBitSet = (byte & 0x40) != 0
        if signBitSet && shift < 32 {
            // Fill all remaining upper bits with 1s.
            // The expression (Int32(-1) << shift) creates a mask like:
            //   shift=7:  0xFFFFFF80
            //   shift=14: 0xFFFFC000
            //   shift=21: 0xFFE00000
            //   shift=28: 0xF0000000
            result |= Int32(-1) << shift
        }

        return result
    }

    /// Decodes a signed 64-bit LEB128 value.
    ///
    /// - Returns: The decoded Int64 value.
    /// - Throws: `LEB128Error` on malformed input.
    public mutating func decodeSigned64() throws -> Int64 {
        var result: Int64 = 0
        var shift: Int64 = 0
        var bytesRead = 0
        var byte: UInt8 = 0

        while true {
            guard position < data.count else {
                throw LEB128Error.unexpectedEnd
            }

            byte = data[position]
            position += 1
            bytesRead += 1

            if bytesRead > 10 {
                throw LEB128Error.tooManyBytes
            }

            let payload = Int64(byte & 0x7F)
            result |= payload << shift
            shift += 7

            if byte & 0x80 == 0 {
                break
            }
        }

        // Sign extension for 64-bit values.
        let signBitSet = (byte & 0x40) != 0
        if signBitSet && shift < 64 {
            result |= Int64(-1) << shift
        }

        return result
    }

    // ========================================================================
    // MARK: Raw Byte Reading
    // ========================================================================

    /// Reads a single byte from the buffer, advancing the position.
    public mutating func readByte() throws -> UInt8 {
        guard position < data.count else {
            throw LEB128Error.unexpectedEnd
        }
        let byte = data[position]
        position += 1
        return byte
    }

    /// Reads `count` bytes from the buffer, advancing the position.
    public mutating func readBytes(_ count: Int) throws -> [UInt8] {
        guard position + count <= data.count else {
            throw LEB128Error.unexpectedEnd
        }
        let bytes = Array(data[position..<(position + count)])
        position += count
        return bytes
    }
}

// ============================================================================
// MARK: - LEB128 Encoder
// ============================================================================

/// Encodes integer values into LEB128 format.
///
/// The encoder is implemented as a collection of static methods since encoding
/// is stateless — it simply transforms a value into bytes.
///
/// Usage:
///
///     let bytes = LEB128Encoder.encodeUnsigned32(624485)
///     // bytes == [0xE5, 0x8E, 0x26]
///
public enum LEB128Encoder {

    /// Encodes an unsigned 32-bit integer as LEB128.
    ///
    /// - Parameter value: The UInt32 value to encode.
    /// - Returns: The LEB128-encoded bytes (1-5 bytes).
    public static func encodeUnsigned32(_ value: UInt32) -> [UInt8] {
        var result: [UInt8] = []
        var remaining = value

        while true {
            // Take the lowest 7 bits.
            var byte = UInt8(remaining & 0x7F)
            remaining >>= 7

            if remaining != 0 {
                // More bytes to come — set the continuation bit.
                byte |= 0x80
            }

            result.append(byte)

            if remaining == 0 {
                break
            }
        }

        return result
    }

    /// Encodes an unsigned 64-bit integer as LEB128.
    ///
    /// - Parameter value: The UInt64 value to encode.
    /// - Returns: The LEB128-encoded bytes (1-10 bytes).
    public static func encodeUnsigned64(_ value: UInt64) -> [UInt8] {
        var result: [UInt8] = []
        var remaining = value

        while true {
            var byte = UInt8(remaining & 0x7F)
            remaining >>= 7

            if remaining != 0 {
                byte |= 0x80
            }

            result.append(byte)

            if remaining == 0 {
                break
            }
        }

        return result
    }

    /// Encodes a signed 32-bit integer as signed LEB128.
    ///
    /// The encoding uses two's complement and emits bytes until the remaining
    /// value is fully represented. The termination condition checks that:
    /// - For positive values: remaining bits are all 0 and the sign bit
    ///   of the last byte is 0
    /// - For negative values: remaining bits are all 1 and the sign bit
    ///   of the last byte is 1
    ///
    /// - Parameter value: The Int32 value to encode.
    /// - Returns: The signed LEB128-encoded bytes.
    public static func encodeSigned32(_ value: Int32) -> [UInt8] {
        var result: [UInt8] = []
        var remaining = value
        var more = true

        while more {
            // Take the lowest 7 bits. We use bitwise AND with Int32 first,
            // then convert to UInt8 to handle negative values correctly.
            let byte = UInt8(remaining & 0x7F)
            remaining >>= 7  // Arithmetic shift: fills with sign bit

            // Check if we're done:
            // - If positive (or zero) and top bit of payload is 0 → done
            // - If negative and top bit of payload is 1 → done
            if (remaining == 0 && (byte & 0x40) == 0) ||
               (remaining == -1 && (byte & 0x40) != 0) {
                more = false
                result.append(byte)
            } else {
                result.append(byte | 0x80)
            }
        }

        return result
    }

    /// Encodes a signed 64-bit integer as signed LEB128.
    ///
    /// - Parameter value: The Int64 value to encode.
    /// - Returns: The signed LEB128-encoded bytes.
    public static func encodeSigned64(_ value: Int64) -> [UInt8] {
        var result: [UInt8] = []
        var remaining = value
        var more = true

        while more {
            let byte = UInt8(remaining & 0x7F)
            remaining >>= 7

            if (remaining == 0 && (byte & 0x40) == 0) ||
               (remaining == -1 && (byte & 0x40) != 0) {
                more = false
                result.append(byte)
            } else {
                result.append(byte | 0x80)
            }
        }

        return result
    }
}

// ============================================================================
// MARK: - Convenience Free Functions
// ============================================================================

/// Convenience function to decode an unsigned 32-bit LEB128 value from bytes.
///
/// - Parameters:
///   - data: The byte array to decode from.
///   - offset: Starting position (default 0).
/// - Returns: A tuple of (decoded value, number of bytes consumed).
public func decodeLEB128Unsigned32(_ data: [UInt8], offset: Int = 0) throws -> (value: UInt32, bytesRead: Int) {
    var decoder = LEB128Decoder(data: data, offset: offset)
    let value = try decoder.decodeUnsigned32()
    return (value, decoder.position - offset)
}

/// Convenience function to decode a signed 32-bit LEB128 value from bytes.
public func decodeLEB128Signed32(_ data: [UInt8], offset: Int = 0) throws -> (value: Int32, bytesRead: Int) {
    var decoder = LEB128Decoder(data: data, offset: offset)
    let value = try decoder.decodeSigned32()
    return (value, decoder.position - offset)
}

/// Convenience function to decode an unsigned 64-bit LEB128 value from bytes.
public func decodeLEB128Unsigned64(_ data: [UInt8], offset: Int = 0) throws -> (value: UInt64, bytesRead: Int) {
    var decoder = LEB128Decoder(data: data, offset: offset)
    let value = try decoder.decodeUnsigned64()
    return (value, decoder.position - offset)
}

/// Convenience function to decode a signed 64-bit LEB128 value from bytes.
public func decodeLEB128Signed64(_ data: [UInt8], offset: Int = 0) throws -> (value: Int64, bytesRead: Int) {
    var decoder = LEB128Decoder(data: data, offset: offset)
    let value = try decoder.decodeSigned64()
    return (value, decoder.position - offset)
}
