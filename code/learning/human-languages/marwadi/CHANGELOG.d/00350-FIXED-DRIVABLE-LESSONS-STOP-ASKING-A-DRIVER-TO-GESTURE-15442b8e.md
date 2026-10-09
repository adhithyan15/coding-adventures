## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing at pictures, people and printed signs, touching a name card, tapping
beats, a hand held out, and a hand in front of the mouth (issue #12070, ninth
pass). Each ask is now said for the ear and voice where that keeps the
learning goal, or moved into a cue the narration defers (`[YOU POINT: …]`,
`[YOU READ: …]`: "once you have stopped driving — …"). The new gesture check
in human-language-data demands zero such spoken cues in drivable lessons.
Every edited lesson stays `drivable: true` (only its `core/lesson-modality`
source hash changes).

- **Count:** 1 spoken cue and 31 prose instructions in 27 drivable lessons.
- Pointing at a meaning becomes saying it: MW-C05-hear-mharo, MW-C05-hear-
  naam, MW-C05-hear-tharo ("Say *my* when you hear *mhāro*", "Say *your* …"),
  MW-C08-hear-baap, MW-C08-hear-bahan, MW-C08-hear-maa ("Say *mother* or
  *father* …"), MW-C05-answer ("say the name the speaker gives"), MW-C04-hear-
  paani ("say which cup it names"), MW-C03-listen-say ("answer *formal thanks*
  or *respectful yes* for each").
- Pointing at a picture becomes picturing it: MW-C16-hear-bas, MW-C16-hear-
  thela, MW-C17-hear-ghodo, MW-C17-hear-riksha, MW-C18-hear-daal, MW-C18-hear-
  sabji.
- Pointing at printed script cannot be done by ear, so it moves into a cue the
  narration defers: MW-C05-hear-kain (two `[YOU POINT: …]` cues), MW-C05-kain
  and MW-C35-read-ten-ticket (`[YOU POINT: …]`), MW-C24-final-four (`[YOU
  READ: both printed lines, then point to …]`).
- Beats: MW-C05-hear-hai "Tap four beats" → "Count four beats aloud";
  MW-C07-hear-later "tap the two groups" → "say the two groups apart, then say
  them once together".
- Hands: MW-C21-hear-show-request and MW-R24-pay-two drop "with your hand out"
  and keep the scene ("as if the cloth were on the counter", "as if the money
  were in your hand"); MW-C22-hear-price-question "with your hand on the
  thing" → "as if the thing … were in front of you"; MW-C24-hear-do "with the
  matching gesture" → "picturing the matching hand each time: taking for *lo*,
  giving for *do*"; MW-C21-hear-show-request "Point first, name the thing
  second, ask last" (word order, read as a request) → "The pointing word
  first, the thing second, the ask last".
- Aspiration: MW-C32-hear-chaar "Put a hand in front of your mouth and say it"
  and MW-C33-hear-chha "Hand at the mouth again" → "listen for the breath".
  Judgement call: the hand test is the clearest way to feel a puff, but the
  contrast it shows is audible, which is what a driver can use; the non-
  drivable writing lessons keep the hand test. MW-C33-hear-chha's recall "the
  plain *ch* you tested with a hand at your mouth" was a memory, not a
  request, but the check reads any hand-at-the-mouth phrase in a spoken cue,
  so it now recalls "its plain *ch*, the one with no puff of air".
