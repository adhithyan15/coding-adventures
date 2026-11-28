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
(depth 1), heading (depth 1), nested P/H1 (depth 2), or nested P/P (depth 2).
A P start opens a paragraph; another P closes it and opens a distinct
paragraph; `/P` closes an open paragraph; H1 start closes P before opening H1;
H1 needs an explicit `/H1`; EOF closes P but rejects an open H1. Invalid
transitions are refused and accepted traces end at root.

The baseline has SAT legal traces (`P, H1, /H1, EOF` and `P, P, /P, EOF`)
and an UNSAT query for any depth-2 scope in the full bounded transition
relation. The repeated-P trace shows that the model permits distinct
paragraphs. Two negative controls remove the P-to-H1 close and the P-to-P
close; each returns SAT with the corresponding depth-2 nested-scope witness.
This isolates closure effects rather than baking a one-paragraph limit into the
representation.

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
source-level conformance evidence for the Python source so far; no equivalent
exhaustive replay has yet been established for Rust.

## HWEB08 request-line boundary SAT slice

`sat_http10_request_line.py` uses the repository Go CDCL solver for a bounded
byte-level miter between HWEB08 and HWEB00. It gives each octet in the complete
request line eight Boolean variables, fixes the common GET prefix and
` SP HTTP/1.0 CRLF` suffix, and checks total target lengths 1, 2, and 3. The
target is `/` followed by zero to two ASCII letters, digits, `/`, `.`, `_`, or
`-`. The HWEB00 miter constrains the same wire coordinates `wire[4:-2]` as its
printable nonspace address. The internal H10 separator is an SP in that slice,
so no shared-wire acceptance assignment exists in the checked domain.

For every bounded target length, a legal HWEB08 line alone is SAT, strict
HWEB08/HWEB00 shared-wire acceptance is UNSAT, and the negative control that
relaxes the H00 printable-nonspace guard is SAT. This gives a concrete witness
that the HWEB00 guard is material. The model fixes GET, excludes targets longer
than three bytes, and models language conditions rather than either parser's
scanner implementation. It does not prove scanner/source equivalence. HWEB08
requires canonical CRLF; HWEB00 independently accepts LF or CRLF.

Run from the repository root with the pinned tools:

```sh
HWEB_GO=/path/to/pinned/go/bin/go \
  /path/to/pinned/python/bin/python3.13 \
  code/packages/python/historical-web/verification/sat_http10_request_line.py
```

## HWEB09 C++ flat-arena append SAT slice

`sat_cpp_arena_append.py` models one `append_node` transition from the HWEB09
C++ implementation. The bounded index width is 3 bits: the pre-state has two
nodes, the appended node is index 2, the post-state count is 3, and `no_index`
is 7. This local arithmetic cannot wrap. The model checks a root append, the
first child of an existing parent, and a next sibling after an existing child.
Each link to the appended node is SAT in-range; requiring that same link to be
one past the post-state count is UNSAT.

For cycle reasoning, the forward traversal graph consists of
`first_child_index` and `next_sibling_index`; `parent_index` is a backward
reference. The appended node begins with no first child or next sibling, while
only pre-existing nodes gain a forward link to it. Given an already acyclic
pre-state, this append transition adds a leaf and cannot create a traversal
cycle. The model's stack-parent precondition is that a child parent is an
existing index (or root uses `no_index`); this comes from the parser's
`stack_.back()` origin, not a standalone runtime check in `append_node`.
Weakening that precondition to let the parent equal the new node produces a
SAT self-child cycle control.

Run from the repository root with the pinned tools:

```sh
HWEB_GO=/path/to/pinned/go/bin/go \
  /path/to/pinned/python/bin/python3.13 \
  code/packages/python/historical-web/verification/sat_cpp_arena_append.py
```

This is a small one-step model of the C++ append/link transition, not a proof
of `std::vector`'s arbitrary-size growth, the no-wrap case at machine limits,
all parser paths, or the whole C++ implementation. It adds no production
runtime or platform requirement.
