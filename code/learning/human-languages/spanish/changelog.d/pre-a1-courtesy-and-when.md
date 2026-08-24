## Unreleased — Pre-A1: what courtesy is made of, and how to say when

Adds chapters 320-323 — twenty `type: word` lessons at sequences 5120-5310, all
pre-A1, all `variety: american-neutral`.

**320 — The Words Behind Please and Sorry** (`SPINE-COURTESY-THANK`): *el
favor*, *la molestia*, *la pena*, *la suerte*, *la culpa*. The track already
teaches the courtesy *phrases* — *por favor*, *gracias*, *de nada*, *lo siento*,
*perdón*, *con permiso*, *no hay de qué*. This chapter opens them and hands the
learner the nouns inside, so politeness stops being a set of fixed strings.

**321 — Pleasure, Reason, Respect** (`SPINE-COURTESY-THANK`): *el placer*, *la
razón*, *el respeto*, *la paciencia*, *el consejo*. What you offer back: the
answer to *mucho gusto*, the way Spanish agrees (*tienes razón* — you *have*
reason), and the three things you can ask somebody for by name.

**322 — Five Sizes of When** (`SPINE-TAKE-LEAVE`): *anoche*, *la semana*,
*temprano*, *enseguida*, *todavía*. Five scales of time, from one night to a
thing that has not stopped.

**323 — Before, After, and While** (`SPINE-TAKE-LEAVE`): *ya*, *entonces*,
*antes*, *después*, *mientras*. Ordering words — enough to say more at a door
than *hasta luego*.

### Why these words, in this order

- **Phrases opened into their parts.** *Por favor* has been taught since chapter
  14 as one unit; `ES-C320-favor` splits it and shows *favor* standing alone.
  This is the move `ES-C317-poco` made with *tampoco* and `ES-C321-consejo`
  makes again with *ajo*: the payoff of a word the learner already carries.
- **The `-o`/`-a` contrast, reinforced rather than restated.** Chapter 304
  established that `-o` switches and `-e` does not; chapter 316 added that
  *nunca* ends in `-a` and still does not move. `ES-C322-temprano` closes the
  set from the other side: *temprano* ends in `-o` and does **not** switch,
  because it says *when* rather than what somebody is like. An `-o` on the end
  is not a promise.
- **The Spanish adverbial `-s`, taught as a pattern rather than four facts.**
  *entonces*, *antes*, *después* and *mientras* all carry a final `-s` that is
  not a plural. Each lesson notes it in passing; `ES-C323-mientras` names it and
  closes the chapter on it.
- **Doublets, where Spanish borrowed its own word twice.** *rationem* gives both
  the weathered *razón* and the bookish *ración*; *respectus* gives *respeto*
  and *respecto*; Latin *consilium* gives *consejo* while English split the same
  word into *counsel* and *council*.
- **Two sound roads already taught, re-proved.** *sortem* → *suerte* walks the
  stressed *o* → *ue* break from chapter 300. *consilium* → *consejo* walks the
  *-lium* → jota road that `ES-C297-ajo` established with *allium* → *ajo*.

### Etymon ledger

Re-spends `favere-latin` (with `ES-C06-por-favor`) and `tempus-latin` (with
`ES-C30-el-tiempo`), so *el favor* and *temprano* land as payoffs on roots the
track has already spent rather than as new facts. Mints `moles-latin`,
`poena-latin`, `sortem-latin`, `culpa-latin`, `placere-latin`,
`rationem-latin`, `respicere-latin`, `specere-latin`, `pati-latin`,
`consilium-latin`, `noctem-latin`, `septem-latin`, `sequi-latin`, `via-latin`,
`iam-latin`, `tunc-latin`, `ante-latin`, `post-latin`, `interim-latin` and
`inter-latin`.

Two English handles are worth naming because they are not obvious. *Ya* is Latin
*iam*, which an English speaker already says inside the borrowed *déjà vu* —
*already seen*. And *entonces* is *in tunc*, where *tunc* grows out of the same
old pointing stem behind English *that*, *there*, *then* and *the*.

### Substitutions from the planned allocation

Three words in the chapter-322 plan turned out to be taught already and were
replaced:

- *hoy* and *ahora* are both introduced by `ES-C65-ahora-hoy` (sequence 2495).
- *mañana* is claimed by `ES-LEX-MANANA` (`ES-C05-hasta-mañana`).

Replaced with *anoche* (< *ad noctem*), *la semana* (< *septimana*, the set of
seven — a payoff on *siete*) and *enseguida* (< *en seguida*, "in what follows"
— a payoff on *conseguir* and *según*). *Pronto* was also checked and is taken
by `ES-LEX-PRONTO`.

### Wiring

- `spanish/chapters.json` — four chapter entries, each with a `production`
  payoff on the chapter's last lesson.
- `spanish/curriculum.json` — paths `ES-PATH-320-01` … `ES-PATH-323-01`,
  extensions `ES-EXT-320-COURT`, `ES-EXT-321-COURT`, `ES-EXT-322-WHEN`,
  `ES-EXT-323-WHEN`, and the four new path ids appended to the
  `SPINE-COURTESY-THANK` and `SPINE-TAKE-LEAVE` segment ledgers.
- `core/book-generation.json` — four targets, kept inside the Spanish group.
- `spanish/book/book.tex` — no hand-edit. HL21 4/4 made this file generated, so
  the four `\input` lines are re-derived from the `book-generation.json` targets
  above.

Spanish at-or-below-pre-A1 vocabulary moves 229/300 → **249/300** (407 → 427
total), measured on top of the kinship tranche.
