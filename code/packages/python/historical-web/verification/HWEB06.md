# HWEB06 bounded paragraph assurance

`tests/test_html2_paragraphs_assurance.py` replays the independent
`historical-web-html2-paragraphs-v1/vectors.json` positive, negative, historical,
and XML comparison cases. The historical comparison runs the same source
through both public exports, so HWEB00's empty `P` marker remains distinct from
the later text-bearing paragraph. The fixture is separate from HWEB00's 1992
vectors so a later paragraph profile cannot silently change the earlier tree
contract.

The exhaustive transition check enumerates every token word of lengths zero
through six over start-P, end-P, start-H1, end-H1, and one text token. EOF is
implicit after each word. This yields **19,531** cases. A small independent
state model predicts exact trees or rejection: P is implicitly closed by
explicit end, the next P, the next H1, or EOF; H1 only closes explicitly. The
test compares the parser's full canonical tree or requires the documented
`ValueError` result for every word.

The repository VER05 property engine checks 512 seeded generated marker words
against the same independent state model. One VER06 campaign mutates legal
marker-token histories against that model. A separate VER06 campaign maps
arbitrary byte mutations through UTF-8 with `surrogateescape` to exercise raw
scanner candidates; `ValueError` is expected rejection, successful output must
meet text-only distinct-root invariants, and other exceptions are crashes. Both
campaigns run 2,048 iterations with explicit input caps. Long sequential inputs
check 20,000 P blocks and 20,000 explicitly closed H1 blocks without a timing
threshold; an unterminated 100,000-character H1 exercises the EOF boundary. A
`str` subclass override regression verifies CRLF normalization bypasses a
virtual `replace` override after source admission.

XML comparison uses the same omitted-end source accepted by HWEB06 and checks
that Python's standard XML parser rejects it. XML 1.0 requires explicit end
tags; see the repository's [XML01 parser contract](../../../../specs/XML01-xml-parser.md)
and the XML 1.0 Recommendation at <https://www.w3.org/TR/xml/>. This is a
semantic comparison, not a claim that either parser implements the other's
language.

The paragraph token SAT model is owned by the separate Go verification work.
This Python package suite does not run or duplicate that solver. Its evidence
is finite exhaustive transition enumeration plus the stated PBT/fuzz budgets;
it does not establish an unbounded proof or a DRAT certificate.
