# Historical Web profiles

This dependency-free Rust crate implements two early-Web profiles from
[HWEB00](../../../specs/HWEB00-historical-web.md): CERN's 1991 HTTP/0.9 request
and response rules, and the W3C archive's November 3, 1992 *HTML Tags*
vocabulary. It accepts caller-owned in-memory bytes and strings. It does not
open sockets, fetch URLs, read files, or render markup.

```rust
use coding_adventures_historical_web::{
    decode_http09_response, encode_http09_request, parse_html_tags_1992,
    Http09Address, HtmlNodeKind,
};

let address = Http09Address::new("/index.html")?;
assert_eq!(encode_http09_request(&address), b"GET /index.html\r\n");
assert_eq!(decode_http09_response(b"Hello\r\n")?, "Hello\r\n");

let document = parse_html_tags_1992("<UL><LI>One<LI>Two</UL>")?;
let list = document.node(document.roots[0]).unwrap();
assert!(matches!(list.kind, HtmlNodeKind::Element { ref tag, .. } if tag == "ul"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

The flat `HtmlDocument` arena represents children with integer indices, so a
deep document can be built and dropped without recursive ownership. The parser
uses an explicit open-element stack with tag-indexed positions, coalesces text
in place, and applies only ASCII case folding. Its documented recovery behavior
is the archived profile's small deterministic model, not modern browser error
recovery.

## Three different tree models

The 1992 *HTML Tags* archive describes an SGML-era vocabulary. It documents
omitted list and glossary end tags; for example, `<UL><LI>One<LI>Two</UL>`
contains two list items. XML 1.0 is a later, case-sensitive, explicit-end-tag
markup language, so that exact source is not well-formed XML. Compare the
repository's [XML parser](../../../specs/XML01-xml-parser.md). Venture's current
browser parser follows the WHATWG HTML tokenizer and tree construction model,
including its much larger modern error-recovery rules. This crate deliberately
preserves only the archived 1992 teaching profile.

The package replays `code/specs/fixtures/historical-web-v1/vectors.json`, shared
with the Python reference and Go port. See HWEB03 for the public API and
evidence boundaries.
