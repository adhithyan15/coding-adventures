### Added — guards, scoped to the defect class

- `standalone-book.test.ts` grew three: a foreign `XX-CNN` in a chapter, the same in a
  lesson's **frontmatter** (which never reaches the `.tex`, and is where the German
  instance hid), and an "already met" claim carrying an **out-of-volume locator**. The
  last is deliberately context-sensitive: ~86 chapters legitimately say "already met in
  Chapter 24", and a guard that bans the phrase outright would ban the ramp's own
  callbacks. Each was proven to fail on a reintroduced defect before being trusted.

