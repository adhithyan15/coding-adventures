# Portable GFM extension v1 fixture

This directory is the language-neutral executable corpus for
[TE04 portable GFM extension conformance](../../TE04-gfm-portable-conformance.md).
Its sixteen independently authored source/Document-AST/HTML triples cover
strikethrough, task items, pipe tables, source-preserving fences, negative
lookalikes, and one mixed document.

Run the strict validator from the repository root:

```sh
python3 code/scripts/gfm_extension_conformance.py
python3 -m unittest discover -s code/scripts/tests -p test_gfm_extension_conformance.py
```

`schema.json` closes the transport and AST shapes.  The standard-library
validator additionally rejects duplicate keys, ambiguous JSON numbers,
invalid Unicode scalars, overlarge/deep data, reordered or renamed cases, and
coordinated changes to source/AST/HTML literals. Direct decoded-object callers
receive the same bounded-work protection: aliases, cycles, excessive items,
and excessive aggregate string data are rejected. Fence-name tokenization uses
the contract's exact ECMAScript whitespace set, independent of Python's.

Fence payloads are inert strings.  Passing this corpus does not execute
Mermaid, Graphviz, plugins, HTML, files, or network requests.  The neutral
foundation grants no native-language adoption credit by itself.

See [VALIDATION.md](VALIDATION.md) for the durable local evidence and explicit
limits recorded before publication.
