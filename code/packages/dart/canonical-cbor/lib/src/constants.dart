// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

/// Constants, thresholds, masks, and limits for deterministic CBOR (RFC 8949 §4.2.3).
///
/// In accordance with Clean Code principles, zero literal magic numbers are permitted
/// in codec logic. Every wire byte, opcode, mask, shift, and capacity is explicitly
/// named here with intention-revealing identifiers.

/// Maximum allowed container/tag recursion depth per CBR01 specification.
/// Root item is at depth 0. Items requiring depth 129 are rejected with
/// [CborErrorId.tooDeep] (decode) or [CborErrorId.encodeTooDeep] (encode).
const int maxNestingDepth = 128;

/// Maximum serialized bytes for a single canonical CBOR item (1 MiB / 1,048,576 bytes).
/// Checked encoders reject items exceeding this limit with [CborErrorId.encodeTooLarge].
/// Note: No digit separator (_) is used to maintain compatibility across Dart sdk: ^3.0.0.
const int maxEncodedBytes = 1048576;

/// Initial recursion depth of the root CBOR item.
const int rootDepth = 0;

/// Bit shift to place the 3-bit major type into bits 7..5 of a CBOR header byte.
const int majorTypeShift = 5;

/// Bit mask to extract the lower 5 bits (additional information) of a header byte.
const int additionalInfoMask = 0x1F;

/// RFC 8949 Section 3.1 Major Types:
const int majorUnsigned = 0;
const int majorNegative = 1;
const int majorBytes = 2;
const int majorText = 3;
const int majorArray = 4;
const int majorMap = 5;
const int majorTag = 6;
const int majorSimpleOrFloat = 7;

/// Additional-info representation codes for argument lengths:
const int infoInlineMax = 23;
const int info1ByteArg = 24;
const int info2ByteArg = 25;
const int info4ByteArg = 26;
const int info8ByteArg = 27;
const int infoReserved28 = 28;
const int infoReserved29 = 29;
const int infoReserved30 = 30;
const int infoIndefinite = 31;

/// Multi-byte header argument widths in bytes:
const int argWidth1Byte = 1;
const int argWidth2Bytes = 2;
const int argWidth4Bytes = 4;
const int argWidth8Bytes = 8;

/// Simple value codes for major type 7 (RFC 8949 Section 3.3):
const int simpleFalse = 20;
const int simpleTrue = 21;
const int simpleNull = 22;
const int simpleFloat16 = 25;
const int simpleFloat32 = 26;
const int simpleFloat64 = 27;

/// Exact on-wire byte values for closed canonical simple values:
const int wireByteFalse =
    (majorSimpleOrFloat << majorTypeShift) | simpleFalse; // 0xF4
const int wireByteTrue =
    (majorSimpleOrFloat << majorTypeShift) | simpleTrue; // 0xF5
const int wireByteNull =
    (majorSimpleOrFloat << majorTypeShift) | simpleNull; // 0xF6

/// Bit widths and byte masks for big-endian integer serialization:
const int uint64BitWidth = 64;
const int bitsPerByte = 8;
const int byteMask = 0xFF;

/// Maximum unsigned integer boundaries for CBOR argument width selection:
final BigInt maxUint8 = BigInt.from(0xFF);
final BigInt maxUint16 = BigInt.from(0xFFFF);
final BigInt maxUint32 = BigInt.from(0xFFFFFFFF);
final BigInt maxUint64 = (BigInt.one << uint64BitWidth) - BigInt.one;

/// Minimum wire bytes required per element to prevent hostile pre-allocation DoS:
const int minBytesPerElement = 1;
const int minBytesPerMapEntry = 2;

/// UTF-16 surrogate code point boundaries:
const int surrogateHighStart = 0xD800;
const int surrogateHighEnd = 0xDBFF;
const int surrogateLowStart = 0xDC00;
const int surrogateLowEnd = 0xDFFF;
