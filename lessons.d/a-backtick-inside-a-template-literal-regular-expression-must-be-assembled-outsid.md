---
category: TypeScript / JavaScript
---

# A backtick inside a template-literal regular expression must be assembled outside the literal

A Vitest assertion tried to build a dynamic regular expression for a Markdown
code span inside a TypeScript template literal. Writing the escaped Markdown
backtick directly inside the template closed the TypeScript literal, so the
suite failed during transformation before any test ran.

Build the pattern from a template-literal segment and a separately quoted
backtick (or use a safely delimited ordinary string). Run the focused test once
immediately after adding a new dynamic regular expression; a parser failure is
not evidence about the behavior the assertion was meant to cover.
