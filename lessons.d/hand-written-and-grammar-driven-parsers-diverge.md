---
category: Compiler / VM / language pipeline
---

# Hand-written and grammar-driven parsers diverge

Grammar-driven parsers pick up `python.grammar` updates automatically; hand-written ones (Perl python-parser) have hardcoded type checks. After token name changes, grep ALL parsers for the old name.
