---
category: Testing & coverage
---

# A Chinese headword in characters no lesson writes raises the letter-anchoring unwritten count, so exposure characters cannot enter a headword

**What went wrong.** The plan for Chinese chapter 30 gave four new words hanzi
headwords (怎么样, 忙, 累, 还行) whose characters no lesson writes. The plan called
them "exposure": `script-closure.ts` exempts a headword that has a
`romanization`, so closure stayed at 0. The gate, the self-check and closure
all passed. Then `tests/letter-anchoring.test.ts` failed with "chinese
unwritten: 忙 怎 样 累 行 还: expected 6 to be less than or equal to 0".

**Why.** `measureLetterAnchoring` reads every Han glyph in every non-writing
lesson's headword and owes each one a letter lesson that writes it. Closure
excuses an exposure headword. Anchoring does not. The Chinese ceiling in
`tests/letter-anchoring-ceilings/chinese.json` is `unwritten: 0`, and it may only
fall. Japanese is also at 0, because it writes every kanji its headwords use.

**Fix.** The four words got pinyin headwords (`zěnmeyàng`, `máng`, `lèi`,
`hái xíng`) and were taught by ear, and their characters are not printed
anywhere. The one character the chapter writes, 很, kept its hanzi headword.
The ceiling did not move.

**What to do differently.** Before planning hanzi (or kanji) headwords, run
`tests/letter-anchoring.test.ts` on a sample, not only the gate and closure.
In Chinese and Japanese, every character in a headword needs a writing lesson
in the track, or the word has to wait for one. Exposure through the headword
works for closure only.
