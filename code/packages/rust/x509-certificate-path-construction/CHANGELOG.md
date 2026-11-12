# Changelog

All notable changes to `x509-certificate-path-construction` will be documented
in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-11

### Added

- Added non-copyable adjacent-link projection that derives the exact child and
  selected issuer at one retained path depth without rerunning selection.
- Added explicit target-child and candidate-child accessors that preserve their
  original decoded-certificate lifetimes.
- Added policy-backed path construction that applies one validated
  `X509NameValuePolicy` at every non-terminal selection step.
- Added distinct non-copyable success evidence retaining both the constructed
  path and exact validated Name policy, plus stable payload-free nested errors.

### Changed

- Shared the bounded traversal core between generic and policy-backed entry
  points so limit, cycle, terminal, ordering, and depth semantics remain
  identical.

### Security

- Out-of-range depths expose no partial evidence, projected endpoints remain
  bound to the borrowed construction, and redacted debug output contains no
  certificate data.
- Candidate and path bounds reject before either terminal or Name-policy
  evaluation; the first Name-comparison failure aborts; complete accepted pools
  are still scanned to reject ambiguous matches.
- Generic callback construction evidence cannot be converted into the distinct
  policy-backed evidence type.
- Adjacency evidence grants no signature, certificate-policy, path-validation,
  trust, revocation, TLS, HTTP, OAuth, or network authority.

## [0.1.0] - 2026-10-23

### Added

- Added bounded target-to-terminal chain construction over one caller-owned
  candidate pool and the existing unique issuer selector.
- Added non-copyable evidence retaining the exact target, candidate pool, and
  ordered selected indices.
- Added stable payload-free errors for invalid limits, issuer selection,
  certificate cycles, and path-depth exhaustion.

### Security

- Candidate and path bounds reject before policy callbacks, every accepted
  certificate receives one terminal-policy decision, and complete certificate
  DER equality rejects cycles through either repeated indices or re-decodes.
- Construction grants no trust, signature, certificate-role, constraint,
  validity, identity, revocation, TLS, HTTP, OAuth, or network authority.
