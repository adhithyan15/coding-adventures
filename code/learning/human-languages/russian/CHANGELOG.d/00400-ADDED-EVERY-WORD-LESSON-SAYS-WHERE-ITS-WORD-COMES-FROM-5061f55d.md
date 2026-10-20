## Added — every word lesson says where its word comes from

`ETYMOLOGY_GAP_PINS.russian` falls from 1135 lessons without an `etymology_hook` and 1148 without visible etymology to 0 and 0 (HL24 §9).

- **Shape.** The 1,135 generated word lessons (chapters 29 onward) each gained a `## The word, taken apart` block after the section that introduces the word, a matching `etymology_hook` of 120 characters or fewer, and `roots:` slugs. Thirteen earlier hand-written lessons gained the half they lacked: RU-C12-papa, RU-C17-drugoy, RU-C21-pomogite, RU-C22-kakoy, RU-C25-dvadtsat and RU-C26-vtoroy-tretiy as a block; RU-C02-menya-zovut, RU-C19-potomu-chto, RU-C20-nado, RU-C21-prostite, RU-C21-povtorite, RU-C21-medlenno and RU-C22-skolko as one origin sentence in an existing section, paid for by trimming nearby prose.
- **Source.** Max Vasmer's *Russisches etymologisches Wörterbuch* (Russian edition, with Trubachev's additions), paraphrased and cited "(Vasmer)" beside the fact; no sentence is copied. Vasmer's hedges ("probably", "doubtful", "disputed") are kept, and his older reconstruction notation is given in standard starred forms (Proto-Slavic yers as *ŭ*, *ĭ*). Old Russian and Old Church Slavonic forms that need a letter this track never teaches (*ъ*, *ѣ*) are given in scholarly transliteration (*sapogŭ*, *nadobě*).
- **What they say.** Of the 1,135 generated blocks: 457 words built from parts the learner can see, 257 inherited Slavic words with their cousins, 63 inherited words whose deeper origin is unclear, 40 Church Slavonic shapes (most named by their sound change, such as *-ра-* for *-оро-* or *щ* for *ч*), 90 Germanic loans (Gothic, Old Norse, German, Dutch), 52 French, 28 Greek, 26 English, 15 Turkic, 84 from other donors (Latin, Polish, Italian, Persian, Arabic and more), and 23 whose origin Vasmer calls uncertain or disputed. 935 lessons cite Vasmer.
- **Durations.** No lesson crossed its declared length because of this change. RU-C02-menya-zovut, which already estimated over its declared 240 s, had `duration.max_seconds` raised to its computed 275 s; it is the only raise.
- **Roots.** 1,143 lessons gained `roots:` slugs (1,167 additions, 332 reusing a slug the corpus already had); `check:root-slug-splits` reports no new split.

### Review corrections

An independent review read all 1,148 entries beside their Vasmer articles, with Derksen's Proto-Slavic dictionary for the standard reconstructions. 36 lessons needed a text correction (about 3%), and 7 more needed only a root-slug change. No block used the wrong article, and none copies Vasmer: the longest shared run is a 21-character Latin phrase. Every edit keeps the `hl-knowledge` lines and stays inside the declared duration.

- **Forms and reconstructions.**
  - RU-C19-potomu-chto: *что* rests on *\*čĭ-*, not on *\*kŭ-*, the stem of *кто*.
  - RU-C76-ninety: drops the invented *\*devęno-desętŭ*. Vasmer gives only an Indo-European "ninth ten".
  - RU-C70-friday: Old Russian *pjatŭkŭ*, not *pjatokŭ*.
  - RU-C248-plashch: Old Church Slavonic *plaštĭ*, not *plaštŭ*.
  - RU-C52-book: *\*küiniv* now carries its star.
  - RU-C26-vtoroy-tretiy: *\*vĭtorŭ* goes with Sanskrit *vítaras*, and *\*vŭtorŭ* with Lithuanian *añtras*.
- **What Vasmer says.**
  - RU-C20-nado: *доба* is "a time, a season", not "fitness".
  - RU-C34-sadness: adds Vasmer's preferred link with "to gnaw".
  - RU-C48-room: adds the Polish route that the older stress points to.
  - RU-C56-forest, RU-C119-chest and RU-C30-finger: the notes are now credited to Trubachev.
  - RU-C64-good: Abaev backed the Ossetian loan, not the link with Khors.
  - RU-C66-slow: the hook drops an unsourced tie to *мешкать*.
  - RU-C74-always: Lithuanian *visadà* matches the forms without *г*. Vasmer ties *-гда* to *kadà*.
  - RU-C112-music: tells the two stresses apart.
  - RU-C160-korobka: restores the loan view.
  - RU-C163-shchi: leads with Vasmer's own view, Old Russian *sŭto*, "food".
  - RU-C164-vozdukh: no longer calls the word a copy of Greek.
  - RU-C193-sergi: adds Vasmer's preferred Old Chuvash source.
  - RU-C208-trogat: Vasmer's doubt is about *tergeō*, not *trahō*.
  - RU-C234-podmetat: the verb is *мести*, "to sweep".
  - RU-C249-smetana: the source is "to throw off", not "to sweep off". Adds the *\*sŭmętana* view.
  - RU-C251-gus: Vasmer's likeliest view is a native word reshaped after Germanic *gans*.
  - RU-C189-russkiy: adds the Finnish step, *Ruotsi*.
- **Hedges.**
  - RU-C45-carrot, RU-C59-wind, RU-C120-monkey and RU-C70-saturday: Vasmer's "cannot be proved", "probably" and "not fully satisfying" are back.
  - RU-C57-shore: *берег* is no longer flatly inherited. Its *г* is irregular, and a loan is widely argued.
  - RU-C59-wind: English *wind* is now a "cousin", because "comes from" is kept for borrowings.
- **Church Slavonic.**
  - RU-C87-square: its *щ* is common Slavic *\*sk*, not a Church Slavonic sign.
  - RU-C123-stairs: native Old Russian *lěstvica*, not "from Old Church Slavonic".
  - RU-C255-soobshcheniye: notes that *общий* is the Church Slavonic form of native *обчий*.
- **Donor.** RU-C241-podval: *вал*, "a rampart", came probably through Polish from German *Wall*, Latin *vallum*. It does not come from *валить*.
- **Hooks that contradicted their blocks.** RU-C215-k-schastyu and RU-C216-schastlivyy now give *\*sŭ-*, "good".
- **Roots.** `check:root-slug-splits` still reports no new split.
  - Merged: `slavic-dukhu` into `slavic-dykh`, because *дух* and *дышать* are one root.
  - Merged: `slavic-gor` into `slavic-goreti`, because *жар* and *гореть* are one root.
  - Split: *счастье* (`slavic-chesti`, *\*čęstĭ*, "share") from *часто* (`slavic-chestu`, *\*čęstŭ*, "dense").
  - Corrected: *стоматолог* takes `stoma-greek`. It had shared `stomakhos-greek`, the slug of Italian *stomaco*, "stomach".
  - Corrected: inherited *поздно* and *опаздывать* take `pie-pos` in place of `post-latin`.
  - Corrected: the calque *предмет* takes `slavic-metati` in place of `iacere-latin`.
  - Corrected: *щи* takes a neutral `slavic-shchi`.
  - Corrected: *подвал* takes `vallum-latin`.

