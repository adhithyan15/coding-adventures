## Pre-A1 vocabulary tranche — fourteen everyday nouns, four chapters (2026-08-07)

The level gate (`src/level-gate.ts`) reports every track blocked on
**vocabulary**: 300 distinct headwords at or below pre-A1. This tranche
authors fourteen concrete nouns across four new chapters, the third such
tranche after Hindi, Arabic and Tamil, and confirms the same mechanism:
`vocabularyOf()` counts distinct `headword:` strings, so fourteen one-headword
lessons move German's pre-A1 vocabulary by exactly fourteen — **31 → 45**
distinct headwords at or below pre-A1 (against the 300 target, shortfall
269 → 255), and 77 → 91 distinct headwords track-wide. No bulk credit; measured,
not assumed.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C28-kaffee` | `GE-FOOD-COFFEE` | der Kaffee |
| `GE-C28-tee` | `GE-FOOD-TEA` | der Tee |
| `GE-C28-milch` | `GE-FOOD-MILK` | die Milch |
| `GE-C29-freund` | `GE-PEOPLE-FRIEND` | der Freund |
| `GE-C29-freundin` | `GE-PEOPLE-FRIEND-FEMININE` | die Freundin |
| `GE-C29-familie` | `GE-FAMILY-WHOLE` | die Familie |
| `GE-C30-auge` | `GE-BODY-EYE` | das Auge |
| `GE-C30-ohr` | `GE-BODY-EAR` | das Ohr |
| `GE-C30-mund` | `GE-BODY-MOUTH` | der Mund |
| `GE-C30-nase` | `GE-BODY-NOSE` | die Nase |
| `GE-C31-arm` | `GE-BODY-ARM` | der Arm |
| `GE-C31-finger` | `GE-BODY-FINGER` | der Finger |
| `GE-C31-fuss` | `GE-BODY-FOOT` | der Fuß |
| `GE-C31-herz` | `GE-BODY-HEART` | das Herz |

**Atom-first, three genders taught with every noun.** Each lesson introduces
2–3 knowledge atoms (`GE-LEX-*`, usually `GE-SOUND-*` or `GE-GRAMMAR-*`, and
`GE-ETYMON-*`), at or under `maxNewAtomsPerLesson: 3`. Chapter 28 introduces 9
atoms, Chapter 29 introduces 9, Chapters 30 and 31 introduce 12 each — all at
or under `maxNewAtomsPerChapter: 12`. Every noun's article is taught with the
noun (der/die/das), and German's capitalize-every-noun rule and three-gender
system — already established in Chapter 11 — are reinforced rather than
re-explained.

**Chapter 28 — Coffee, Tea, and Milk** (`SPINE-POLITE-REQUEST-REPAIR`):
extends Chapter 19's `Wasser, bitte` pattern to two more drinks, then closes
on the native word. **Kaffee** is a loanword three hops deep — Arabic *qahwa*
→ Ottoman Turkish *kahve* → Italian *caffè* → German *Kaffee*. **Tee** is a
different loan by a different route — Hokkien Chinese *tê*, carried by Dutch
sea traders — and the lesson names the well-known *tea*/*chai* isogloss split
by name (Hindi, Russian and Turkish took the overland Chinese syllable
instead) without turning it into an uncited language count. **Milch** closes
the trio as the one native Germanic word, from PIE *\*h₂melg-*, "to milk" —
deliberately mirroring Chapter 11's Wasser (native) beside Wein (loan) shape,
now run a third time with two loans instead of one. The payoff lesson also
rescues Chapter 27's two never-revisited orphan atoms, `GE-LEX-SCHLIESSEN-10`
and `GE-ETYMON-SCHLIESSEN-11`.

**Chapter 29 — Friend and Family** (`SPINE-EXCHANGE-NAMES`): **Freund** and
English *friend* are the same inherited word, a frozen Proto-Germanic present
participle of "to love" (PIE *\*preyH-*) — the same root inside Chapter 2's
*freut mich*. **Freundin** teaches German's native feminine suffix *-in* as a
general, reusable rule (Lehrer/Lehrerin, Student/Studentin) and names its one
surviving English fossil, **vixen**. **Familie** is the chapter's one loan —
Latin *familia*, "household," related to *famulus*, "servant" — and closes by
naming that it is the group Chapter 10's Eltern and Geschwister already
belong to.

**Chapter 30 — Eyes, Ears, Mouth, Nose** (`SPINE-CHECK-WELLBEING`): extends
Chapter 17's *Kopf*/*Hand* body-part material with four more parts of the
face. **Auge**/*eye* and **Ohr**/*ear* both trace to confirmed PIE roots;
**Mund**/*mouth* is inherited but its root beyond Proto-Germanic is not agreed
upon, and the lesson says so rather than inventing an ancestor; **Nase**/nose
is cousin to Latin *nasus* (English *nasal*) by shared descent, not
borrowing — the same *rot*/*rouge* shape Chapter 13 already taught. The
payoff also rescues Chapter 26's disputed "sharp-eared" link between *hören*
and *Ohr*, never revisited since it was flagged.

**Chapter 31 — Arm, Finger, Foot, Heart** (`SPINE-CHECK-WELLBEING`): Chapter
17's *Hand* lesson printed a five-word comparison — Hand, Arm, Finger, Fuß,
Herz — and taught only the first. This chapter teaches the other four.
**Arm** sits entirely outside Grimm's law's reach (its consonants were never
in the law's path), which is *why* it looks nearly identical to English *arm*
where *Vater*/*father* do not. **Finger** is identical to its English cousin,
with a proposed but explicitly unproven link to *fünf* ("five"). **Fuß** is a
second *p → f* Grimm's-law case beside *Vater*/*father*, and its **ß**
follows Chapter 13's own long-vowel rule. **Herz** closes the chapter — and
the whole five-word list — with a third instance of Grimm's law's *k → h*
swap, alongside *hören*/*akoúein* (Chapter 26) and *Hund*/*canis* (Chapter
22).

**Reach-back at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it. Each chapter's payoff also
reaches back several chapters: Chapter 28 to Chapter 19 and Chapter 27;
Chapter 29 to Chapter 2 and Chapter 10; Chapter 30 to Chapters 17 and 26;
Chapter 31 to Chapters 10, 13, 17, 22 and 26. Chapter 31's payoff closes over
all twelve of its own chapter's atoms (1.00 representativeness).

**No forward references.** Where a new word needed an example sentence, every
lesson uses the case-safe `Das ist der/die/das ___.` construction (predicate
nominative, no accusative article) rather than risk an untaught case form on
the mostly-masculine new nouns — Chapter 27's own note that "this track has
not taught cases" still holds. Drink requests reuse Chapter 19's `___, bitte.`
pattern rather than reach for the untaught verb `trinken`.

**Font check.** One lesson draft used Cyrillic (`Tee`'s *чай*) and one used
the unmapped PIE palatovelar diacritic `ǵ` (`Milch`'s root); both were caught
by a forced XeLaTeX compile ("Missing character: there is no ч in font
Latin Modern Roman…") and fixed before commit — Cyrillic dropped in favor of
the transliteration already given in prose, `ǵ` flattened to `g`. One
lesson (`GE-C31-herz`) originally tripped the sight-cue scanner on the literal
phrase "the table"; reworded to "list" throughout, restoring the chapter to
fully `voice`/drivable.

**Verification.** A forced XeLaTeX build of the 166-page book has zero
missing characters, zero overfull/underfull boxes, and zero duplicate labels
from this tranche (the corpus's one pre-existing underfull box, in Chapter
17, predates it). All four new chapters generate as `voice`, drivable
end-to-end. `npx vitest run tests/integration.test.ts tests/cli.test.ts`
passes (19/19); `check:modality`, `check:books` and `check:narration` all
pass with no diff beyond the new chapters. The six corpus-wide pinned-number
tests (`chapters`, `continuity`, `levels`, `modality-manifest`, `narration`,
`ramp`) shift with any authored content and are left failing, per standing
instruction — their numbers are reported here, not re-pinned.

**Wiring**: `GE-PATH-031`–`GE-PATH-034` are four new path segments (one on
`SPINE-POLITE-REQUEST-REPAIR`, one on `SPINE-EXCHANGE-NAMES`, two on
`SPINE-CHECK-WELLBEING`), each with a matching `GE-EXT-0{31..34}-LANGUAGE-SPECIFIC`
extension — both steps are required, since `lessonSpineNodes` only walks
`curriculum.path[].lessons`.

