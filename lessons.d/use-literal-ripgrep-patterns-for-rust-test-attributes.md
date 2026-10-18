---
category: Testing & coverage
---

# Use literal ripgrep patterns for Rust test attributes

A combined regex used shell-style bracket escaping for the Rust `#[test]`
attribute. Ripgrep rejected the pattern as an unclosed character class before
searching any source. Prefer `rg -F -e '#[test]' -e 'TimelineEvent'` when the
patterns are literal. If regex syntax is necessary, escape the brackets for
the regex engine explicitly instead of borrowing a shell glob convention.
