# dom-to-document-ast

Converts `dom-core` documents into the format-agnostic `document-ast` IR.

The package is intentionally separate from `html-parser`: the parser remains a
DOM producer that can be used by browser, tooling, CSS, and scripting work, while
this crate is a small adapter for pipelines that want to flow into existing
document layout/rendering packages.
