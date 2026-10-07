---
category: Testing & coverage
---

# Distinguish absent override strings from explicit whitespace-only configuration

Independent review found that trimming the entire limit-override string before the absent-value check accepted an explicit whitespace-only argument as defaults. The parser maps an omitted flag to the exactly empty string. Test that exact value for absence, then trim each supplied pair/name/value inside the grammar. Process regressions for spaces, mixed ASCII whitespace and Unicode whitespace first reproduced exit 0 and now require an empty-pair diagnostic, failed status, empty stdout and preserved files.
