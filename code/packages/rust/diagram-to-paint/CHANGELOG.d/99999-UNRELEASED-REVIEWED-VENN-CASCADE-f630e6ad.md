## Unreleased — reviewed Venn property cascade

Present style fields overlay earlier values in authored order. Missing fields
retain earlier values; authored zero remains explicit. Region lookup precedes
text lookup, with the first duplicate key selected and union keys sorted.
Twelve complete IR cases cover three ingress forms and target/order controls;
192 complete scene/query cases retain176 earlier cases and add16 cascade cases.
These are bounded portable contracts, not full CSS/rendering or Mermaid parity.

Actual source `f630e6ad` passed 2,765 tests across 120 targets, zero failures,
one unchanged parent-only archive ignore, and strict all-target Clippy for all
20 affected packages. Compiler884 and full unfiltered LANG500 passed. Focused
runtime has 116 sources/696 actual cells on six available backends; all636 old
tuples and106 old sources stay exact. CLR is unavailable and uncounted.
Source/native-focused review19fd3975 and style-focused76b54bb9 passed; independent
full terminal review is pending. Hosted checks and publication remain separate.
Final-source Miri **DEFERRED/PENDING**, never passed.
