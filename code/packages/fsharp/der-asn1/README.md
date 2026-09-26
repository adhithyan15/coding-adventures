# CodingAdventures.DerAsn1.FSharp

Bounded typed ASN.1 DER value decoding for .NET. The package composes the
native F# `der-tlv` framing layer with typed BOOLEAN, INTEGER, BIT STRING,
OCTET STRING, NULL, IA5String, OBJECT IDENTIFIER, SEQUENCE, SET, and explicit
context-tag decoding.

Validated values are unforgeable and defensively immutable. Decoder and cursor
operations share explicit depth and element budgets; failed cursor reads are
transactional; errors expose stable identifiers and local offsets without
including hostile input payloads.

The portable conformance suite consumes all 122 language-neutral ASN.1 cases,
including direct verification of the 46 delegated DER-TLV cases and all 22
typed error identifiers.

Run `BUILD` or `BUILD_windows` for the full fixture suite and the 95% production
line-coverage gate.
