// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

import 'cbor_types.dart';

/// Redaction placeholder used in log-safe diagnostics to prevent sensitive data leakage.
const String redactedPlaceholder = '<redacted>';

/// Diagnostic message table for all closed CBR01 error identifiers.
///
/// Kept separate from serialization logic to facilitate localization and string audit.
/// All diagnostics are strictly payload-blind, prefixing every error with `canonical-cbor: `
/// and omitting user keys, byte buffers, stream offsets, or collection lengths.
const Map<CborErrorId, String> errorMessages = {
  CborErrorId.unexpectedEof: 'canonical-cbor: unexpected end of input',
  CborErrorId.trailingBytes:
      'canonical-cbor: trailing bytes after decoded item',
  CborErrorId.reserved: 'canonical-cbor: reserved additional-info value',
  CborErrorId.indefinite: 'canonical-cbor: indefinite item rejected',
  CborErrorId.nonMinimalInteger:
      'canonical-cbor: argument is not in smallest form',
  CborErrorId.invalidUtf8: 'canonical-cbor: text is not valid UTF-8',
  CborErrorId.nonCanonicalMapOrder:
      'canonical-cbor: map key order is not canonical',
  CborErrorId.unsupportedSimple: 'canonical-cbor: unsupported simple value',
  CborErrorId.floatNotSupported: 'canonical-cbor: floats are not supported',
  CborErrorId.tooDeep: 'canonical-cbor: decoded nesting is too deep',
  CborErrorId.lengthTooLarge: 'canonical-cbor: declared length is too large',
  CborErrorId.duplicateMapKey: 'canonical-cbor: duplicate canonical map key',
  CborErrorId.encodeTooDeep: 'canonical-cbor: encoded nesting is too deep',
  CborErrorId.encodeTooLarge: 'canonical-cbor: encoded item is too large',
};

/// Looks up the static payload-blind diagnostic string for the given [id].
String errorMessageFor(CborErrorId id) =>
    errorMessages[id] ?? 'canonical-cbor: unknown error';
