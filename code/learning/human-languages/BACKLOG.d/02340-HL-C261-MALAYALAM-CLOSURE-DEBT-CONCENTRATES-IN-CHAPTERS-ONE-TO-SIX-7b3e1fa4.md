## HL-C261 — Malayalam's remaining script-closure debt is all in Chapters 1–6

After the romanization sweep and the `ML-S125`–`ML-S132` letters, Malayalam is
down to nineteen lessons that ask the reader to decode an untaught mark, and
sixteen of them sit in Chapters 1–6. The two chapter practice lessons alone
carry twelve and eight untaught characters:

    ML-C02-practice   12   അആഎണപറിീുെേൺ
    ML-C01-practice    8   അഇതദലശിെ
    ML-C05-taamasikkuka 7  ഇചഞയൊൻൽ
    ML-C06-dative-ikku  7  എജഞപലേൻ

None of these can be closed by adding romanizations: the untaught characters are
in the lesson BODY, not the headword, so the exposure rule does not reach them.
They close only when the letters are taught earlier than the lessons that use
them, and that runs into two structural blockers worth recording before somebody
rediscovers them one CI failure at a time.

**Blocker one — the drizzled ladder starts at Chapter 6.** `ML-S01-letter-ka` is
the first one-letter lesson and it lives in Chapter 6, while Chapter 1's own
runway (`ML-W01-*`) teaches only the seven shapes of *namaskāram*. Everything
Chapters 1–5 print beyond those shapes is untaught by construction. Closing it
means either extending the `ML-W*` runway per word through the opening chapters,
or moving the head of the `ML-S*` chain forward into Chapters 1–5.

**Blocker two — Chapters 6–31 have pinned lesson counts outside this track.**
`code/programs/typescript/language-ladder/tests/bookhashes.test.ts` hardcodes a
`[chapter, lessonCount]` pair for every Malayalam chapter from 6 to 31.
Inserting a script lesson anywhere in that range moves a pin in a file shared
with twenty-two other tracks, so the work cannot be done from a single-track
branch without a cross-language edit. Chapters 32 and up are unpinned, which is
why this tranche's eight letters landed there.

**A third, smaller one, found on the way.** `ML-C32-tinnuka` still decodes **ഊ**
one lesson early, because Chapter 32 already sits at exactly its twelve-atom
chapter budget and would have gone over. The **ഊ** lesson was placed in Chapter
33 instead. A Chapter 32 split would let it move back and clear that violation
too.

The honest summary: the romanization seam is exhausted for this track
(load-bearing headwords are at zero) and the never-taught-glyph seam is
exhausted (zero of sixty-seven), so the next real move on Malayalam closure is a
Chapter 1–6 script runway, and it needs a coordinator who can also move the
shared pin.
