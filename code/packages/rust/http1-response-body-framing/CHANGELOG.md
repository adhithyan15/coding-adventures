# Changelog

All notable changes to `http1-response-body-framing` will be documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added allocation-free response body-framing classification over borrowed,
  syntax-validated headers.
- Added strict canonical `Content-Length`, duplicate-field, transfer-coding,
  bodyless-framing status, HEAD, CONNECT, body-size, and close-delimited policy.
- Added payload-free errors and explicit retained maximums for close-delimited
  consumption.

### Security

- Transfer coding is rejected until a separately reviewed decoder exists.
- The package performs no I/O, body consumption, credential interpretation or
  release, provider action, audit, trace emission, or external effect.
