// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

import 'cbor_types.dart';

/// Redaction placeholder used in log-safe diagnostics to prevent sensitive data leakage.
const String redactedPlaceholder = '<redacted>';

/// Static diagnostic message literals for all 14 closed CBR01 error identifiers.
///
/// Kept separate from serialization logic to facilitate localization and string audit.
/// All diagnostics are strictly payload-blind, prefixing every error with `canonical-cbor: `
/// and omitting user keys, byte buffers, stream offsets, or collection lengths.
const String msgUnexpectedEof = 'canonical-cbor: unexpected end of input';
const String msgTrailingBytes =
    'canonical-cbor: trailing bytes after decoded item';
const String msgReserved = 'canonical-cbor: reserved additional-info value';
const String msgIndefinite = 'canonical-cbor: indefinite item rejected';
const String msgNonMinimalInteger =
    'canonical-cbor: argument is not in smallest form';
const String msgInvalidUtf8 = 'canonical-cbor: text is not valid UTF-8';
const String msgNonCanonicalMapOrder =
    'canonical-cbor: map key order is not canonical';
const String msgUnsupportedSimple = 'canonical-cbor: unsupported simple value';
const String msgFloatNotSupported = 'canonical-cbor: floats are not supported';
const String msgTooDeep = 'canonical-cbor: decoded nesting is too deep';
const String msgLengthTooLarge = 'canonical-cbor: declared length is too large';
const String msgDuplicateMapKey = 'canonical-cbor: duplicate canonical map key';
const String msgEncodeTooDeep = 'canonical-cbor: encoded nesting is too deep';
const String msgEncodeTooLarge = 'canonical-cbor: encoded item is too large';

/// Diagnostic message table for all closed CBR01 error identifiers.
const Map<CborErrorId, String> errorMessages = {
  CborErrorId.unexpectedEof: msgUnexpectedEof,
  CborErrorId.trailingBytes: msgTrailingBytes,
  CborErrorId.reserved: msgReserved,
  CborErrorId.indefinite: msgIndefinite,
  CborErrorId.nonMinimalInteger: msgNonMinimalInteger,
  CborErrorId.invalidUtf8: msgInvalidUtf8,
  CborErrorId.nonCanonicalMapOrder: msgNonCanonicalMapOrder,
  CborErrorId.unsupportedSimple: msgUnsupportedSimple,
  CborErrorId.floatNotSupported: msgFloatNotSupported,
  CborErrorId.tooDeep: msgTooDeep,
  CborErrorId.lengthTooLarge: msgLengthTooLarge,
  CborErrorId.duplicateMapKey: msgDuplicateMapKey,
  CborErrorId.encodeTooDeep: msgEncodeTooDeep,
  CborErrorId.encodeTooLarge: msgEncodeTooLarge,
};

/// Looks up the static payload-blind diagnostic string for the given [id].
///
/// Uses an exhaustive switch to ensure compiler-enforced handling of all enum values.
String errorMessageFor(CborErrorId id) {
  switch (id) {
    case CborErrorId.unexpectedEof:
      return msgUnexpectedEof;
    case CborErrorId.trailingBytes:
      return msgTrailingBytes;
    case CborErrorId.reserved:
      return msgReserved;
    case CborErrorId.indefinite:
      return msgIndefinite;
    case CborErrorId.nonMinimalInteger:
      return msgNonMinimalInteger;
    case CborErrorId.invalidUtf8:
      return msgInvalidUtf8;
    case CborErrorId.nonCanonicalMapOrder:
      return msgNonCanonicalMapOrder;
    case CborErrorId.unsupportedSimple:
      return msgUnsupportedSimple;
    case CborErrorId.floatNotSupported:
      return msgFloatNotSupported;
    case CborErrorId.tooDeep:
      return msgTooDeep;
    case CborErrorId.lengthTooLarge:
      return msgLengthTooLarge;
    case CborErrorId.duplicateMapKey:
      return msgDuplicateMapKey;
    case CborErrorId.encodeTooDeep:
      return msgEncodeTooDeep;
    case CborErrorId.encodeTooLarge:
      return msgEncodeTooLarge;
  }
}
