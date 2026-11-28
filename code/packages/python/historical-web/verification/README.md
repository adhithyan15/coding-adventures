# Bounded SAT evidence

`sat_http09.py` is an isolated proof harness for the narrow HTTP/0.9 request
contract. For address lengths 0, 1, and 2 it encodes each payload octet as
eight Boolean bits, asks the repository's Go CDCL solver whether admission can
coexist with a forbidden byte, and checks a deliberately weakened SAT control.
It then replays every octet payload in that bound through the public parser.

Run from this package directory:

```sh
PATH=/path/to/go-1.26/bin:$PATH PYTHONPATH=src \
  .venv/bin/python verification/sat_http09.py
```

The observed run reports 3 UNSAT SAT-model queries, a SAT negative control, and
131,586 parser cases (all 256^n payloads for n=0..2, both LF and CRLF). A final
CR before LF is explicitly accounted for because the parser consumes it as the
optional CRLF prefix. This is bounded model evidence and finite conformance,
not a DRAT certificate, production proof, or claim about later HTTP versions.

## HWEB06 paragraph / heading SAT slice

`sat_html2_paragraphs.py` asks the repository Go CDCL solver about a separate
strict structural subset. Its event alphabet is exactly `P`, `/P`, `H1`,
`/H1`, and `EOF`; each query is exactly four event slots, with EOF in slot 4.
The state is an explicit one-hot scope/depth value: root (depth 0), paragraph
(depth 1), heading (depth 1), or the invalid nested paragraph/heading state
(depth 2). A P start opens a paragraph; another P closes it and opens a distinct
paragraph; `/P` closes an open paragraph; H1 start closes P before opening H1;
H1 needs an explicit `/H1`; EOF closes P but rejects an open H1. Invalid
transitions are refused and accepted traces end at root.

The baseline has SAT legal traces (`P, H1, /H1, EOF` and `P, P, /P, EOF`)
and an UNSAT query for any depth-2 scope in the full bounded transition relation.
The repeated-P trace shows that the model permits distinct paragraphs. The negative control
removes the P-to-H1 implied close and returns SAT for that same trace, with the
heading nested under the paragraph. This isolates the effect of the closure
rule rather than baking a one-paragraph limit into the representation.

Run it from the repository root with the pinned tools:

```sh
HWEB_GO=/path/to/pinned/go/bin/go \
  /path/to/pinned/python/bin/python3.13 \
  code/packages/python/historical-web/verification/sat_html2_paragraphs.py
```

The model deliberately omits text, whitespace, attributes, byte/character
scanning, and parser recovery. It is bounded transition-model evidence, not a
proof that the Rust or Python implementation is equivalent to the model. The
separate exhaustive replay of bounded inputs against the real parser is the
source-level conformance evidence.
