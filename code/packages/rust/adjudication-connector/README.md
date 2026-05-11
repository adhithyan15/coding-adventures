# adjudication-connector (Rust)

Lowers the adjudication IR (ADJ01) to logic-engine clauses (LP19) per
ADJ11. Runs end-to-end adjudications.

## What This Is

This crate is the wire-up layer between the framework's intermediate
representation and its probability-aware logic engine. It is the
implementation of [`ADJ11`](../../../specs/ADJ11-problog-connector.md).

Given an `IRDocument` (per ADJ01), it produces a `KnowledgeBase` that
LP19's engine can search. Given a query, it returns either a
deterministic substitution (when the IR has no probabilistic content)
or a proof DAG plus the query's probability (otherwise).

## Where It Fits

```
   adjudication-ir (ADJ01)                logic-engine (LP19)
        │                                       ▲
        ▼                                       │
   adjudication-connector (ADJ11)  ──── lowering rules ────┘
        │
        ▼
   end-to-end adjudication result
```

## API at a Glance

```rust
use logic_core::{atom, compound};
use adjudication_ir::{IRDocument, IRNode, NodeId, NodeKind, Polarity, Modality, Span, DocumentId};
use adjudication_connector::run_adjudication;

// Build an IR document (in real use, the extractor LLM produces this)
let doc_id = DocumentId::new("declaration");
let ir = IRDocument {
    document_id: doc_id.clone(),
    nodes: vec![
        IRNode {
            id: NodeId::new("F1"),
            kind: NodeKind::Fact,
            term: compound("father", vec![atom("homer"), atom("bart")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id.clone(), 0, 20)],
            confidence: 0.95,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        },
        IRNode {
            id: NodeId::new("Q1"),
            kind: NodeKind::Query,
            term: compound("father", vec![atom("homer"), atom("bart")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id, 0, 20)],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: Default::default(),
        },
    ],
};

let results = run_adjudication(&ir).expect("lowering should succeed");
assert_eq!(results.len(), 1);
// results[0] contains the engine's SearchResult for the Query Q1.
```

## Rule Subtype Encoding

ADJ Rule nodes encode their subtype in the term (see ADJ01). The
connector recognises the following compound functors as Rule subtypes:

| Term shape | LP19 lowering |
|---|---|
| `definitional(head, [body...])` | `Rule { probability: Certain }` |
| `probabilistic(p, head, [body...])` | `Rule { probability: Value(p) }` |
| `constraint([body...])` | `Rule` with synthetic `_constraint(c_N)` head |
| `default(head, [body...], [exceptions...])` | `Rule` with `Pos` body + `Neg` exceptions |

Any other compound functor used at a Rule node produces a
`LoweringError::UnknownRuleSubtype`.

## Status

Experimental. The deterministic and probabilistic engine paths both
work end-to-end. JSON ingestion, `as_of` priority resolution, and
conditional-evidence wiring (LP19c) are planned follow-ups.
