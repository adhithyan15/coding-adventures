## Every Japanese word lesson now says where its word comes from

Every one of the 695 word lessons now has an `etymology_hook` and states its
word's origin in its body. Before this change, 601 lessons had no hook, 688
did not state an origin, and 2 had an empty "taken apart" section.

- **What the lessons gained.** 689 lessons gained a "## The word, taken apart"
  block before the practice that follows the word's introduction. Each block
  carries an `hl-knowledge` marker that introduces and assesses nothing. The 2
  empty sections (*iie*, *arigatō*) each gained an origin sentence. 4 lessons
  whose text already named an origin (*hai*, *nihongo*, *kōhī*, *yon*) gained
  a corrected hook. Every new hook is at most 120 characters and agrees with
  its block.
- **Hooks replaced.** 94 lessons already had a hook. 91 were replaced: most
  described a sign, a sound, a counting rule or a grammar point, and the rest
  were origin notes rewritten to agree with the new block. The hooks of
  *arigatō*, *konnichiwa* and *ocha* were kept; each already states the
  word's origin correctly.
- **Native words (519).** Each is called inherited rather than borrowed. Its
  kanji is described as a later kun'yomi, as ateji, or as a spelling chosen
  for meaning and read as a whole, following JMdict's tags and KANJIDIC2's
  readings. Compounds are taken apart into their native parts. Deeper origins
  are never asserted: a single native root says its older origin is not
  settled.
- **Sino-Japanese words (136).** Each character is named by its meaning,
  checked against Unihan (or KANJIDIC2 where Unihan's short definition lacks
  the sense), and by its Middle Chinese reading in Baxter–Sagart's notation,
  derived from the Qieyun categories. A few later Chinese readings (*su* in
  *isu* and *sensu*, *sui* in *suika*, *ton* in *futon*) are named as later
  readings. No word is called a Japanese coinage, because no source at hand
  says so.
- **Mixed words (28)** name each part's layer: *yūhan*, *maiasa*,
  *daidokoro*, *kippu*, *doyōbi* and others.
- **Loanwords (3).** *Kōhī* and *kan* name the Dutch and English donors that
  JMdict gives. *Tenpura* names JMdict's two Portuguese candidates, *tempero*
  and *têmporas*, and says which one is the source is not settled.
- **Not settled (9).** *Hai*, *uma*, *ume*, *semi*, *kaban*, *udon*, *sei*,
  *ninniku* and *kotatsu* say their origin is not settled, with the usual
  account named as an account. *Uma* and *ume* are kun'yomi that are usually
  explained as very early loans from Chinese.
- **Folk etymology refuted.** *Arigatō* now says it does not come from
  Portuguese *obrigado*.
- **Durations.** No `max_seconds` value was raised. *JA-C18-tsu-counter* came
  within 7 seconds of the 300-second ceiling, so one sentence in its "You'll
  Want to Know" section was shortened instead.
- **Script.** Every new block writes Japanese only in the lesson's own
  headword or in signs already taught; everything else is romanized. Script
  closure and forward references are unchanged.
- **Book contents.** The book now runs past page 999, so the preamble widens
  the contents page-number box to 2.4em and its title clearance to 3.4em, and
  gives wrapped chapter titles a ragged right edge, as the Punjabi book does.
