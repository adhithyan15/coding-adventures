## Fixed — chapters 2, 3 and 4 payoffs cover their chapters

Three chapter payoffs were below the 0.5 `chapter-payoff-not-representative`
floor, each listing only its own two practice atoms. Each practice lesson now
exercises the chapter's atoms with recall prompts and answers drawn only from
the introducing lessons, split into a `Script — the letters, from memory`
section (glyph recognition) and spoken recall in Guided Practice or the
wrap-up. New atoms are added to `requires.knowledge`, `practises.knowledge` and
the section markers, and each `chapters.d` payoff note and summary is
rewritten to match.

- **Chapter 2** (`AR-C02-practice`): 2/33 (0.06) → 17/33 (0.52). It now asks
  which letters build اسم, ما, أنت and تشرفنا and what the ي of اسمي adds;
  whether *ism* is kin to *name* (Hebrew *shem*); how "my" and "your" are glued
  on; who hears *anta*/*anti* (Hebrew *atta*/*att*); *mā* before a noun; and
  what *tasharrafnā* literally says. Guided Practice and the wrap-up now declare
  the *ismī*, *mā ismuka/ismuki*, zero-copula and gender-not-register atoms they
  already drilled. The "pieces" list becomes one sentence, the warm-up and
  next-chapter line are shortened. This is the most the payoff can honestly
  assess: the chapter's other sixteen atoms (the *thaman*, *mudarris* and
  *ahlan* anchors and the W04–W06 script lessons) sit in `AR-PATH-008`, after this lesson in
  `AR-PATH-007`, so they cannot be its prerequisites. `max_seconds` 240 → 290
  (computed 285).
- **Chapter 3** (`AR-C03-practice`): 2/19 (0.11) → 19/19 (1.00). Adds
  `AR-W09-khayr-bikhayr` as a prerequisite, which brings in the ḥājj anchor and
  the W07–W09 letters (all earlier on the path). It now asks what separates
  ح خ ج (and their H, C/G descendants), describes ك and the non-joining ر,
  spells خير, names the one new letter of كيف حالك (ف) and the none of بخير and
  الحمد لله, and where حال breaks; the wrap-up asks for both answers
  literally, *ḥāl* as how things have turned, *kayfiyya*, the word for a
  pilgrim and the ḥ–m–d root. To stay under the 300 s cap, the gender table
  becomes one sentence on *-ka/-ki* in "the pieces" and the zero-copula and
  "what do al-, bi-, li- share" questions are dropped from the wrap-up (both
  ideas remain stated in the lesson). Computed duration was already 299 s
  against a declared 240; `max_seconds` 240 → 290 (computed 290).
- **Chapter 4** (`AR-C04-practice`): 2/16 (0.13) → 16/16 (1.00). Adds
  `AR-W12-maa-salama` as a prerequisite (it reaches W10 and W11). It now asks
  the shapes of ع (and its dotted twin غ) and ه, what ة is, what builds مع and
  السلامة, and which two shapes of إلى اللقاء the writing has not reached; the
  wrap-up asks what the *faʿāla* pattern made of *salām*, whether *maʿa
  s-salāma* is a plain sign-off or a wish, whether *maʿa* attaches (Hebrew
  *ʿim*), and what *ilā l-liqāʾ* says and from which root. To fit, the root
  ledger keeps one word per root, the attaching-pieces table, the zero-copula
  line, the *hasta* question and the spoken bookend are removed. Computed
  duration was already 296 s against a declared 240; `max_seconds` 240 → 290
  (computed 290).

