## Unreleased — Chapters 282-288: thirty-five words on the seven pre-A1 nodes

Spanish stood at 48 distinct headwords against the 300 the pre-A1 vocabulary floor
asks for — the furthest behind of any track, having been overtaken by all five
Indic tranches. These seven chapters answer with thirty-five, under HL14's rule:
**one new headword per lesson**, and reuse of everything already taught, unlimited
and deliberate.

```
282 MEET-GREET             bienvenido, el saludo, el abrazo, la sonrisa, el invitado
283 COURTESY-THANK         disculpe, con permiso, amable, mil gracias, no hay de qué
284 RESPOND-BASIC          de acuerdo, seguro, cierto, exacto, por supuesto
285 EXCHANGE-NAMES         el vecino, la amiga, el maestro, la enfermera, el cocinero
286 CHECK-WELLBEING        el ojo, la boca, la garganta, el dolor, la fiebre
287 POLITE-REQUEST-REPAIR  el jabón, la toalla, el vaso, la cuchara, el plato
288 TAKE-LEAVE             la salida, el viaje, la vuelta, la llegada, la despedida
```

The vocabulary shortfall falls 252 to 217 — a drop of exactly thirty-five, the
lesson count — and the pre-A1 reinforcement blocker does not move at all, holding
at 24.

**Why the position and the level are independent.** Spanish had already realised
all seven pre-A1 spine nodes, so there was no unused node to aim at: node REUSE is
the whole mechanism here. Each chapter sits on one pre-A1 node, five lessons
sharing it, and all seven nodes are used. Two earlier Spanish tranches spent good
words on A1 nodes and moved this number by zero; appending at chapter 282 while
declaring `stage: pre-A1` is what makes these thirty-five count. `ES-C55-la-bebo`
has sat at chapter 144 of 281 on a pre-A1 node for a long time and validates
clean — where a chapter sits in the book and what level it teaches are separate
facts.

**The ramp got gentler while the book got longer.** The R1 reinforcement ratio
falls 0.2977 to 0.2949 with the numerator held at 1106: not one of the
thirty-five new atoms misses its R1 window. That is the chaining doing the work.
Every lesson chains to the one before; each chapter's first lesson practises the
previous chapter's last two atoms and its second lesson practises the last one,
so a word introduced by a payoff lesson is still being retrieved two lessons into
the following chapter. The corpus share of atoms never revisited anywhere holds at
15%, and Spanish's own `atomsNeverRevisited` actually falls, 129 to 128:
`ES-C281-sucio` was the last lesson in the track and had never been revisited by
anything, and `ES-C282-bienvenido` now revisits it.

**Every headword was checked against all 479 existing lessons before it was
written**, in both directions, with the forward-reference detector's own
word-boundary regex on both the plain and the emphasis path. Spanish is by far the
largest track and the collision rate was correspondingly high: of the candidates
screened, `la sal`, `la leche`, `el aceite`, `el azúcar`, `la llave`, `el pie`,
`el pelo`, `el médico`, `el panadero`, `el hijo`, `el niño`, `la niña`,
`el compañero`, `vale`, `apenas`, `nunca`, `verdad`, `el camino`, `el tren`,
`el autobús`, `la ciudad`, `la mesa` and `desde luego` were all discarded for
colliding with committed prose, and `claro` was discarded because `ES-C272-si-claro`
already teaches it inside the headword `sí, claro`. A full re-simulation of the
forward-reference pass over the finished 514-lesson track reproduces Spanish's
committed figure of 91 exactly and reports zero references involving a chapter
282-288 lesson in either direction. The corpus ceiling holds at 500 and the
rule-statement count holds at 30, both of which had zero headroom.

**Two threads run the length of the seven chapters** rather than sitting in one
lesson each. The first is the prefix as a reusable part: `sub-` turns a laugh into
a smile in `sonrisa` and places an assumption under a conversation in
`por supuesto`; `se-` takes the care out of `seguro` and sifts a `secret` aside
from `cierto`; `dis-` undoes a fault in `disculpe`. A prefix a reader can hear is
worth more than the word that carried it, because it works on words nobody has
taught them yet.

The second is that names for people and acts are descriptions that stopped being
descriptions. `Vecino` names a street, `amiga` a verb of loving, `maestro` a
comparison, `enfermera` a condition and `cocinero` an action — five ways of
pointing at a person without ever saying what a person is. `Salida`, `llegada`,
`vuelta` and `despedida` do the same for a departure, and four of the five words
in that chapter are verbs the reader already had, wearing an ending.

Two sound changes are shown often enough to become predictions rather than facts:
stressed *o* opening into *ue* (`acuerdo`, `vuelta`) and stressed *e* into *ie*
(`cierto`, `fiebre`), both cross-referenced to `puerta` where the reader first met
them. `Llegada` then pays off the `pl- → ll-` rule taught long ago: the reader can
derive `plicare` from the shape of `llegar` without being told.

Verified: `tsc` exit 0; `vitest` exit 0, 41 files / 714 tests, exit code read from
`$?` rather than the Tests line; `build.sh` exit 0 with the log read separately —
0 missing characters, 0 overfull, 0 underfull, 1209 pages, and
`scan_latex_log_warnings.py` reports every warning class at its recorded baseline;
`check:books`, `check:narration`, `check:modality`, `check:progress` and
`check:figures` all exit 0. Spanish's zero ceiling on chapter numbers in prose is
unchanged at zero, and `scriptClosureViolations` does not move.

Eighteen pins were set by hand across seven test files, integers only, with no
assertion weakened, removed or restructured. `repin_tests.py` was not run: the
synthetic `drivablePercent` fixture at `modality-manifest.test.ts:359` is
untouched at 67, and only the corpus pin moved, 69 to 70.

Unicode: zero NUL, ZWJ, ZWNJ, bidi controls, BOM and soft hyphen across all 35
lesson files, verified by reading the bytes rather than by grep. The only
non-ASCII characters in the tranche are the em dash, the rightwards arrow, and
Spanish's own `á é í ó ú ñ ¿` — every one of them decomposed to its Unicode name
so that no accented letter is a lookalike from another block.

