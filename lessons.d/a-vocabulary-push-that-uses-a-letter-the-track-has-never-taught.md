---
category: Repo policy / workflow reminders
---

# A vocabulary push that uses a letter the track has never taught needs that letter's lesson first

The Arabic B1 planning prototype built five sample words. Two of them,
مؤتمر and رئيس, failed `script-closure` and `letter-anchoring`, because the
track had a letter lesson for every hamza seat except ؤ and ئ. Up to A2 no
headword had needed them, so the gap had never shown.

The fix was to put letter lessons for both into the B1 core
(AR-W285-hamza-on-waw after سؤال, AR-W288-hamza-on-yaa after طائرة). Until
the core was in, the drafters built with `AR_B1_CORE_LETTERS=0`, which made
the builder refuse ؤ and ئ.

What to do differently: before briefing a vocabulary push, list the letters
the track's letter lessons teach, compare it with the script's full letter
set, and schedule any missing letter lesson in the core, next to the first
word that needs it. A builder flag that refuses untaught letters keeps
drafters from depending on a lesson that does not exist yet.
