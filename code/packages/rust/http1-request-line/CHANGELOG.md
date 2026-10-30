# Changelog

All notable changes to `http1-request-line` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added bounded HTTP token method and origin-form target validation.
- Added exact allocation-free HTTP/1.0 and HTTP/1.1 request-line encoding into
  caller-owned storage.
- Added stable payload-free errors and redacted Debug output.

### Security

- Header and body framing, destination authorization, credentials, audit,
  trace, TLS, HTTP transport, and OAuth provider effects remain separate.
