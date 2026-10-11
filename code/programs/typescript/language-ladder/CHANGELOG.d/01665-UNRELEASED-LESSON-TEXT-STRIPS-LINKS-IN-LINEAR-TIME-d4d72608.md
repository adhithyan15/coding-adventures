## Unreleased — lesson text strips links in linear time

- `plainInline` in `src/lessonbody.ts` turned `[text](target)` and
  `![alt](src)` into plain text with regexes whose bracket classes did not
  exclude their own openers. A paragraph of unclosed `[` (or of `[a](` with
  no `)`) made every opener scan to the end of the text: about two seconds on
  50,000 characters. Each class now excludes its opener (`[^\][]`, `[^()]`),
  so each character is scanned from at most one opener. Lesson bodies are
  authored in the repo, so this was a robustness fix, not an exposure. A new
  test feeds four 50,000-character floods and asserts the answers.
