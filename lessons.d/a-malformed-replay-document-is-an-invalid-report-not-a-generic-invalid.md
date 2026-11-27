---
category: Rust
---

# A malformed replay document is an invalid report, not a generic invalid input

Replay APIs validate a whole portable report, so malformed JSON on that path
belongs to the stable `invalid_report` error category. `invalid_input` describes
requests that fail an operation's request schema, such as a malformed analysis
request. Keep consumer assertions tied to the operation's public error contract
instead of assuming every JSON parse failure shares one category.
