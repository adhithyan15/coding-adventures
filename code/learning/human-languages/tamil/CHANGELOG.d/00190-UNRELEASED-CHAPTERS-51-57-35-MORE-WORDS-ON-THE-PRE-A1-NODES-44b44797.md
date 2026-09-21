## Unreleased — Chapters 51-57: 35 more words on the pre-A1 nodes

Thirty-five lessons, one new word each, appended after chapter 50 and chained
from `TA-C50-welcome`. The pre-A1 vocabulary count moves 84 → 119 of 300, a rise
of exactly one per lesson, and Tamil goes from the furthest-behind track on that
measure to level with the leaders.

Seven chapters, one pre-A1 spine node each, all seven nodes used a second time:

| Ch | Node | Words |
|---|---|---|
| 51 | MEET-GREET | ஆகாயம் சூரியன் நிலா விண்மீன் மேகம் |
| 52 | EXCHANGE-NAMES | மரம் கிளை தளிர் வேர் விதை |
| 53 | POLITE-REQUEST-REPAIR | நதி மலை வயல் பாதை கிராமம் |
| 54 | COURTESY-THANK | பாய் கூடை கத்தி கிண்ணம் பெட்டி |
| 55 | CHECK-WELLBEING | கழுத்து முதுகு உதடு நகம் எலும்பு |
| 56 | RESPOND-BASIC | சிறிது நிறைய குறைவு தேவையில்லை ஆகட்டும் |
| 57 | TAKE-LEAVE | காற்று மணல் சேறு புகை கனல் |

R1 — the tight reinforcement window — improved 0.2746 → 0.2723 with its
numerator held at 1127: the tranche added 35 atoms and missed the window with
none of them. Forward references held at 16, rule statements at 30, script
closure violations at 35 and cross-chapter number references at their baseline.

**Nine candidate words were discarded rather than reworded.** வானம் ('sky') is
already glossed inside `TA-C20-vaanilai`, ஞாயிறு ('sun') is committed as the
name of a weekday in `TA-C10-vaara-kizhamai`, and ஆறு ('river') is the same
string the book already counts with — so ஆகாயம், சூரியன் and நதி took their
places. இலை ('leaf') was caught only by checking ROMANIZATIONS as well as
script: `TA-C50-betel-leaf` spells out "*-ilai* is *ilai*, 'a leaf'" in prose,
so teaching it here would make that page point forward; தளிர் replaced it.
வேண்டாம் sits inside `TA-C39-vendum` and போதும் is already chapter 47's
headword, so தேவையில்லை and ஆகட்டும் took the last two reply slots.

**One discard no string check could have made.** அதிகம் ('a lot') passed every
substring and romanization test, because the corpus does not contain the word —
it contains *ati-*, glossed as "excessive, very" inside `TA-C26-kaalai`'s
account of அதிகாலை. Teaching அதிகம் would have re-taught that morpheme and
pointed the earlier page forward. நிறைய replaced it.

**Two were discarded by the taught-glyph filter.** கொஞ்சம் ('a little') and
ஓலை ('palm leaf') need ொ and ஓ, neither of which any Tamil writing lesson
teaches. Every headword in the tranche is spelled from the forty glyphs the
track's own writing lessons already cover, so the tranche adds zero script
closure violations and launders nothing through an exposure-exempt headword.
கொஞ்சம் still earns its mention in `TA-C56-a-little`, romanized.

Every cousin citation is checked against a string already committed in this
repo rather than recalled: the Malayalam and Telugu words quoted here are the
headwords of `ML-C53`..`ML-C59` and `TE-C53`..`TE-C59`. Citations needing a
letter the Tamil track has never shown are romanized instead, so the tranche
adds no new foreign glyphs to any lesson.

`TA-C50-welcome` was reworded before anything was appended (HL-C201). It said
the welcome "brings this book back to where it started", which was true while
it was the last lesson and false the moment chapter 51 existed. It now says the
word sets the reader back down where the book began — a claim about the reader,
not about where the book stops.

The book renders in `scriptSet: tamil-comparisons`, copied field for field from
the chapter-50 target: 403 pages, zero missing characters, zero overfull and
zero underfull boxes.

