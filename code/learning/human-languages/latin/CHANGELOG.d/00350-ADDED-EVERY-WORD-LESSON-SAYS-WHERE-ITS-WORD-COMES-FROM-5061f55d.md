## Added — every word lesson says where its word comes from

The 1,090 generated word lessons (LA-C63-bracchium through LA-C280-vallum, in sequence order) now carry both an `etymology_hook` and a visible `## The word, taken apart` block (`## The phrase, taken apart` for LA-C195-dies-natalis), placed before "What you've built". The question each block answers is where the Latin word itself comes from. Romance descendants appear only where the lesson already named them. The five Chapter 1 greetings already had a taken-apart block and gained the hook they lacked. Fifteen hand-written lessons that had only a hook gained a short taken-apart block before Guided Practice (human-language-data: Latin etymology gaps 1095 / 1105 → 0 / 0).

- **Sources.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC; CC BY-SA 4.0), checked row by row, and Lewis & Short, *A Latin Dictionary* (Perseus Digital Library `lexica`, CC BY-SA 4.0). Both were read locally and nothing is vendored. Every line is our own sentence, and only single words and glosses are taken from either source. A source tag stands only after a fact that source's entry for that word holds, and a script checked every italic form before each tag against the EtymDB tree or the Lewis & Short article. 559 lessons cite EtymDB 2.1 and 777 cite Lewis & Short.
- **Answers.** Of the 1,090: 287 inherited from Proto-Indo-European or Proto-Italic, 67 borrowed from Greek, 9 borrowed from another language (Celtic *gladius*, *cervīsia* and *raeda*; Sabellic *bōs*, *lupus* and *popīna*; Punic *mappa*; Germanic *sāpō*; Etruscan *histriō*), 544 built inside Latin from Latin parts, and 183 with no sure origin. The uncertain ones say so and give the usual account as a hedge where there is one. Etruscan and Mediterranean-substrate proposals are always hedged.
- **Reconstructions.** Every Proto-Indo-European and Proto-Italic form is starred and sits in a hedged sentence ("usually traced", "probably"). Forms follow EtymDB's spelling, written in the book font's letters: *\*h2*, *\*kw*, *\*gh*, with Greek romanised. Latin-internal derivation (*victōria* from *victor*) is kept apart from inheritance (*novus* from *\*newos*). Greek source words are named and glossed.
- **Rows checked, not copied.** 54 EtymDB rows or Lewis & Short claims were rejected or relabelled, each logged with a reason:
  - homographs: *īnfāns* given the *in-* "in", *pila* "ball" given the root of *pīla* "mortar", *os* "bone" given *ōs* "mouth", *labrum* "lip" given *lavō*, *saliō* given *sāl*, *levō* given *lēvis*, *malō* given *malus*;
  - "inherited" Greek loans relabelled as borrowings: *bracchium*, *oleum*, *olīva*, *poēta*, *spēlunca*, *elephantus*;
  - *bōs* and *lupus* relabelled as Sabellic loans;
  - junk suffix rows and stray links, such as *līberī* linked to an English word or *apis* to the Egyptian god Apis;
  - Lewis & Short comparisons now abandoned, left out: *marītus* from *mās*, *patina* from *pateō*, a Celtic *tolerō*.
- **No forward references.** Every Latin word a block names, other than the headword's own forms and older spellings, carries an English gloss, checked by script. A word the learner has not met yet is never left bare.
- **Roots.** All 1,090 lessons gained `roots:`, led by the word's own lemma. 578 of the 1,344 slugs already existed in the corpus (`agere-latin`, `tantus`, `heri`), and 766 are new `<lemma>-latin` slugs. Latin *nam* "for" is `latin-nam-for`, because bare `nam` is Hindi *nām* "name" and `nam-latin` would split against it. `check:root-slug-splits` reports no new split.
- **Hand-written lessons.**
  - Hooks were added to LA-C01-salve, -ave, -vale, -gratias and -ita-non. LA-C01-ave's block now adds that many scholars think *avē* began as a Punic greeting, which its hook also says.
  - Taken-apart blocks were added to LA-C05-dies-lunae-veneris, LA-C06-niger-albus, LA-C12-medius-dies-nox, LA-C15-tempestas-pluit, LA-C15-weather-verbs, LA-C16-undecim-viginti, LA-C19-quid-agis, LA-C19-ut-vales, LA-C19-valeo-family, LA-C20-quid-tibi-nomen, LA-C21-volup-est-convenisse, LA-C22-tu-vos, LA-C30-bonam-noctem, LA-C34-bonum-vesperum and LA-C36-salve-post-meridiem.
  - None repeats what an earlier etymology block already gives. *nox* and *vesper* keep their own lessons' accounts, and *merīdiēs* keeps LA-C35's.
