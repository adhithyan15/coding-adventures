---
category: Testing & coverage
---

# Double-quoted YAML frontmatter ships literal backslashes, because the reader does not decode escapes

A Malayalam punctuation lesson needed a pair of quotation marks as its headword,
so it was written the way YAML documents it:

```yaml
headword: "\" \""
```

That shipped a five-character string with **literal backslashes** in it. The
generated narration carried `"\\\" \\\""` where its five sibling lessons carried
`"."`, `","`, `"?"` and so on.

**The repo's frontmatter reader is deliberately tiny and does not decode
escapes.** `src/frontmatter.ts`'s `unquote()` strips the outer pair and returns
the rest verbatim:

```ts
if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
  return value.slice(1, -1);
}
```

So `"\" \""` loses only the outermost quotes and keeps both backslashes. Single
quotes round-trip correctly, because nothing inside them needs escaping:

```yaml
headword: '" "'
```

**Nothing caught it, and the reason is worth knowing.** The lesson's rendered
*title* was correct throughout, because titles come from the body `#` heading
rather than from frontmatter. Validate passed, all twelve gates passed, the full
suite passed, the book compiled strict and the LaTeX warning scanner reported
zero. The defect lived entirely in a field that no check compares against
anything.

**Why it is worth fixing rather than tolerating.** These were `type: writing` and
`type: review` lessons, and `CONTENT_TYPES` (`constants.ts`) restricts the
glossary and index — the sinks that interpolate a raw headword into
`\textbf{…}` — to `word` and `phrase`. So no backslash reached LaTeX this time.
On a `word` lesson it would have, and `\"` is a TeX accent control sequence.

**Do this instead.** Quote a frontmatter value that contains quotation marks or
backslashes with **single** quotes. A quick check on any new lesson:

```sh
python3 -c "import yaml,sys;print(repr(yaml.safe_load(open(sys.argv[1]).read().split('---')[1])['headword']))" <lesson>.md
```

A real YAML parser and the repo's reader should agree. Where they disagree, the
repo's reader is what ships. This also catches the sibling trap: an *unquoted*
plain scalar containing `: ` (colon-space) parses fine under `indexOf(":")` and
throws *"mapping values are not allowed here"* under a standard parser, so the
file is readable by the build and unreadable by every other tool.
