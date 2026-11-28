# HWEB02 — Go profile for the early Web

This profile ports HWEB00's `http-0.9` and `html-tags-1992` contracts to Go.
The package is in-memory and dependency-free; it has no network, filesystem, or
process behavior. Its exported values are intentionally plain Go values so
ports and callers can compare trees without a renderer.

## Public API

`EncodeHTTP09Request(address string) ([]byte, error)` validates a non-empty
printable-ASCII address and emits `GET `, the address, and CRLF.
`ParseHTTP09Request(wire []byte) (string, error)` accepts exactly one request
line with CRLF or LF. `DecodeHTTP09Response(body []byte) (string, error)`
decodes the complete ASCII response and preserves line endings.

`ParseHTMLTags1992(source string) ([]Node, error)` returns an ordered tree.
`Node` is an interface implemented by `TextNode` and `Element`; `Element` has
`Tag string`, `Attributes map[string]*string`, and `Children []Node`. A nil
attribute value represents a valueless recognized attribute. Parse failures
are reserved for impossible internal states; malformed profile markup follows
HWEB00's deterministic recovery rules.

The implementation manually scans bytes/runes rather than using regular
expressions. Open-element lookup is indexed by tag and tree validation/building
uses iterative work where depth can be attacker-controlled. It preserves the
profile's explicit deviations from HTML5: unmatched end tags are ignored,
unknown tags are skipped, list/glossary omission is local to its container,
and `plaintext` consumes the remainder.

Go strings are byte sequences: HTTP addresses are accepted only when every
byte is printable ASCII, while `ParseHTMLTags1992` preserves source UTF-8
bytes in Go strings and folds only ASCII tag/attribute letters. Callers that
need Unicode scalar validation must perform it before calling this historical
profile; the parser makes no normalization claim.

## Conformance and bounds

Go tests replay every `http-0.9`, `html-tags-1992`, and combined exchange in
`code/specs/fixtures/historical-web-v1/vectors.json`, including negative
vectors. The Go lane does not add a second interpretation of those vectors;
its only intended implementation deviation is Go's `(value, error)` result
shape and pointers for nullable attributes.
