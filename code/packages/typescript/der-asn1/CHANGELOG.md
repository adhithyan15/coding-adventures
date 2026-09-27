# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added brand-checked composition helpers backed by private element, decoder,
  and cursor state so downstream decoders do not trust overridable accessors.

## [0.1.0] - 2026-09-20

### Added

- Added the bounded typed DER decoder, canonical primitive values, exact
  constructed wrappers, shared work budgets, and stable payload-free errors.
- Added complete consumption of the 122-case neutral DER ASN.1 suite and all
  46 referenced DER-TLV framing cases.
