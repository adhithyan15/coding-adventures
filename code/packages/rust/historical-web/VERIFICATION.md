# Historical Web Rust verification receipt

The Rust integration suite in `tests/assurance.rs` replays every shared
`historical-web-v1/vectors.json` HTTP, HTML, and combined exchange entry. It
checks exact wire bytes and response strings, compares recursively rendered
arena trees against fixture JSON, and fails closed if a top-level fixture
category is missing. Tree integrity is checked independently with an iterative
walk for valid indices, reachability, single ownership, acyclicity, lowercase
tags, and unique recognized attributes.

The exhaustive HTTP framing oracle enumerates all octet payloads of lengths
zero through two with LF and CRLF endings: **131,586 pairs**. It decides
validity using its own prefix/framing/printable-ASCII rules and compares the
exact decoded address or rejection. A deterministic property loop checks 1,000
printable-address encode/parse round trips and response identity over 257
ASCII bodies. Regressions exercise quoted and valueless first-wins anchor
attributes, UTF-8 preservation, one-pass entity decoding, SGML-era omitted
`LI` ends, malformed unmatched ends, and 20,000-level hostile nesting followed
by arena destruction. A focused raw-text regression places a multibyte UTF-8
scalar at an unaligned search-window endpoint in both `xmp` and `listing`, then
checks exact body text and tokenization of the following elements.
Typed request and response errors are asserted from invalid public inputs, and
the document arena accessor is checked for both an existing root and an
out-of-range index.

The suite also invokes the repository's VER06 Rust fuzz engine with fixed
seeds and seed `0x485745423033`, 2,048 mutations per target, and a 256-byte
input cap. Every mutation in the mixed wire/markup campaign is checked against
the independent HTTP acceptance oracle, even when it has an invalid method
prefix. Every valid UTF-8 mutation is also parsed as markup; parser errors fail
the campaign because this profile recovers malformed markup. Invalid UTF-8
bytes remain in the HTTP domain and are not replacement-decoded into a
different markup input. A separate response target checks ASCII identity, and
the markup path checks the independent arena invariant. Reports are asserted
free of crashes, timeouts, and OOM outcomes.
These deterministic in-test campaigns are reproducible receipts, not claims of
native coverage or unbounded parser safety.

The HTTP admission SAT model is already maintained in
`code/packages/python/historical-web/verification/sat_http09.py` and delegates
to the repository Go CDCL solver. It proves three bounded UNSAT cases (address
lengths zero through two) with a SAT mutant control, then replays the same
131,586 public-parser cases. The Rust suite does not fork a second solver or
claim a Rust-specific SAT proof: its independent exhaustive oracle checks the
Rust export directly. SAT evidence is bounded and is not a DRAT certificate or
a source-level proof.

Run from this package directory with `cargo test -p
coding_adventures_historical_web`. No external network, filesystem, process,
or modern browser behavior is part of the crate contract.
