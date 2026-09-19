import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  measureExamCoverage,
  formatExamCoverage,
} from "../../src/exam-inventory.js";
import { FIXTURE, fixtureLesson as lesson } from "../exam-inventory-fixture.js";

describe("the committed A1 inventory", () => {
  const inventory = loadExamInventory("spanish", "A1");

  it("refuses an empty probe, because an empty probe scores as covered", () => {
    // `probe: []` asks for zero atoms, every one of which is trivially present,
    // so the point would be reported covered while demonstrating nothing. It is
    // the one malformed shape that moves the number in the flattering direction,
    // which is why the loader rejects it rather than tolerating it.
    for (const point of inventory.points) {
      expect(Array.isArray(point.probe) ? point.probe.length : 1).toBeGreaterThan(0);
    }
  });

  it("names every probe atom in the corpus convention, so a typo is visible", () => {
    // A misspelt atom resolves to "not introduced", which is fail-safe but
    // silent. Shape-checking the name catches the common half of that.
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) {
        expect(atom, `${point.id} probes '${atom}'`).toMatch(/^ES-[A-Z0-9-]+$/);
      }
    }
  });

  it("classifies every unmapped point, so a new null cannot slip in unnoticed", () => {
    // This test used to hold a hand-written list of nulls that were PARTLY true
    // and therefore needed a note saying which half existed. Chapters 257-261
    // closed the last of those, and emptying the list left `for (const id of
    // [])` behind — a loop that never runs, in a test with no other assertion,
    // passing unconditionally. The invariant it existed for was enforced by
    // nothing, and a future null probe with no note would have sailed through.
    //
    // So it is derived now instead of listed. Pinning the exact set of nulls
    // means any NEW one fails here and has to be classified deliberately —
    // which is the whole job this test was meant to do.
    const unmapped = inventory.points.filter((point) => point.probe === null).map((point) => point.id);
    // This list was EMPTY while the file enumerated grammar and nothing else.
    // Enumerating the PCIC functional inventory, the general and specific
    // notions, and the orthography inventory added 188 points, and 50 of them
    // have no corresponding atom anywhere in the corpus. That is the finding,
    // not a defect: an unmapped point is uncovered and reported by name, never
    // skipped. Every entry below carries a `note` naming the source exponent
    // that is missing, and the loop underneath proves the note is there.
    //
    // 50 -> 48. `A1-F2-16` and `A1-F2-17` — ask about and express ability —
    // leave this list because chapter 389 teaches `saber`, which is the exponent
    // the PCIC actually asks for. They were NOT closed by pointing them at
    // `poder`: both notes said in as many words that the source asks for *saber*
    // plus an infinitive and that substituting `poder` would be a different
    // structure, so closing them honestly meant authoring the verb the syllabus
    // names. HL23 §10 minted `SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO` for these two
    // points, which is why that rung is justified by this inventory rather than
    // invented to give two verbs somewhere to live.
    //
    // Read as a map of the real gaps: the F-* entries are speech acts the book
    // never performs (the affirmative imperative, toasting, congratulating);
    // the NE-* entries are whole A1 domains with no lesson
    // at all (clothing, cinema and music, the internet and dictating an
    // address, police and fire); and the O-* entries are nearly the entire
    // orthography inventory -- the alphabet, capitalisation, and every
    // punctuation mark except the question and exclamation pair.
    // 48 -> 44. THE WHOLE `Nociones evaluativas` GAP CLOSES AT ONCE, and three of
    // the four are the reason `SPINE-DESCRIBE-QUALITIES` exists at all.
    //
    // `A1-NG6-03` (attractiveness), `A1-NG6-08` (interest) and `A1-NG6-10` (ease and
    // difficulty) leave this list because chapters 397-399 author the exponents the
    // PCIC actually names — guapo, feo and bonito; interesante; fácil and difícil.
    // HL23 §12.2 justified the qualities rung BY these three points, and a
    // justification that leaves the points unmapped is a justification used as an
    // excuse. They close the same way `A1-F2-16`/`A1-F2-17` did: by authoring the
    // source's own exponent, never by pointing the point at a word already present.
    //
    // `A1-NG6-09` (capacity and competence with saber) is DIFFERENT, and it is a bug
    // this slice found rather than work it did. Its note read "the corpus never
    // introduces saber as a verb, only the fixed phrase no se" — which stopped being
    // true when chapter 389 authored `saber` for the two points named above. The atom
    // it needs, `ES-LEX-SABER`, has existed since #13154 and nothing pointed at it.
    // A null whose stated reason has expired is worse than a bare null, because the
    // note is precisely what the loop below trusts to prove the null was considered.
    // 44 -> 41. THE THREE POINTS CHAPTER 420 REFUSED TO CLAIM, plus the one whose
    // note had gone stale. `A1-NE18-06` (cinema and theatre) was held null on
    // purpose while actor and actriz were absent; chapters 421-423 author them,
    // so it leaves the list by the only route this project accepts. `A1-NE16-02`
    // stood uncovered on ONE missing exponent -- three of its four had been
    // taught since chapter 393 -- and `pagina web` supplies it.
    //
    // `A1-NE02-01` is the HL-C376 class again, and worse than a bare null: its
    // note said "the corpus introduces none of them" while `simpatico`
    // (ES-C380-simpatico) and `alegre` (ES-C380-alegre) were both headwords. The
    // note is corrected in the same change that authors the remaining six, so
    // the point does not close on a correction -- it closes on six lessons.
    // 41 -> 33. THE WHOLE `Puntuacion` GAP CLOSES AT ONCE, which is the second
    // time a complete category has moved in one chapter and the first time it
    // was the emptiest one. The comment below named it as the work queue at
    // 1/9; chapter 424 takes it to 9/9.
    //
    // These eight are a different KIND of gap from every vocabulary point this
    // campaign has closed. The corpus has PRINTED all eight marks for hundreds
    // of chapters and named none of them: the dialogue `raya` opens every
    // exchange the book has ever set, including the three chapters merged
    // immediately before this one, and the reader was left to decode it by
    // guess. So nothing here is a new word — it is the book finally saying what
    // its own typography has been doing.
    // 33 -> 18. THE ORTHOGRAPHY DIMENSION CLOSES ENTIRELY. Chapters 425 and 426
    // take `Ortografia de letras y palabras` 0/7 -> 7/7 and `Abreviaturas y
    // siglas` 0/3 -> 3/3, and with `Puntuacion` closed one chapter earlier,
    // every O-* point in the inventory is now covered.
    //
    // The eighteen that remain are ALL notions and functions -- no grammar, no
    // orthography. That is a change in the SHAPE of the remaining work, not
    // only its size: what is left needs vocabulary and exponents rather than
    // rules, so the next tranches will look like chapters 421-423 again rather
    // than like 424-426.
    //
    // A1-O1-04 (capitals) is the one of the ten with a consequence a marker sees
    // on every line rather than a fact a reader can look up: an English-trained
    // hand capitalises `lunes`, `enero`, `espanol` and every word of a book
    // title, and Spanish capitalises none of them.
    // 18 -> 11, and THREE of the seven cost no authoring at all. `A1-NE13-03`
    // (farmacia), `A1-NE15-04` (pescado) and `A1-NE07-06` (ser trabajador) each
    // enumerate ONE exponent, and each of those three was already a lesson
    // headword -- ES-C353, ES-C361 and ES-C423 respectively. Their notes said
    // the corpus never introduced them, and all three notes were false.
    //
    // This is the HL-C375 class, found again by the HL-C376 method: resolve
    // every exponent through a lesson `headword:`, never through an atom id.
    // Two of the three had been wrong for many chapters; the third went stale
    // the moment chapter 423 landed `trabajador` and nothing re-read the note.
    // A null whose stated reason has expired is worse than a bare null, because
    // the note is exactly what the loop below trusts to prove the null was
    // considered.
    //
    // The other four are chapter 427's, and they are ordinary authoring.
    // 11 -> 7, and every one of chapter 428's five words was the LAST MISSING
    // PIECE of something the corpus could otherwise nearly do.
    //
    // `A1-NE09-06` needed two symbol names out of seven exponents: punto, guion
    // and pagina web had joined internet and correo electronico across chapters
    // 342, 422 and 424, and the note had not caught up -- the SECOND stale note
    // found in two tranches by the HL-C376 headword method.
    //
    // `A1-F5-10` and `A1-F5-09` are a shape worth naming: `feliz` and `la salud`
    // were BOTH already taught, and neither covered its point, because an
    // adjective does not congratulate and a noun does not toast. The fix is the
    // same word put to a different use -- felicidad pluralised into an act, and
    // salud with its article dropped. Coverage is about what the corpus can DO,
    // not only which strings it contains.
    //
    // The seven that remain are the residue, and two of them are meant to stay
    // null: `A1-F2-10` and `A1-F6-06` argue in their own notes that A1 has no
    // linguistic exponent to probe, and wiring either would be the over-claim
    // this campaign has refused since `A1-NE18-06`.
    // 7 -> 2. THE INVENTORY REACHES ITS CEILING, and the shape of the last five
    // is the finding: THREE of them were stale notes rather than gaps.
    //
    // `A1-F4-01` (give an order) closes with ZERO authoring. Its note said the
    // corpus "never introduces the affirmative imperative" while ES-C50-habla
    // teaches habla/come/vive and ES-C50-ocho-cortos teaches the eight
    // irregulars. The inventory was contradicting ITSELF: `A1-V-11` had been
    // wired to those same two atoms all along. The softened-order exponent is
    // `por favor` plus ES-C50-sintesis-pedir-bien, which reads *Coma, por favor*
    // and argues the attenuation lives in the tu/usted choice.
    //
    // `A1-NE06-01` and `A1-NE07-04` were PARTLY stale. NE06-01's note claimed
    // the corpus taught "none of the four listed words" while universidad,
    // clase and biblioteca were all headwords -- the gap was ONE word wide, and
    // `instituto` is the one that mattered, being both a false friend and the
    // missing middle rung between escuela and universidad. NE07-04's note
    // claimed neither `paro` nor `trabajo` as a noun existed, and ES-C394-trabajo
    // introduces the noun -- so only the negative half was missing.
    //
    // The other two are ordinary authoring. `A1-NE12-02` was accurate at four of
    // seven and chapter 430 adds vaqueros, jersey and bolso. `A1-NE06-05` is the
    // one whose note was RIGHT: it had already corrected a false claim about
    // `examen` and held the point uncovered because the MARKS half was wholly
    // absent, and chapter 429 supplies all four of nota, calificacion, aprobar
    // and suspender, so it closes on both halves rather than on one.
    //
    // Five stale notes in four tranches, all found by the HL-C376 headword
    // method. The notes are the least-maintained part of the inventory because
    // nothing re-reads them when a chapter lands.
    //
    // The two that remain are meant to remain. 271/273 IS the ceiling.
    expect(unmapped.sort()).toEqual([
      "A1-F2-10", "A1-F6-06",
    ]);

    // Every null must SAY why it is null. The note is what stops a null from
    // reading as "nobody has looked yet": it names the exponent the source asks
    // for and states that the corpus does not introduce it. Without this
    // assertion the list above could grow by a bare null with no reasoning.
    for (const id of unmapped) {
      const point = inventory.points.find((candidate) => candidate.id === id)!;
      expect(point.probe, `${id} must stay null or gain a probe deliberately`).toBeNull();
      expect(point.note?.trim(), `${id} is unmapped and must say why`).toBeTruthy();
    }
  });
});

