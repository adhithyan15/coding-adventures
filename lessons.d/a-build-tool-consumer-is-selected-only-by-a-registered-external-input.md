---
category: BUILD files & dependency management
---

# A build-tool consumer is selected only by a registered external-input row

A selection test expected a Warrant adopter package to run whenever its `.warrant` file changed,
merely because that package existed and other Warrant consumers were selected. The repository
input registry had rows for the global grammar checker and Warrant oracle, but no row for that
adopter's spec. The test exposed the mistaken expectation rather than a missing dependency edge.

Before asserting a package selection, inspect its actual row and matcher. Package presence or a
shared file format does not create an external-input edge; register the exact source path only for
the consumer whose tests read it.
