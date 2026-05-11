# adjudication-ir (Rust)

The typed intermediate representation for rule-based adjudication.

## What This Is

This crate is the Rust reference implementation of [`ADJ01`](../../../specs/ADJ01-adjudication-ir-grammar.md).
It defines the IR's data shapes (Fact / Query / Uncertainty / Rule /
Exception / Discarded nodes, with polarity, modality, source spans, and
provenance) and a total `validate` function that enforces the well-
formedness rules before any checker pass or logic backend touches the
document.

The IR is the foundation of the Adjudication framework. Every IR node:

- carries the **logic-core Term** it represents (no separate term language),
- carries mandatory **polarity** and **modality** tags,
- carries the **source spans** it was derived from,
- optionally carries a `lowered_from` edge pointing to a parent node
  in the refinement DAG.

## How It Fits in the Stack

```
   logic-core (LP00)            ← Term, LogicVar, Substitution, unify
        │
        ▼
   adjudication-ir              ← this crate (ADJ01)
        │
        ├── ADJ02 coverage checker
        ├── ADJ03 polarity/modality checker
        ├── ADJ04 round-trip checker
        ├── ADJ05 adversarial verifier
        ├── ADJ06 clarification dialogue
        └── ADJ09 rule compilation
```

This crate is consumed by every checker-pass and dialogue
implementation. They produce and inspect IR documents; this crate
guarantees the documents are well-formed before they leave any
boundary.

## API at a Glance

```rust
use logic_core::{atom, compound};
use adjudication_ir::{
    DocumentId, IRDocument, IRNode, NodeId, NodeKind, Polarity, Modality,
    Span, validate,
};

let doc_id = DocumentId::new("doc1");

let node = IRNode {
    id: NodeId::new("F1"),
    kind: NodeKind::Fact,
    term: compound("chest_pain", vec![atom("patient")]),
    polarity: Polarity::Denied,
    modality: Modality::Present,
    source_spans: vec![Span::new(doc_id.clone(), 0, 28)],
    confidence: 0.93,
    lowered_from: None,
    discard_reason: None,
    metadata: Default::default(),
};

let doc = IRDocument {
    document_id: doc_id,
    nodes: vec![node],
};

match validate(&doc) {
    Ok(()) => println!("IR is well-formed"),
    Err(e) => println!("Validation failed: {:?}", e),
}
```

## Status

Experimental. The structural types are stable enough to build the
checker passes against. JSON serialization will arrive in a subsequent
slice once the schema is hardened.
