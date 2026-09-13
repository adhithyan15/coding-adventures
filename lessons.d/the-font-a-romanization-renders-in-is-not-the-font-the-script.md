# The font a ROMANIZATION renders in is not the font the script renders in — scope the glyph probe to the whole page, not to the script (human-language books)

**Context:** HL-C222, the Bengali script tranche. HL-C214 had been written ONE
TRANCHE EARLIER, saying in so many words: check the generated `.tex` against the
actual font file. I did. CI still failed:

    bengali: missing_character rose to 29 against a baseline of 0

**What happened:** the lessons romanise Bengali's inherent vowel as `nɔ`, `kɔ`,
`mɔ` — U+0254 LATIN SMALL LETTER OPEN O, the correct IPA. Bengali's book sets
`\setmainfont{Latin Modern Roman}`, and **Latin Modern does not have U+0254**.
Twenty-nine occurrences, exactly the number CI counted.

**Why the probe missed it, and this is the whole lesson.** I scanned
`bengali/book/**/*.tex` for codepoints **in the Bengali range** against
**NotoSansBengali**. Both halves of that are too narrow:

- the Bengali characters were never the risk — they are inside `\bn{...}`, which
  selects the Noto face that was chosen *because* it covers them;
- **everything else on the page — the prose, the romanization, the IPA — renders
  in the MAIN font**, and nothing was checking it.

A book is not one font. Scope the probe to **every non-ASCII character not inside
a script wrapper, against the main font**, and separately to the wrapped runs
against their own face.

```python
text = re.sub(r"\\bn\{(?:[^{}]|\{[^{}]*\})*\}", "", tex)   # strip wrapped runs
missing = {c for c in text if ord(c) > 127 and ord(c) not in latin_modern_cmap}
```

Note the nested-brace form of that regex: `\bn{\textbf{x}}` is common and a
`[^{}]*` version silently fails to strip it, which puts Bengali characters into
the main-font bucket and produces a flood of false positives.

**And a second instrument failure inside the same investigation.** My first pass
scanned `bengali/book/` **while checked out on a different branch** — the Hindi
one, which has no Bengali chapter — and reported the five pre-existing preamble
characters as if they were the answer. Same family as `git stash` not stashing
untracked files: *the tool ran perfectly against the wrong tree.* **Print
`git branch --show-current` in the same command as any cross-branch diagnosis.**

**Rule of thumb for this corpus:** the Latin-script books (Spanish, Latin, French…)
have only Latin Modern, which is a **typesetter's** font — good Western European
coverage, thin outside it. It lacks `ǣ` (HL-C214) and `ɔ` (this one), and it will
lack the next IPA symbol somebody reaches for. If a romanization needs a phonetic
character, check it before writing 29 of them; `ô` (U+00F4) and `ə` (U+0259) are
present and usually say what is needed.
