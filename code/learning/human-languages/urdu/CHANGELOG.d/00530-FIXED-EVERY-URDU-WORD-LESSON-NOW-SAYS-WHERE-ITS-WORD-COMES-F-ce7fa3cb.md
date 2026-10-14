## Fixed — every Urdu word lesson now says where its word comes from

Every one of the 1,218 word lessons now has an `etymology_hook` and states its
word's origin in its body. Before this change, 1,123 lessons had no hook, 1,137
did not state an origin, and one (UR-C04-thik) had an empty "taken apart"
section.

- **What the lessons gained.** 1,139 lessons gained a "## The word, taken
  apart" block (or "The phrase, taken apart") after the section that introduces
  the word. Each block carries an `hl-knowledge` marker that introduces and
  assesses nothing. 58 existing sections were corrected or gained an origin
  sentence, among them UR-C04-thik's empty one, which now names *\*ṭhīkka*
  (CDIAL 5503). 19 lessons that already stated their origin gained a hook only.
  Every new hook is at most 120 characters and agrees with its block; 77 of
  the 79 hand-written lessons swapped a long or non-origin hook (for example
  "Urdu pairs respectful jī with hā̃…") for a short origin hook. The hooks of
  UR-C01-salam and UR-C01-shukriya were already origin hooks and stay.
- **Inherited words and Sanskrit loans** follow Turner's *Comparative
  Dictionary of the Indo-Aryan Languages* (CDIAL): 695 citations to 534
  entries across 646 lessons, each checked by script against the entry's
  headword, its numbered forms and its Hindi rows. CDIAL's probability and
  uncertainty notes are kept, as are its Dravidian and Munda notes and its notes
  on direction (*kachchā* and *pakkā* as forms taken from Punjabi; *bonā* as
  one taken from Gujarati). Three Dravidian comparisons cite DEDR.
- **Same origin as the Hindi lessons.** Where the Hindi track teaches the same
  word, the Urdu block gives the same origin, CDIAL entry, hedges and root
  slug. It differs only where CDIAL files the word under another form of the
  entry, or where a hedge was missing: *paṛosī* (*\*prativeśiya*, 8599),
  *uṛnā* (*uḍḍayate*, 1697), *bāp* (*\*bāppa*, 9209), *moṛ* (*mōṭati*, 10186),
  *mirch* (*\*maricca*, 9875), *pehle* and *pehlā* (*\*prathilla*, 8652),
  *bhāgnā* (*bhagna*, 9361), *lomṛī* (*\*lompaṭa*, 11153), *ghuṭnā*
  (*\*ghuṭṭa*, 4479), *sāth* (*sārthena*, 13364), *nayā* (*naviya*, 7025),
  *ghar* (CDIAL's "phonetically difficult"), *sochnā* (*śocyate*, 12621),
  *khaṛā* (*khaḍaka*, 3784), *joṛnā* (*\*yoṭayati*, 10496), *sambhālnā*
  (*saṃbharati*, 12961), *ulṭī* (*\*ullaṭyate*, 2368), *nal* (*naḍaka*, 6936),
  and *nāī*, whose companion *nāū* sits under *snāpaka* (13792).
- **Loans.** Arabic loans name their root in Lane's *Arabic-English Lexicon*
  (179 lessons) and say they came through Persian. Persian loans are "usually
  traced to" Persian. English and Portuguese donors are named, and four words
  are traced through Persian to Turkic (*qainchī*, *qormā*, *qīmā*; *bahādur*
  to a Turkic or Mongol word).
  Compounds and conjunct verbs (*qarz denā*, *dastkhat karnā*, *khush qismatī
  se*) are taken apart word by word.
- **Uncertain origins.** 47 origins are stated as not settled, most of them
  words CDIAL does not list (*batānā*, *sastā*, *-vālā*, *idhar*, *kidhar*,
  *lauṭnā*, *chaṭnī*, *palak*, *razāī*, *gilahrī*, *chapātī*, *kaṛhī* and
  others). Where CDIAL lists a near form, the block names it.
- **Older blocks corrected.** UR-C06-ana taught *ānā* as *ā-* "toward" plus
  *yā-* "go"; CDIAL lists no Hindi or Urdu word under *āyāti* (1288) and puts
  *ānā* under *āpayati*, "causes to reach" (1200), as the Hindi lesson does. The
  block, its practice and recall lines, its gloss and the A1 exam-inventory note
  now say so. UR-C17-yih and UR-C18-voh now cite *eṣa* (2530) and *asau* (972)
  instead of *iyam*; UR-C07-sochna cites *śocyate*; UR-C10-naak keeps CDIAL's
  "possibly" for *\*nas-ka-*; UR-C12-roti, UR-C14-topi and UR-C15-dhoop name the
  forms CDIAL reconstructs (*\*roṭṭa*, *\*ṭoppa*, *\*dhuppā*); UR-C09-bahan
  reports CDIAL's account of *bhaginī*; UR-C15-barish builds *bārish* on the stem
  of *bāridan*, "to rain", not on *bār*, "load"; UR-C17-kahan marks
  *kasmin* as not settled; the numbers cite *dva*, *trīṇi*, *\*ekka* and
  *\*kṣaṭ* where CDIAL lists them.
- **Root slugs.** UR-C06-jana and UR-C06-ana no longer share the slug
  `sanskrit-ya`: *jānā* uses `sanskrit-yati` and *ānā* `sanskrit-apayati`.
  UR-C29-nau uses `sanskrit-nava-nine`, UR-C17-yih `sanskrit-esha`.
  `check:root-slug-splits` reports 77 known entries, none new.
- **Durations.** No `max_seconds` value was raised. Four lessons whose new
  block would have pushed the computed length to 300 s or more were tightened
  instead (UR-C07-parhna, UR-C22-kya-kaha, UR-C24-vahan, UR-C26-hai-na); every
  lesson's computed length is at most 299 s.
