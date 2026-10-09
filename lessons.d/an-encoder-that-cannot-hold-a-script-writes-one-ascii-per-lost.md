---
category: Repo policy / workflow reminders
---

# An encoder that cannot hold a script writes one ASCII ? per lost character, and valid-but-meaningless text passes every gate that only checks safety and reproducibility

Chapters 1-6 of the Japanese human-languages track were authored (#12472)
through a tool that wrote a legacy single-byte encoding. Every kana, kanji and
curly quote came out as one `?`, so 26 review pulses read like

```
[PAUSE 15s] Say ??? and tap all three morae before writing the new sign.
[PAUSE 15s] Write ? from memory before tracing ?.
[PAUSE 15s] Say ?????, add the dakuten to ?, and recall how ?hard to exist? became thanks.
```

and shipped that way into the published book and the narration for seven
weeks. Nothing failed: the text is valid UTF-8, valid Markdown, valid LaTeX,
every glyph is in every font, and `check:books` / `check:narration` confirmed
the byte-identical damage on every run because the generators reproduce
their input faithfully. The English around the damage survived, so a skim of
the diff looked fine too.

**The fix** was in two parts. The text came back by inference, because no
revision (not even the pre-squash PR commit) ever held it. Three constraints
agreed every time: one `?` per lost *character* (not byte), the block's
`hl-knowledge: assesses=[…]` list, and the sentence around the gap ("three
morae", "its two known signs"). Then `src/lost-script.ts` in
human-language-data became a corpus gate (`tests/lost-script.test.ts`). It
flags a `?` standing where a word belongs: a run not following a word, a
mark glued to a following letter, and a lone mark with the sentence carrying
on after it. French is exempt from the lone rule because its typography
spaces the mark.

**What to do differently:**

- After writing non-ASCII content with any tool, grep the result for runs
  of `?` (`grep -rnP '(?<![\p{L}?])\?{2,}|(?<!\S)\? [a-z]'`) before
  committing. The damage is invisible to every structural check by
  construction. (Expect French and the punctuation lessons to show up in
  that rough grep; the gate's allowances handle them.)
- A gate that proves output is *safe* and *reproducible* says nothing about
  whether it is *meaningful*. When content is generated or round-tripped,
  add at least one check on the meaning of the text. `literal-markup.ts`
  (HTML entities printed as text) taught the same lesson once already.
- When recovering lost text, check `git log -p --follow` and the PR's own
  commits (`gh api repos/<o>/<r>/pulls/<n>/commits`) first, and record every
  inference in the track changelog.