describe("what the corpus actually covers", () => {
  it("pins A1 coverage, which is the number this project is judged on", () => {
    const lessons = loadTrackLessons("spanish");
    const coverage = measureExamCoverage(loadExamInventory("spanish", "A1"), lessons);
    expect(coverage.inventoryComplete).toBe(false);

    // First measured at 53/85 over 220 chapters — a curriculum that had climbed
    // to a B2 node while missing 62% of... no: while holding only 62% of the A1
    // grammar an examiner may ask for. Chapters 221-225 then taught the
    // demonstratives, which had been absent ENTIRELY (este/ese/aquel and the
    // neuters, 3 of 3 points), taking it to 56/85.
    //
    // Chapters 226-229 then taught the degree words -- `muy`, `bastante`, `mal`
    // -- closing four more points across three categories, and taking
    // `El sintagma adjetival` off the floor at last.
    //
    // Chapters 230-235 then closed the contractions, `quien`, and both missing
    // coordinators -- which finished `Coordinacion` outright.
    //
    // Chapters 236-240 then taught the gerund and the personal `a`. A third
    // point, A1-V-03, was DELIBERATELY left open: chapter 238 teaches the
    // progressive and contrasts it with the plain present, which is adjacent to
    // that point but is not it, and closing it with progressive atoms would be
    // exactly the gaming this gate exists to catch.
    //
    // Chapters 241-245 then paid HL-C127's debt: the vosotros preterite and the
    // imperfect plural, both of which chapter 204 promised the reader in print.
    // Both past tenses are now complete paradigms.
    //
    // Chapters 246-250 then closed the stressed pronouns, the exclamative `que`
    // and the vocative.
    //
    // Chapters 251-255 then finished every set the book had taught only half of:
    // ahi/alli beside aqui, ahora/hoy beside manana, unos/unas beside un/una,
    // vuestro beside nuestro, and the ver/dar preterite. A1-Q-04 closed with
    // no new content at all -- `bastante` was taught at ch227 and its probe had
    // simply never been wired, which is its own kind of measurement error.
    //
    // The eight that remain are a DIFFERENT problem. Four of them -- A1-SN-03,
    // A1-Q-03, A1-N-01, A1-V-03 -- are things the book demonstrates on nearly
    // every page and never states, so they need lessons that make explicit what
    // the reader already does by reflex. The rest are unbuilt structures.
    //
    // This number is allowed to move only two ways. Up, when a lesson teaches
    // something the inventory lists. Down, when one is retired. It must NOT
    // move because a probe was loosened or a point deleted — if this assertion
    // fails alongside an edit to exam-inventory-es-a1.json, read that edit
    // before re-pinning.
    //
    // Chapters 257-261 then closed the last four of the "demonstrated but never
    // stated" points, which is why `partiallyTrue` is now empty: no null probe
    // remains that is PARTLY true. The four that are still null are absent
    // outright -- the ordinals, `uno...otro`, word-order flexibility, and the
    // infinitive as subject -- and need no note to say which half exists.
    // WHY THIS NUMBER JUST FELL FROM 100% TO 82%, AND WHY THAT IS THE FIX
    // Everything above is the history of the GRAMMAR dimension, which reached
    // 85/85. The comment ten lines up says this number must not move because a
    // point was added or a probe loosened, and warns the next reader to read
    // the inventory edit before re-pinning. This is that edit, so here is the
    // reasoning it asks for.
    //
    // The file used to enumerate ONE of the four HL20 content dimensions. It
    // now also enumerates the PCIC functional inventory (54 points), the
    // general and specific notions (36 + 77), and the orthography inventory
    // (21) -- 188 new points, each restated from the A1 column the source
    // publishes separately from A2. 138 of them map to atoms the corpus really
    // introduces; 50 have no corresponding atom and are null.
    //
    //     before: 85/85   = 100%, 0 unmapped
    //     after: 223/273  =  82%, 50 unmapped
    //
    // The denominator grew because the target got honest, not because the book
    // got worse -- no lesson was retired and no probe was loosened, and the
    // grammar dimension is still 85/85 inside the new total. A 100% that
    // measured a quarter of the construct was the flattering failure HL20 was
    // written to close, and 82% of a four-dimension target is a larger true
    // number than 100% of a one-dimension one.
    //
    // This is also why `percent` is pinned exactly rather than as a floor: it
    // is allowed to fall, but only for a reason stated here in prose.
    //
    // 223 -> 225, 50 -> 48 unmapped. HL23 §10 authors `saber` (chapter 389) and
    // maps `A1-F2-16` and `A1-F2-17` onto it. `percent` is unmoved at 82: two
    // points out of 273 is 0.7pp, which rounds away. That is worth saying out
    // loud — a headline percentage that does not move is not evidence that
    // nothing happened, which is exactly why `covered` and `unmapped` are pinned
    // beside it rather than the percentage alone.
    //
    // 225 -> 229, 48 -> 44 unmapped. HL23 §12.2's qualities rung closes the whole
    // `Nociones evaluativas` gap: chapters 397-399 author the source's own exponents
    // for `A1-NG6-03` (guapo, bonito — feo the corpus already had), `A1-NG6-08`
    // (interesante) and `A1-NG6-10` (fácil, difícil). `A1-NG6-09` is the odd one and
    // cost no authoring at all: its note claimed the corpus never introduces `saber`,
    // which stopped being true when chapter 389 authored it for `A1-F2-16`/`A1-F2-17`
    // two slices ago. The atom existed and nothing pointed at it.
    //
    // `percent` MOVES this time, 82 -> 84. Contrast the 223 -> 225 note above, where
    // two points rounded away: four points out of 273 is 1.5pp and survives rounding.
    // Both behaviours are correct and neither is evidence on its own, which is the
    // argument for pinning `covered` and `unmapped` beside it.
    expect(coverage.enumerated).toBe(273); // 85 grammar + 54 functions + 113 notions + 21 orthography
    expect(coverage.covered).toBe(271); // 266 -> 271 (ch429-430): the LAST FIVE enumerated points, and three of the five were stale notes rather than gaps. A1-F4-01 closes with zero authoring -- the affirmative imperative has been taught by ES-C50-habla and ES-C50-ocho-cortos all along, and the inventory contradicted itself because A1-V-11 was already wired to those same atoms. A1-NE06-01 needed only instituto (universidad, clase and biblioteca were headwords), and A1-NE07-04 needed only estar en paro (ES-C394-trabajo introduces the noun). A1-NE12-02 and A1-NE06-05 are ordinary authoring: three garments and the four marks words. 271/273 is the CEILING, not a stopping point chosen for convenience -- A1-F2-10 and A1-F6-06 argue in their own notes that A1 has no linguistic exponent to probe. // 262 -> 266 (ch428): A1-NE09-06 (dictating an e-mail address, which needed only arroba and guion bajo), A1-F5-10 (felicidades), A1-F5-09 (the toast salud) and A1-NE20-05 (the superordinate animal). Two of the four close on a word the corpus ALREADY had, used differently: feliz does not congratulate and la salud does not toast, so the chapter supplies the act rather than the string. // 255 -> 262 (ch427): FOUR points authored -- A1-NE15-02 (vendedor, comprador), A1-NE11-04 (policia, bombero), A1-NE17-02 (abogado) and A1-NE15-03 (empresa) -- and THREE wired at no cost, because farmacia (ES-C353), pescado (ES-C361) and trabajador (ES-C423) were already headwords whose notes claimed they were absent. The chapter is organised by HOW each profession word was built rather than by what the person does, because that is what makes an unfamiliar one readable: -dor on a verb, -ero on a thing, policia from its institution, abogado inherited from Latin whole. // 245 -> 255 (ch425-426): the ENTIRE orthography dimension. Ortografia de letras y palabras 0/7 -> 7/7 and Abreviaturas y siglas 0/3 -> 3/3, on top of Puntuacion closed in ch424. Ten points, and like ch424 not one of them is a vocabulary gap: the letters have been on every page since chapter one and the corpus had never named them. The letter NAMES are the load-bearing part -- a listening paper dictates a surname letter by letter, and hache, jota and equis are not guessable from English. // 237 -> 245 (ch424): the ENTIRE Puntuacion category, 1/9 -> 9/9. Eight marks the corpus printed constantly and never named. Two of the eight carry a consequence a marker can see rather than a fact a reader can look up: Spanish writes NO comma before the final y of a list, where an English-trained hand puts one, and a Spanish letter opens with a colon after the greeting where English uses a comma. // 234 -> 237 (ch421-423): A1-NE18-06 (cinema and theatre), A1-NE16-02 (computing and new technology) and A1-NE02-01 (character and personality adjectives). Nine headwords across three chapters. NE18-06 is the one chapter 420 deliberately left null pending actor and actriz, and it closes here by authoring them rather than by loosening the claim. NE16-02 needed a single exponent, pagina web, against three taught since ch393. NE02-01 needed six of eight, and its note wrongly said the corpus taught none of the eight while simpatico and alegre were headwords -- corrected here, but the point closes on the six new lessons, not on the correction. // 232 -> 234 (ch420): A1-NE18-01 (artistic disciplines) and A1-NE08-02 (shows and exhibitions). Three words -- teatro, exposicion, circo -- close two points, because six of NE18-01's eight exponents were already taught. A1-NE18-06 (cinema and theatre) is deliberately LEFT NULL: it also wants actor and actriz, and wiring it on teatro alone would repeat the over-claim blocked on A1-NE06-05. // 231 -> 232: A1-F3-03, whose note said the corpus never introduced preferir while ES-C396-preferir does. A1-NE06-05 was wired here too and REVERTED before merge: its label is "examinations and marks" and the marks half (nota, calificacion, aprobar, suspender) is absent, so wiring it on examen alone would have contradicted the call at A1-NE18-06, where pelicula without teatro is held not to buy "cinema and theatre". The 21% figure in the older comment below is WITHDRAWN -- see HL-C376; it came from a keyword scan, and the ES-LEX-<WORD> scan that replaced it was also wrong because half the corpus uses chain ids (ES-LEX-C354-WHERE-28 is el cine). Resolve an exponent through the lesson headword, never an atom id. // 229 -> 231 (ch419). TWO points, ONE new lesson, and the split is the finding: A1-NE18-02 (music and dance) needed authoring -- ES-LEX-BAILAR did not exist -- while A1-NE18-05 (photography) needed NOTHING but a probe: ES-LEX-FOTO and ES-LEX-FOTOGRAFIA have been introduced by ch405 all along and the point was counted uncovered only because nobody wired it. A corpus-wide scan puts 451 of the 2,189 uncovered points (21%) in that second class. // 85 grammar (unchanged) + 144 newly mapped // ...and 262-266 close the last four enumerated points. The inventory scope remains partial. // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250 // +6 ch251-256 // +4 ch257-261: the four rules the book had always demonstrated and never stated // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250 // +6 ch251-255: the half-taught sets finished, plus bastante which was already taught and merely unwired // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250: the stressed pronouns, the exclamative and the vocative // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245: the vosotros preterite and the imperfect plural, both promised in chapter 204 // +3: ch221-225 demonstratives // +4: ch226-229 degree words // +4: ch230-235 joining words // +2: ch236-240 the gerund and the personal a // +3: chapters 221-225 teach the demonstratives // +4: chapters 226-229 teach muy, bastante and mal // +4: chapters 230-235 teach al/del, quien, o and ni
    expect(coverage.percent).toBe(99); // 97 -> 99 (ch429-430) // 96 -> 97 (ch428) // 93 -> 96 (ch427) // 90 -> 93 (ch425-426) // 87 -> 90 (ch424) // 86 -> 87 (ch421-423) // 85 -> 86 (ch420) // 84 -> 85 (ch419) // 53 -> 56 -> 60 -> 64 -> 66 -> 68 -> 71 -> 77 -> 81 -> 85/85 grammar-only, then 223/273 across four dimensions
    expect(coverage.unmapped).toBe(2); // 7 -> 2 (ch429-430), and the 2 are the deliberate nulls // 11 -> 7 (ch428) // 18 -> 11 (ch427) // 28 -> 18 (ch425-426) // 36 -> 28 (ch424) // 39 -> 36 (ch421-423) // 41 -> 39 (ch420) // 42 -> 41 // 44 -> 42 (ch419): NE18-02 authored, NE18-05 merely wired // was 0 while only grammar was enumerated

    // Whole categories missing is a different failure from thin coverage, and
    // the report has to keep them distinguishable. These three are GRAMMAR
    // categories and are deliberately unchanged: the new points all landed in
    // new categories, so if one of these ever moves, a grammar point moved.
    expect(coverage.byCategory["Los demostrativos"]).toEqual({ enumerated: 3, covered: 3 }); // closed by chapters 221-225
    expect(coverage.byCategory["El sintagma adjetival"]).toEqual({ enumerated: 1, covered: 1 }); // closed by ch226-229: muy, poco and bastante are all taught now
    expect(coverage.byCategory["La oracion simple"]).toEqual({ enumerated: 6, covered: 6 });

    // The two categories that are now entirely absent from the book. Naming
    // them is the point of the per-category tally: "82%" is a mood, "the
    // orthography inventory is 2/21 and clothing is 0/3" is a work queue.
    expect(coverage.byCategory["Ortografia de letras y palabras"]).toEqual({ enumerated: 7, covered: 7 }); // 0 -> 7 (ch425-426): the category this comment named as entirely absent, closed entire
    expect(coverage.byCategory["Puntuacion"]).toEqual({ enumerated: 9, covered: 9 }); // 1 -> 9 (ch424): the category the comment above named as the work queue, closed entire
  });

  it("reports the shortfall in a form somebody can act on", () => {
    const lessons = loadTrackLessons("spanish");
    const report = formatExamCoverage(
      measureExamCoverage(loadExamInventory("spanish", "A1"), lessons),
    );
    expect(report).toContain("spanish A1 (partial inventory): 271/273 points covered (99%)");
    expect(report).toContain("2 with no corresponding atom");
    // Worst category first, not alphabetical. This USED to be checkable against
    // the real corpus, whose emptiest category kept changing as the campaign
    // closed points — `El sintagma adjetival` at 0/1, then `Los cuantificadores`
    // at 1/4, then 2/4, then 3/4. Every ENUMERATED category is now at 100%, so
    // the ordering falls back to the alphabetical tie-break, and
    // asserting the real report's first line would pin that tie-break while
    // claiming to pin the sort.
    //
    // The property therefore moves to data that can still falsify it. This is
    // not a weakening — the real report simply stopped being a test case for
    // ordering the moment there was nothing left to order.
    const uneven = measureExamCoverage(
      {
        ...FIXTURE,
        points: [
          // The names matter. "Poor"/"Rich" would order the same way
          // alphabetically as by shortfall, so the assertion could not tell the
          // sort from the tie-break — a security review proved that by deleting
          // the shortfall comparator and watching this test still pass. These
          // names make the two orderings CONTRADICT: alphabetically Alpha comes
          // first, by shortfall Zeta does.
          { id: "F-1", category: "Alpha", label: "covered", probe: ["ES-A"] },
          { id: "F-2", category: "Alpha", label: "covered", probe: ["ES-B"] },
          { id: "F-3", category: "Zeta", label: "uncovered", probe: null },
        ],
      },
      [lesson("ES-1", ["ES-A"]), lesson("ES-2", ["ES-B"])],
    );
    const unevenReport = formatExamCoverage(uneven).split("\n");
    expect(unevenReport[2]).toContain("0/1  Zeta");
    expect(unevenReport[3]).toContain("2/2  Alpha");
  });
});
