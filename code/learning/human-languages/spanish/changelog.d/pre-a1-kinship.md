## Unreleased — pre-A1 kinship vocabulary (chapters 306-309)

Twenty pre-A1 vocabulary lessons hanging on `SPINE-EXCHANGE-NAMES`, sequences
4420-4610, atoms `ES-LEX-C306-KIN-01` through `ES-LEX-C309-KIN-20`.

### Why here

The node teaches a learner to give their own name. The very next thing anyone
needs is a word for everybody else in the room, so this tranche continues
straight out of `ES-C305-encantado` rather than opening a node of its own.

### Added

- **Chapter 306 — Blood and Household**: `el hijo`, `la hija`, `el abuelo`,
  `la abuela`, `el esposo`.
- **Chapter 307 — The Wider Circle**: `la esposa`, `el tío`, `la tía`,
  `el primo`, `la prima`.
- **Chapter 308 — Children and Everybody**: `el niño`, `la niña`, `el chico`,
  `la chica`, `la gente`.
- **Chapter 309 — Naming the Whole**: `la familia`, `el apodo`, `el pariente`,
  `la pareja`, `el bebé`.

### The etymological through-line

Every lesson pays off an English cognate, and the chapters are ordered so the
roots talk to each other:

- `hijo`/`hija` open with the Latin **f- → silent h-** sound law (*filius* →
  *hijo*, and so *facere* → *hacer*, *ferrum* → *hierro*), cashed out in
  English *filial* and *affiliate*.
- `abuelo` introduces **avus**, which then explains English *uncle*
  (*avunculus*, "little grandfather") and *atavistic* — and two lessons later
  explains why `tío` looks nothing like *uncle*: Spanish took the Greek
  *theîos*, English took the Latin word off the `abuelo` root.
- `esposo`/`esposa` run **spondere**, "to pledge" — English *spouse*,
  *sponsor*, *respond*, *despondent* — and account for the prothetic **e-** in
  front of Latin *sp-*.
- `primo`/`prima` re-spend **primus**, already introduced by *primero*, and
  show that Spanish kept the adjective of *consobrinus primus* while English
  kept the noun.
- `gente` opens **gens**, the clan, and recovers the original sense of English
  *gentle*, *gentleman*, *genteel* and *Gentile*.
- `pareja` runs **par**, "equal", through *pair*, *peer*, *parity* and
  *umpire* (< *noumpere*, "not equal").

Two lessons deliberately teach the limits of the method. `chico` has no English
descendant at all and flags *chick* as a look-alike with no shared ancestor —
the learner's first false friend. `pariente` is the sharper case: it is
genuinely cognate with English *parent* but does **not** mean parent, and the
lesson also warns off *apparent*, which comes from a different Latin verb
(*appārēre*, "to appear") that merely resembles *parere*, "to bring forth".

`niño`/`niña` and `bebé` close the arc by pointing out that the words for the
smallest people have no Latin pedigree at all — they are babble, like English
*baby* — while noting that English already borrows `El Niño` untranslated, and
that Spanish `la niña del ojo` and English *pupil* (< *pupilla*, "little doll")
are the same metaphor invented twice.

### Wiring

- `chapters.json`: four chapter entries with production payoffs.
- `curriculum.json`: paths `ES-PATH-306-01`..`ES-PATH-309-01`, extensions
  `ES-EXT-306-KIN`..`ES-EXT-309-KIN`, and the four path ids appended to the
  `SPINE-EXCHANGE-NAMES` segment ledger.
- `core/book-generation.json` and `book/book.tex`: four generated chapters.

### Effect on the level gate

Spanish at-or-below pre-A1 vocabulary rises from **169/300 to 189/300**
(347 to 367 lessons overall) — exactly the twenty added here.
