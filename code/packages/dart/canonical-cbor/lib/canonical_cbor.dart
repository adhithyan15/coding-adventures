// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

/// Canonical CBOR codec (RFC 8949 §4.2.3 length-first map key ordering).
///
/// This package provides a deterministic, zero-dependency CBOR codec conforming
/// to the language-neutral CBR01 specification. It implements the canonical
/// profile required by COSE (RFC 9052), FIDO2 / CTAP2, WebAuthn-PRF, and
/// cryptographic Vault audit records.
///
/// # Security & Determinism Invariants
///
/// 1. Strict Bijection:
///    Every logical value has exactly one legal wire representation. The encoder
///    always emits the minimal-width encoding, and the decoder rejects expanded
///    integers, indefinite lengths, and floating-point encodings.
///
/// 2. Length-First Map Key Ordering:
///    Map keys are sorted by encoded-key byte length ascending, then by unsigned
///    bytewise lexicographical comparison. Duplicate keys are rejected.
///
/// 3. Resource Bounds:
///    - Maximum container nesting depth is capped at 128 levels ([maxNestingDepth]).
///    - Maximum encoded item size is bounded to 1 MiB ([maxEncodedBytes]).
///    - Decoders verify declared lengths against remaining input before allocating
///      memory to prevent Out-Of-Memory (OOM) denial-of-service attacks.
///
/// 4. Payload-Blind Diagnostics:
///    All errors return static string descriptions prefixed with `canonical-cbor: `
///    with zero reflection of user payload bytes or secret tokens.
///
/// 5. Atomic Rollback:
///    [encodeIntoChecked] leaves destination buffers completely untouched if
///    encoding fails.
library canonical_cbor;

export 'src/cbor_types.dart';
export 'src/codec.dart'
    show
        decode,
        decodeMapSpans,
        encode,
        encodeChecked,
        encodeInto,
        encodeIntoChecked;
export 'src/constants.dart';
export 'src/messages.dart';
