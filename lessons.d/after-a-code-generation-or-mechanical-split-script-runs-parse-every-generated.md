# After a code-generation or mechanical split script runs, parse every generated file before trusting the transformation

**After a code-generation or mechanical split script runs, parse every generated file before trusting the transformation.** While splitting the human-language cross-script integration test, patch-marker `+` characters accidentally survived inside a template string and were written into a shared helper. The structural statement-count check still passed because it measured moved assertions, not generated TypeScript syntax. Run the narrowest parser/test immediately after generation; statement counts prove coverage movement, not that wrapper text is valid source. Run it from the package directory too: invoking `npx vitest` at the repository root made Vitest discover an unrelated package whose local dependencies were not installed, turning a correct focused command into a false cross-repository failure.

A condensed quick-reference of mistakes made during development, grouped by category. Read this file before starting work that touches BUILD files, CI, native extensions, or any of the language-specific pitfalls below. Entries are kept short on purpose — when a rule recurs, the canonical entry is here, not buried in chronology.

---
