# dom-core

Small DOM tree model for Venture browser packages.

This crate preserves browser-facing structure: document type nodes, elements,
attributes, text, and comments. Higher layers can project this tree into
`document-ast` for document rendering or combine it with CSS/layout packages
for browser rendering.
