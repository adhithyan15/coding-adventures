---
title: Welcome to Acme Docs
sidebar_label: Home
sidebar_position: 1
---

This is a **demo documentation site** built end-to-end by the DOC00
v0 package cluster.  Every byte of this site — the markdown
parsing, heading anchors, table of contents, code blocks, syntax
highlighting, sidebar, page shell, search index, search client
JS — comes from eleven small composable packages that you can
audit independently.

## What this demo shows

- Markdown is parsed by `commonmark-parser` into a `DocumentNode`
  AST.
- The AST flows through the **content pipeline**:
  `forme-doc-frontmatter` → `forme-doc-heading-anchors` →
  `forme-doc-toc-extractor` → `forme-doc-code-block-decorator`
  → `forme-doc-syntax-highlighter`.
- The decorated AST is rendered to HTML by
  `document-ast-to-html`, then wrapped in chrome by
  `forme-doc-page-shell`.
- The sidebar is built by `forme-doc-sidebar-builder`.
- The search index is built by `forme-doc-search-tokenizer`
  plus `forme-doc-search-index-builder`.
- The whole site is composed by `forme-doc-site-emitter` into a
  `PageBundleConfig` and written to `dist/` by this driver.

Browse the sidebar on the left.  Try the navigation; every
internal link below resolves to a real page you can read.

## Where to go next

- [Getting started](/getting-started) — install and run.
- [Installation guide](/guide/installation) — detailed setup.
- [Configuration guide](/guide/configuration) — knobs and dials.
- [API reference](/api/reference) — the public surface.
- [FAQ](/faq) — common questions answered.
