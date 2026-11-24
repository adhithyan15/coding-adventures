# Changelog

All notable changes to the coding-adventures monorepo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## Adding an entry

Every change gets its own numbered Markdown file beside this one; nothing is
appended here. The name is three parts, and it is enforced by
`isValidDocShardName` in
`code/packages/typescript/human-language-data/src/doc-shard.ts`:

```
NNNNN-SLUG-HEX8.md
```

- `NNNNN` — five digits, zero-padded, the rank. It is the order: code-unit
  order is chronological, so a new entry takes the next free rank. Zero is not
  a valid rank.
- `SLUG` — uppercase ASCII words joined by single dashes, saying what changed.
  Uppercase keeps the directory portable and readable.
- `HEX8` — eight lowercase hex characters, a digest that disambiguates parallel
  authors so two branches writing different entries never collide.

`_meta.md` (this file) is the only reserved name. A name that does not match
the pattern is rejected, so a lowercase slug or a seven-character digest fails
the check instead of landing a shard nobody can order.

