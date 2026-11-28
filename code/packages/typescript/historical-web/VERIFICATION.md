# Historical Web TypeScript verification receipt

`tests/assurance.test.ts` replays every shared HTTP, HTML, and combined
exchange fixture. Fixture loading fails closed when a category is absent, and
hex decoding rejects malformed or odd-length encodings. The request encoder
is checked against canonical CRLF even when a shared parser vector uses LF;
fixture bytes are separately checked and passed unchanged to the parser.
Response strings and the complete nested HTML trees are compared exactly.

The independent framing oracle exhaustively enumerates every octet payload of
length zero through two with LF and CRLF endings: **131,586 pairs**. It checks
acceptance and the exact address returned by the production parser. Deterministic
properties exercise 1,000 printable-address round trips and 257 ASCII response
bodies. Boundary regressions cover the historical empty `P` marker followed by
sibling text, omitted SGML list-item endings, first-wins valueless anchor
attributes, one-pass entities, CRLF normalization inside a quoted attribute,
UTF-16 lone-surrogate preservation, and multibyte UTF-8 adjacent to raw-text
search-window endpoints.

A 12,000-level nested document and 20,000 malformed tag-like prefixes exercise
iterative parser and arena handling. An iterative independent graph walk checks
valid child references, reachability, unique ownership, acyclicity, and
lowercase element tags. A runtime boundary regression checks direct JavaScript
construction and structural forgery against the encoder, because Node's type
stripping does not enforce TypeScript `private` declarations. Invalid
HTTP/response results are checked by their tagged error kinds, including an
untyped non-string HTML call returning `invalid-source`.

The suite invokes the repository VER06 TypeScript fuzz engine (via its built
`@coding-adventures/test-bootstrap/fuzz` export) with a fixed seed, 2,048
mutations, and a 256-byte cap. Every mutation is checked against the independent
HTTP oracle, including invalid method prefixes. Every valid UTF-8 mutation is
also parsed as markup, and a parser error counts as a crash; invalid UTF-8 is
not replacement-decoded into a different source string. Fuzz outcomes also
check the arena graph invariant. These are deterministic bounded campaign
receipts, not a claim of native coverage or unbounded safety.

The bounded HTTP SAT model is shared with the Python profile and delegates to
the repository Go CDCL solver. This TypeScript lane does not duplicate the SAT
solver; the independent exhaustive runtime replay above checks the TypeScript
export directly. SAT evidence remains bounded, not a DRAT certificate or a
source-level proof.
