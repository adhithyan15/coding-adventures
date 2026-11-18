---
category: Compiler / VM / language pipeline
---

# A parsed Warrant AST must stay inside its SourceDocument and layout scope

A helper attempted to return a parsed `WarrantFile` from a block that had already dropped its
`SourceDocument` and layout token vector. Rust correctly rejected the return: parser nodes and token
text borrow both owners. Parse, check, inspect, and evaluate the AST inside one callback while those
owners remain alive; return only owned results from that callback.
