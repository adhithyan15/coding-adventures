## Unreleased - DT18 Swift hash-map fixture routing (Go)

- Changes to the shared DT18 contract or its versioned fixture select the Swift
  hash-map consumer, and the canonical `cases.json` participates in its
  fallback package hash. Exact spec names, language filtering, and a
  slash-delimited fixture prefix keep lookalikes out and missing consumers
  fail closed.
