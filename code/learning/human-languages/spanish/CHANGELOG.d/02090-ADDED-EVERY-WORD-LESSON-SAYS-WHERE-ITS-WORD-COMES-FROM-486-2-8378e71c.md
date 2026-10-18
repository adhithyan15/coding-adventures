## Added — every word lesson says where its word comes from (486 / 209 → 0 / 0)

HL24 §9's etymology gap for Spanish is closed: every word lesson now has an
`etymology_hook` and a visible origin.

- **Hooks for 287 lessons that already told the story.** Each hook agrees with
  the lesson's own block, and every block was reread against EtymDB 2.1
  (Fourrier & Sagot 2020, CC BY-SA 4.0) and Lewis & Short while writing it.
- **New "taken apart" blocks for 199 lessons.** 185 are the generated wave
  ES-C475-profesional through ES-C511-soler; the rest are hand-written lessons
  (ES-C03-tu-usted-register, ES-C04-estar-estado, ES-W01-tilde-diacritica,
  ES-C06-hablo-espanol, ES-C42-te, ES-C45-nos, the four ES-C267 lessons,
  ES-C431-cobrar, ES-C433-junto, ES-C436-prestar, ES-C437-tardar). The ten
  hook-only lessons gained a block, or for ES-C52-como-se-dice (299 s) one
  origin sentence in place of a sentence that said less. Inherited and learned
  are decided by Spanish sound change (*-ct-* → *-ch-*, *f-* → *h-*,
  *pl-*/*cl-* → *ll-*, *-li-* → *j*, the *ie*/*ue* diphthongs), not by the
  dataset's own label; reconstructions carry their star; Arabic loans name the
  Arabic word (*alquiler*, *ahorrar*); about sixteen say the origin is
  uncertain or only usually traced (*ganar*, *buzón*, *regalar*, *atasco*).
- **Corrections to 41 older passages**, among them: *hablaré* is no longer
  traced to a Latin *hablare habeo* (Latin had no such verb; the textbook form
  is *cantare habeo*); *cerrado* no longer sits on *claudere* (ES-C37-cerrar
  already says *serare*); *coincidir* is *cadere* "to fall", not the *caedere*
  of *decidir*; English *counsel* and *council* are two Latin words
  (ES-C321-consejo, ES-C473-aconsejar); *serio* is not "weighty" in Latin;
  *planta* meant a cutting as well as a sole (ES-C444-plantar now agrees with
  ES-C390-planta); *sofá* came through French; *batería* and *actriz* are
  borrowed, not inherited; *lugar* is *localis*; *mira* is "to marvel at";
  *mute*, *dilate*, the *-al* of *equal* and *Antarctic* are fixed; missing
  reconstruction stars restored (*\*plovere*, *\*me-*, *\*tu-*,
  *\*imbarricare*, *\*wardōn*, *\*captiare*, *\*abantiare*, *\*singellus*).
  22 blocks that never named an origin (the ES-C424 to ES-C426 punctuation
  words, *taller*, *máquina*, *pieza*, *cenar*) gained one sentence.
- Eleven `max_seconds` rose to the computed length, all under 300 s:
  ES-C01-buenos-dias 240→250, ES-C03-me 180→182, ES-C03-tu-usted 200→253,
  ES-C03-tu-usted-register 240→263, ES-C04-estar-estado 220→237,
  ES-C06-hablo-espanol 230→280, ES-C07-hablan 250→270,
  ES-C10-futuro-simple 240→288, ES-C421-actriz 240→297,
  ES-C422-pagina-web 240→266, ES-C462-reducir 275→281.
- **Review corrections.** An independent review read all 497 changed lessons
  against EtymDB 2.1, Lewis & Short and the standard accounts (Corominas, the
  DLE), and fixed what it found:
  - *Loan routes.* *empresa* is Italian *impresa*; *bailar* came through Occitan
    *balar* (as ES-C377-baile already said); *ruta* is French *route*; *neto*
    came in through Catalan or French *net*; *nivel* through Catalan *nivell* or
    Occitan.
  - *Etyma and glosses.* *aceptar* is *acceptare*, *opinión* is *opinio*,
    *portero* is Latin *portarius*. Italian *poggio* is a hillock, not "a
    support" (*apoyar* ← *appoggiare* ← *\*appodiare*). *ayer*'s *y* is the
    diphthong of stressed *e*, not a softened *-ri*. *máquina* is learned
    because it kept its middle vowel, not because of its stress. *ver* is one
    syllable, not "two letters".
  - *Hedges.* *andén* now names the usual proposal, Latin *indago*, as a
    perhaps. *jubilar* "to retire" came through the jubilee, not a cheer. How
    *copia* "plenty" became a copy is not settled. *ayer*'s *ad*, *doccia*'s
    *ductio* and *ballare*'s Greek source are "probably".
  - *ES-C421-actriz*'s recall still said *actrix* was inherited and *matrix*
    shared the ending; it now matches the corrected block.
  - *Labels.* Eighteen words a sound test decides are now labelled learned
    (*proteger*, *felicitar*, *habitante*, *meta*, *aceptable*, *iniciar*,
    *profesión*, *divorcio*, *mención*, *apetecer*, *preocupar*, *elegir*) or
    inherited (*ensayo*, *caliente*, *limpio*, *juego*, *boca*, *pelo*).
    *respetar* is semi-learned, not learned.
  - *Visible origin.* 26 of the 46 blocks that only named Spanish parts now
    say where those parts come from (*buenas noches*, *comprador*, *portal*,
    *cumpleaños* and others).
  - *Hooks.* Three hooks over 120 characters (ES-C07-hablan,
    ES-C09-estudiante, ES-C54-hay) were shortened; ES-C426-minuscula-inicial's
    hook no longer names a word its block lacks.
  - *English asides.* "Cousin" is kept for inherited kin; nine lessons used it
    for borrowings (*nocturnal*, *mode*, *disgust*, *clamour*, *multiple*,
    *vision*, *record*, *enterprise*, *balloon*).
  - *Root slugs.* *parecer* and *parecido* join Portuguese on
    `apparere-latin` instead of sharing `parere` with *pariente*; *habitar*,
    *decir*, *esperar* and *merced* use the `-latin` spelling the rest of the
    corpus uses (four split-baseline entries pruned); *cancelar* uses
    `cancellus-latin` like Italian *cancello*; *comprender* also carries
    `latin-comprehendere`.
  - *Durations.* ES-C421-actriz 297→276 and ES-C10-futuro-simple 288→273
    after tightening; the review's own sentences raised ES-C03-como-se-llama
    240→267, ES-C03-me-llamo 180→199 and ES-C419-bailar 240→291, each to its
    computed length.

