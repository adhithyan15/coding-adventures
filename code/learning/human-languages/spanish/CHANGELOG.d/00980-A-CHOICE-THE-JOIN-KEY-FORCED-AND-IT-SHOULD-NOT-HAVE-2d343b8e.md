## A choice the join key forced, and it should not have

`habitual` descends from *habēre*. The Spanish track spells that etymon **two
ways**: `habere` (on *hablaré*) and `habere-latin` (on *deber*, *he hablado*,
*haber*). The cousins join is exact string equality, so picking one silently
joins or splits the panel, and **nothing in CI catches the wrong choice**.

I took `habere-latin`, the majority spelling, and the join is right. A census
over the track's own `roots:` fields — stripping the language suffix and
prefix, nothing else — finds **31 etymon cores spelled more than one way**, of
which **29 are pure shape splits** (same language, different slug shape) and 2
are cross-language pairs. A first pass said 36; that number came from a looser
normaliser and is withdrawn.

| etymon | spellings | what fails to join |
|---|---|---|
| stare | `stare`, `stare-latin`, `latin-stare` | *está* / *estar* / *estuve* — three ways |
| sperare | `sperare`, `sperare-latin` | *espero viajar* / *esperar* |
| caput | `caput-latin`, `latin-caput` | *acabar* / *la cabeza* |
| bursa | `bursa-greek`, `bursa-latin` | ***el bolso* / *la bolsa*** |

The **bursa** pair is the sharpest and is not an artefact of the census: Greek
*byrsa* entered Latin as *bursa*, and **both** Spanish words descend from the
Latin. Tagging one `-greek` and one `-latin` separates two words that are
plainly cousins.

**And the census undercounts**, which the security review found by catching an
error of mine. `ES-C458-admitir` claimed to be the **third** corpus word off
*mittere*, after *meter* and *el permiso*. It is the **fourth**: `ES-C405-permitir`
has been teaching *mittere* since chapter 405. I missed it because that lesson
carries the slug `permittere-latin` while the others carry `mittere-latin` — so
a census keyed on the slug **cannot see the relationship at all**, and 31 is a
floor rather than a count.

`HL-C419` proposed a `check:root-slug-splits` guard for exactly this. It has
stopped being a cleanup: four of this tranche's own candidates landed on split
etymons, one of them made me miscount a family in a published lesson, and the
next tranche will hit more.

