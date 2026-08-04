# Changelog

All notable changes to the `http1` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — HTTP/1.1 request & response head parser** (CCPP02 port
  campaign, bucket A / pure-ISO). The C port of the Rust `http1` crate: turns the
  bytes of an HTTP/1 head (start line + headers + blank line) into a structured
  head and tells the caller where the body starts and how to frame it. A pure-ISO
  crate (no OS), so it rides the `iso-harness` (links nothing, strict-conformance
  flags on).
  - **API.** `http1_parse_request_head` / `http1_parse_response_head` (bytes →
    `Http1Parsed*Head`); `http1_parsed_request_free` / `http1_parsed_response_free`;
    `http1_request_summary` / `http1_response_summary` (redacted, log-safe). An
    `http1_status` enum covers every Rust `Http1ParseError` variant plus NOMEM /
    INVALID.
  - **Composes `c/http-core`.** The head vocabulary (`HttpVersion`, `HttpHeader`,
    `HttpRequestHead`, `HttpResponseHead`, `HttpBodyKind`) comes from http-core,
    whose source `run.sh` compiles in — nothing is linked. `BUILD` declares
    `deps=c/iso-harness c/http-core`.
  - **Ownership.** http-core's heads borrow their strings, so each parsed head
    OWNS the backing storage (method/target/reason + every header name/value,
    copied out of the input) and points its `head` fields into it; the `*_free`
    releases all of it. Allocating paths return `HTTP1_ERR_NOMEM` and unwind.
  - **Body framing.** Request: chunked / positive Content-Length / none. Response:
    1xx/204/304 force none; else chunked / Content-Length (zero → none) / until
    EOF. `HttpBodyKind` is tag-only, so the length is carried in `body_length`
    (meaningful iff `HTTP_BODY_CONTENT_LENGTH`).
  - **Faithfulness.** Leading blank lines skipped; `\n`-terminated lines with an
    optional trailing `\r` (CRLF or bare-LF heads); request start line is exactly
    three tokens, response is version/status/reason (joined with single spaces);
    header split at the first `:` with all-whitespace name trim (non-empty) and
    space/tab value trim; Content-Length integer / status u16 / version `HTTP/x.y`
    validated; head lines UTF-8-validated.
  - **Safety (untrusted input).** Every read is bounds-checked and every size is
    `size_t`-overflow guarded. An adversarial security review confirmed no
    out-of-bounds access, overflow, or error-path leak.
  - **Test (`tests/http1_test.c`).** The Rust tests (simple request,
    Content-Length, response + reason, until-EOF, body-less status override,
    LF-only + duplicate headers, invalid header / Content-Length, the redacted
    summaries) plus chunked, incomplete-head, bad-start-line/version/status, and
    the NULL / zeroed-free paths. 59 checks, verified under gcc + clang with
    `-pedantic-errors`, clean under ASan+UBSan, 0 leaks.
