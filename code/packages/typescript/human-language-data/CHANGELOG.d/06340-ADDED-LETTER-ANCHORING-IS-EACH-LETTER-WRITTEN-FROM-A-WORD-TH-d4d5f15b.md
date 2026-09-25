### Added — letter anchoring: is each letter written from a word the reader knows? (HL-C443)

`measureLetterAnchoring` (`src/letter-anchoring.ts`) measures HL-C443's
gentle-writing rule on every non-Latin track: teach a letter from a word the
reader already knows, and eventually teach every letter they read.

Each letter lesson is exactly one of four kinds. A letter lesson is either a
lesson that gets a filmstrip, or a letter set like "வ, க" or "௧ ௨ ௩".

- **anchored:** an earlier word headword holds the letter.
- **builds-toward:** the word comes later in the same chapter.
- **cold:** no word holds it.
- **unmeasured:** a Han component, which needs decomposition data this
  package does not have.

Each track also lists **unwritten** letters: glyphs read in a word headword
that no letter lesson writes.

`tests/letter-anchoring.test.ts` pins a per-track ceiling on cold,
builds-toward and unwritten. The ceilings are a ratchet: they may fall and
must not rise.

Measured today there are 845 letter lessons:

| Kind | Count |
| --- | --- |
| anchored | 490 |
| builds-toward | 190 |
| cold | 156 |
| unmeasured | 9 |

There are also 93 unwritten letters, mostly Arabic 18, Persian 18, Urdu 16 and
Tamil 14.

`letterBlockIndex` is now exported from `figure-targets.ts`, so the measure
recognises a letter set by the same Writing or Script block rule.
