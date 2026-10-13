---
category: TypeScript / JavaScript
---

# Avoid HTML-comment sanitization regexes in tests when a line-level metadata exclusion is sufficient

CodeQL flagged the test's broad HTML-comment replacement as incomplete multi-character sanitization. The test only needs to exclude its own finite set of speaker tags, so enumerate those exact metadata tokens, remove only the tokens with `replaceAll`, preserve adjacent scored speech, and fail if any `<!-- speaker:` prefix remains instead of attempting to sanitize arbitrary HTML comments with a regular expression.