- **Durations.** The generated lessons compute at 76–127 s against their declared 200. Eleven hand-written lessons declare more time (`max_seconds`), each raised to its computed length and all under 300 s:
  - LA-C05-dies-lunae-veneris 240 → 278
  - LA-C06-niger-albus 240 → 291
  - LA-C12-medius-dies-nox 240 → 270
  - LA-C16-undecim-viginti 240 → 252
  - LA-C19-quid-agis 120 → 205
  - LA-C19-ut-vales 120 → 221
  - LA-C20-quid-tibi-nomen 240 → 252
  - LA-C22-tu-vos 240 → 279
  - LA-C30-bonam-noctem 240 → 279
  - LA-C34-bonum-vesperum 240 → 271
  - LA-C36-salve-post-meridiem 240 → 255

  Seven of the eleven (C06, C12, both C19 lessons, C22, C30, C34) already computed above their declared time before this change.
- **Review corrections.** An independent review read all 1,110 entries against EtymDB 2.1, Lewis & Short and the standard handbooks (de Vaan, Walde-Hofmann, Ernout-Meillet). It found 11 substantive errors (1.0%) and fixed them. No loan was found with the wrong donor.
  - Reconstructions and glosses: *pluō*'s root *\*plew-* is "to flow, to float", not EtymDB's "falls like rain". Proto-Italic *\*walēō* means "to be strong". *unguis* is from *\*h3nogwh-* (the *gu* needs the w). The garbled *\*ker* is dropped from *cor*'s hook. The unattested *gnōria*, *propiter* and *sentientia* are now starred.
  - Sources: *ausum* is Sabine, not Old Latin (Festus, via Lewis & Short; LA-C222-aurum, LA-C149-aureus). *raeda* keeps Lewis & Short's "probably" Celtic. EtymDB only records *templum*'s *\*temp-*; it does not "prefer" it.
  - Derivations: *discrīmen* is built on the root *crī-*, not contracted from *discerimen*. *pūblicus* comes from the attested *poplicus*.
  - Hedges: these are now "perhaps" or "often": *frōns*, *venter*, *passer*, *ēbrius*, *rēn*'s hook, and *porrum* (possibly a loan). *volup* now gives Lewis & Short's own comparison, Greek *elpis*, in place of an unsourced "probably from the root of *volō*".
  - Durations: five hand-written blocks were tightened, and their declared time lowered to the new computed length: C06 291 → 286, C12 270 → 265, C19-quid-agis 205 → 193, C19-ut-vales 221 → 216, C36 255 → 249.
  - Roots: three slugs merged homographs. `os-latin` is *os* "bone", so *ōs* "mouth" in cōram, ōrō and ōsculor is now `os-mouth-latin`. `parere` is *pāreō*, so *pariō* in parēns and pauper is now `parere-bear-latin`. `cum-latin` is *cum* "with", so quondam's *quom* is now `qui-latin`. Two more gave one word two senses: *serō* "join" in praesertim is now `serere-join-latin`, and avārus (*aveō* "crave") is now `avidus-latin`, not avē's `avere`. Five new slugs split an etymon the Romance tracks spell otherwise, and now use those spellings: `scala-latin`, `paucus-latin`, `viduus-latin`, `partire-latin`, and `subitus-latin` (added).

