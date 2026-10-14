## Every Punjabi word lesson now says where its word comes from

Every one of the 623 word lessons now has an `etymology_hook` and states its
word's origin in its body. Before this change, 549 lessons had no hook, 524
did not state an origin, and 31 had an empty "taken apart" section.

- **What the lessons gained.** 524 lessons gained a "## The word, taken apart"
  block (or "The phrase, taken apart") after the section that introduces the
  word. Each block carries an `hl-knowledge` marker that introduces and assesses
  nothing. The 31 empty sections gained one sentence each, which states the
  origin. 21 lessons that already stated their origin gained a hook. Every hook
  is at most 120 characters and agrees with its block.
- **Inherited words and Sanskrit loans** follow Turner's *Comparative
  Dictionary of the Indo-Aryan Languages* (CDIAL). There are 810 citations to
  373 entries across 399 lessons, and a script checked each one against the
  entry's Punjabi, Old Punjabi or Lahnda rows. CDIAL's own probability and
  uncertainty notes are kept, as are its Dravidian, Munda and Austroasiatic
  notes. So are its notes on direction. Some words Punjabi took from Hindi:
  *rasoī*, *paisā*, *patlā*, *das*, *bhāṁḍā* and *piāsā*. Others Hindi took from
  Punjabi: *kaṅghī*, *gaddā*, *kambal*, *khaṭṭā*, *pakkā*, *kaccā* and
  *chuṭṭī*.
- **Loans.** Arabic loans name their root in Lane's *Arabic-English Lexicon*
  and say they came through Persian. Persian loans are "usually traced to"
  Persian. English and Portuguese loans name the donor word.
- **Uncertain origins.** 29 origins are stated as not settled, most of them
  words CDIAL does not list. Where CDIAL lists a close Punjabi form, the block
  names it and its entry number. Examples are *phakaṛṇā* beside *phaṛnā*,
  *cakkṇā* beside *cukkṇā*, *bhaũh* beside *bharavaṭṭā*, and *ihā* beside
  *ajihā*. Three words keep both candidate sources open:
  - *andar*: CDIAL lists it under Sanskrit *antara*, but Hindi *andar* is
    usually traced to Persian.
  - *javān*: CDIAL lists it under a reconstructed *\*yuvāna*, and the rival
    source is Persian *javān*.
  - *bachchā*: CDIAL lists it under Sanskrit *apatya*, and its addenda set the
    word beside Persian *bachcha*.
- **Entries matched to CDIAL rather than the textbook guess.** *Dūjā* is under
  *\*dutīya* (6402), *socṇā* is under *śocyate* (12621), *hassṇā* is under
  *\*hasyate* (14048) and *gāuṇā* is under *gāpayati* (4135).
- **Durations.** Three `max_seconds` values rose to their computed length, all
  under 300 s:
  - `PA-C02-tuhada-naam-ki-hai`: 170 → 184.
  - `PA-C35-changa-ji`: 270 → 275.
  - `PA-C44-duja`: 230 → 233.

  Every other edited lesson still fits its existing limit.
