# Human Languages — backlog

Note on provenance: this file was EMPTY in git for its whole history until now.
Findings from the pre-A1 tranche work were recorded in commit messages and PR
bodies, which are durable and searchable, but a reader opening this file found
nothing. The entries below are the ones that change how the work is done.

## HL-C09HG — Arabic ظ places its dot before the descending upright

The University of Oregon's dedicated emphatic-DHaa video extends the canonical
Arabic inventory with the next lesson-corpus base letter. Independent **ظ**
draws the complete **ط** body first, lifts once to place its upper dot, then
lifts again to draw the tall upright top-to-bottom. Its own video supplies that
body-dot-upright order rather than treating the shared skeleton as proof.

Arabic is now **26 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires three base letters: **غ, ف,
and ق**. Continue in source order with **غ** next; separately account for seated
Hamza and ending variants instead of conflating them with new base letters. The
production `script-data` batch must remain below the 250 kB authored-data target;
this build measures the Arabic-bearing batch at **39.48 kB**.

## HL-C09HF — Arabic ط loops before its descending upright

The University of Oregon's dedicated emphatic-Taa video extends the canonical
Arabic inventory with the next lesson-corpus base letter. Independent **ط**
closes its oval and exits left along the baseline first, then lifts once to
draw the tall upright top-to-bottom into the body's right junction. The direct
MOV and its accessible Panopto mirror agree on that two-run order.

Arabic is now **25 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires four base letters: **ظ, غ,
ف, and ق**. Continue in source order with **ظ** next; separately account for
seated Hamza and ending variants instead of conflating them with new base
letters. The production `script-data` batch must remain below the 250 kB
authored-data target; this build measures the Arabic-bearing batch at **37.42
kB**.

## HL-C09HE — Arabic ز keeps Raa's curve and adds its dot last

The University of Oregon's dedicated Zay video extends the canonical Arabic
inventory with the next lesson-corpus base letter. Independent **ز** draws the
complete **ر** body first — down through the short stroke and left through the
lower curve in one run — then lifts once to place the single upper dot. Its own
video supplies that order rather than treating the shared skeleton as proof.

Arabic is now **24 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires five base letters: **ط, ظ,
غ, ف, and ق**. Continue in source order with **ط** next; separately account for
seated Hamza and ending variants instead of conflating them with new base
letters. The production `script-data` batch must remain below the 250 kB
authored-data target; this build measures the Arabic-bearing batch at **35.76
kB**.

## HL-C09HD — Arabic ذ keeps Daal's body and adds its dot last

The University of Oregon's dedicated Dhaal video now extends the canonical
Arabic inventory beyond the greeting-era starter set. Independent **ذ** draws
the complete **د** body first — down-right through the shoulder and left along
the baseline in one run — then lifts once to place the single upper dot. Its
own video supplies that order rather than treating a shared skeleton as proof.

Arabic is now **23 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires six base letters: **ز, ط, ظ,
غ, ف, and ق**. Continue in source order with **ز** next; separately account for
seated Hamza and ending variants instead of conflating them with new base
letters. The production `script-data` batch must remain below the 250 kB
authored-data target; the current build measures **34.28 kB**.

## HL-C09HC — Arabic ء closes the source-backed starter inventory

Arabic Language Learning Notes' Naskh lesson replaces Arabic's final
conventional placeholder with
an explicitly documented one-stroke variant. Independent **ء** sweeps through
its c-shaped upper head, then continues without lifting from the lower-left end
through the lower diagonal. The source also records the accepted two-stroke
alternative, so the learner path does not erase real instructional variation.

The completion audit confirms **22/22 unique Arabic starter rows are
source-verified**, but Arabic must remain incomplete: the lesson corpus still
uses base letters absent from this starter inventory. Continue in source order
with **ذ** next, then **ز, ط, ظ, غ, ف, and ق**; separately account for seated
Hamza and ending variants instead of conflating them with new base letters.
The production `script-data` batch must remain below the 250 kB authored-data
target; the current build measures **32.71 kB**.

## HL-C09HB — Arabic ن sweeps its deep bowl before placing the dot

Waraqa Institute's adjacent beginner lesson now replaces the final body-and-dot
placeholder in Arabic's canonical sequence. Independent **ن** starts at the
upper-right tip, sweeps down and around its deep below-baseline bowl, then lifts
once to place the centred upper dot last. Arabic, Persian, and Urdu retain
independent source records for the shared glyph.

Arabic is now **21/22 verified, 1 remaining**. Finish the inventory with **ء**
next, then audit Arabic for canonical uniqueness and completion before moving to
the next script. The production `script-data` batch must remain below the 250 kB
authored-data target; the current build measures **31.46 kB**.

## HL-C09HA — Arabic م keeps its closed head and descending tail in one run

The next Arabic placeholder is now source-backed by Waraqa Institute's explicit
beginner writing lesson. Independent **م** forms its small, tightly closed head
first and continues down-left through the below-baseline tail without lifting.
The Arabic-scoped path remains independently addressable from the existing
Persian and Urdu records for the same Unicode glyph.

Arabic is now **20/22 verified, 2 remaining**. Continue in source order with
**ن** next. Arabic remains incomplete until each learner-facing row has its own
source-backed, font-checked path. Its `script-data` batch must remain below the
250 kB authored-data target; the production build measures **30.38 kB**.

## HL-C09GZ — Arabic ث adds its three dots after the shared bowl

The post-Gujarati audit promoted Arabic's earliest conventional placeholder.
The University of Oregon lesson's dedicated **ث** video draws the independent
shallow bowl right-to-left before its three separate upper dots. The fitted
Noto Naskh path preserves that body-first, four-run sequence.

Arabic is now **19/22 verified, 3 remaining**. Continue in source order with
**م** next. Arabic remains incomplete until each learner-facing row has its own
source-backed, font-checked path. Its `script-data` batch measures **29.33 kB**,
below the 250 kB authored-batch target.

## HL-C09GY — Gujarati closes with one canonical row per letter

The completion audit found that restored **ન** and **પ** still had their old
conventional placeholders later in `gujarati.json`. Those duplicate rows made
the inventory appear to contain 46 letters even though the teaching-app
sequence and the font-backed ductus cover 44 unique letters. The stale rows are
removed, and a cross-script uniqueness gate now rejects this class of drift.

Gujarati is now **44/44 source-verified and complete**. Future work should treat
this track as maintenance: repair source, font-fit, rendering, or curriculum
defects when evidence finds one, rather than manufacturing a 45th letter.
The deduplicated Gujarati-bearing `script-data` batch measures **82.13 kB**,
below its 250 kB target.

## HL-C09GX — Gujarati હ flows from its upper loop around the lower bowl

The source-and-font audit replaced **હ**'s conventional placeholder. The
t30apps.com animation gives it one populated SVG path: a continuous run circles
the compact upper loop, threads through the middle turn, and continues around
the broad lower bowl into its rightward finish. Noto Sans Gujarati prints those
features as two disconnected contours, so the fitted learner median uses the
source-backed connecting bridge to preserve that zero-lift order.

Gujarati is now **44/46 verified, 2 remaining**. Return to source order with
**ન** next, the earliest remaining conventional placeholder. The Gujarati-
bearing `script-data` batch measures **82.36 kB**, below its 250 kB target.

## HL-C09GW — Gujarati સ carries its looped body into the long shoulder

The source-and-font audit replaced **સ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles the
rounded upper loop, descends through the left body, and sweeps across the long
right shoulder; after one lift, the tall right spine descends into its lower
foot. The fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **43/46 verified, 3 remaining**. Continue in source order with
**હ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **80.92 kB**, below its 250 kB target.

## HL-C09GV — Gujarati શ joins its upper loop to the lower body before the spine

The source-and-font audit replaced **શ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles the
small upper loop and continues through the broad lower body into its tail;
after one lift, the tall right spine descends into its lower foot. The fitted
Noto Sans Gujarati medians preserve that order.

Gujarati is now **42/46 verified, 4 remaining**. Continue in source order with
**સ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **79.30 kB**, below its 250 kB target.

## HL-C09GU — Gujarati વ completes its rounded body before the right spine

The source-and-font audit replaced **વ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles the
broad rounded left body and returns into the right shoulder; after one lift,
the tall right spine descends into its lower foot. The fitted Noto Sans Gujarati
medians preserve that order.

Gujarati is now **41/46 verified, 5 remaining**. Continue in source order with
**શ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **77.74 kB**, below its 250 kB target.

## HL-C09GT — Gujarati ળ keeps its bowl, arch, and spine in one run

The source-and-font audit replaced **ળ**'s conventional placeholder. The
t30apps.com animation gives it one populated SVG path: one continuous run
circles the broad left bowl, rises through the narrow middle turn, crosses the
high right arch, and descends the tall spine into its foot. The fitted Noto Sans
Gujarati median preserves that zero-lift order.

Gujarati is now **40/46 verified, 6 remaining**. Continue in source order with
**વ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **76.18 kB** after the deterministic group split,
below its 250 kB target.

## HL-C09GS — Gujarati લ separates its rounded body, shoulder, and spine

The source-and-font audit replaced **લ**'s conventional placeholder. The
t30apps.com animation gives it three populated SVG paths: the broad rounded
left body is drawn first, the separate middle shoulder follows after one lift,
and the tall right spine descends into its foot after a second lift. The fitted
Noto Sans Gujarati medians preserve that order.

Gujarati is now **39/46 verified, 7 remaining**. Continue in source order with
**ળ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **244.77 kB**, below its 250 kB target.

## HL-C09GR — Gujarati ર keeps its upper body, loop, and tail continuous

The source-and-font audit replaced **ર**'s conventional placeholder. The
t30apps.com animation gives it one populated SVG path: one continuous run
circles the rounded upper body, narrows through the middle loop, and descends
into the lower-right tail. The fitted Noto Sans Gujarati median preserves that
zero-lift order.

Gujarati is now **38/46 verified, 8 remaining**. Continue in source order with
**લ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **242.99 kB**, below its 250 kB target.

## HL-C09GQ — Gujarati ય completes its rounded body before the right spine

The source-and-font audit replaced **ય**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles through
the rounded upper turn, sweeps around the broad lower body, and exits across the
long right shoulder; after one lift, the tall right spine descends into its
lower foot. The fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **37/46 verified, 9 remaining**. Continue in source order with
**ર** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **241.54 kB**, below its 250 kB target.

## HL-C09GP — Gujarati મ joins its left turn to the shoulder before the spine

The source-and-font audit replaced **મ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run curls through
the left body and compact inner turn before exiting across the long shoulder;
after one lift, the tall right spine descends into its lower foot. Gujarati is
now **36/46 verified, 10 remaining**; continue in source order with **ય** next.
The Gujarati `script-data` batch measures **239.91 kB**, below its 250 kB target.

## HL-C09GO — Gujarati ભ joins its loop and inner turn before the right spine

The source-and-font audit replaced **ભ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one continuous run
circles the broad left loop, winds through the compact inner turn, and exits
across the long right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The fitted Noto Sans Gujarati medians preserve
that order.

Gujarati is now **35/46 verified, 11 remaining**. Continue in source order with
**મ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **238.35 kB**, below its 250 kB target.

## HL-C09GN — Gujarati બ completes its rounded body before the right spine

The source-and-font audit found **બ** represented only by a conventional
placeholder. The t30apps.com animation gives it two populated SVG paths: one continuous run
circles the rounded left body, winds through the compact inner turn, and exits
across the right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The restored entry and fitted Noto Sans Gujarati
medians preserve that order.

Gujarati is now **34/46 verified, 12 remaining**. Continue in source order with
**ભ** next; it is the next conventional placeholder. A correctness defect
still outranks coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **236.70 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GM — Gujarati ફ completes its winding body before the cross-stroke

The source-and-font audit found **ફ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one continuous run
sweeps left across the high cap, winds down and around the body, circles the
small lower-left loop, and exits through the descending tail; after one lift,
a separate diagonal cross-stroke rises from lower left to upper right. The
restored entry and fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **33/46 verified, 13 remaining**. Continue in source order with
**બ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **235.05 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GL — Gujarati પ joins its high left hook to the lower body

The source-and-font audit found **પ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one run curls through
the high left hook, descends the left stem, and sweeps around the broad lower
body into the right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The restored entry and fitted Noto Sans Gujarati
medians preserve that order and the observed lift.

Gujarati is now **32/45 verified, 13 remaining**. Continue in source order with
**ફ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **233.10 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GK — Gujarati ન restores its loop, shoulder, and tall spine

The source-and-font audit found **ન** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one run circles the
small left loop and continues right across the long shoulder; after one lift,
the separate tall right spine descends into its lower foot. The restored entry
and fitted Noto Sans Gujarati medians preserve that order and the observed lift.

Gujarati is now **31/44 verified, 13 remaining**. Continue in source order with
**પ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **231.24 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GJ — Gujarati ધ separates its joined body from the tall spine

The t30apps.com animation gives **ધ** two populated SVG paths: first a joined
run descends from the high left entry, curls through the upper turn, narrows
through the middle, and sweeps around the broad lower body into the right
shoulder; after one lift, the tall right spine descends into its foot. The
fitted Noto Sans Gujarati medians preserve that order and the observed lift.

Gujarati is now **30/43 verified, 13 remaining**. Continue in source order with
**ન** next; it is missing from the current inventory and therefore outranks
later conventional placeholders. A correctness defect still outranks coverage
if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **229.51 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GI — Gujarati દ keeps its upper and lower turns continuous

The t30apps.com animation gives **દ** one populated SVG path. The pen circles
the rounded upper body from its upper-right start, narrows through the middle
turn without lifting, then sweeps around the broad lower body and rises into
the lower-right terminal. The fitted Noto Sans Gujarati median preserves that
continuous order while following the printed glyph's wider bowls and compact
terminal.

Gujarati is now **29/43 verified, 14 remaining**. Continue in source order with
**ધ** next; it is present in the inventory but still has only a conventional
placeholder. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **227.79 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GH — Gujarati થ restores its looped body and tall spine

The source-and-font audit found **થ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one continuous run
circles the small upper loop, descends through the middle, sweeps around the
broad lower body, and rises into the right shoulder; after one lift, a separate
tall spine descends into its lower foot. The restored entry and fitted Noto Sans
Gujarati medians preserve that order and the observed lift.

Gujarati is now **28/43 verified, 15 remaining**. Continue in source order with
**દ** next; it is present in the inventory but still has only a conventional
placeholder. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **226.28 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GG — Gujarati ત separates its open body from the tall spine

The t30apps.com animation gives **ત** two populated SVG paths: first the pen
starts at the lower terminal, sweeps left and upward around the open body, and
continues right across the upper shoulder; after one lift, it descends the tall
right spine and turns through the lower foot. The fitted Noto Sans Gujarati
medians preserve that order while following the printed glyph's wider body and
straighter spine.

Gujarati is now **27/42 verified, 15 remaining**. Continue in source order with
**થ** next; it is missing from the current inventory and therefore outranks
later conventional placeholders. A correctness defect still outranks coverage
if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **224.41 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GF — Gujarati ણ separates its hooked body, bowl, and spine

The t30apps.com animation gives **ણ** three populated SVG paths: first the left
spine flowing into its hooked lower tail, then the separate rounded middle
bowl, and finally the tall right spine and lower foot. The fitted Noto Sans
Gujarati medians preserve that order and the two observed lifts, including the
printed body's deeper below-baseline tail.

Gujarati is now **26/42 verified, 16 remaining**. Continue in source order with
**ત** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **222.85 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GE — Gujarati ઢ keeps its outer bowl and inner loop continuous

The t30apps.com animation gives **ઢ** one populated SVG path: the pen begins at
the upper left, sweeps right across the high shoulder, descends through the
middle and around the broad outer lower bowl, then continues directly around
the small inner loop without lifting. The fitted Noto Sans Gujarati median
preserves that shoulder-to-bowl-to-loop order and zero-lift evidence.

Gujarati is now **25/42 verified, 17 remaining**. Continue in source order with
**ણ** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **221.24 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GD — Gujarati ડ repairs the adjacent retroflex inventory gap

The canonical-order audit found **ડ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`; the prior backlog note
incorrectly called it present. Its animation uses one populated SVG path: the
pen begins at the upper right, sweeps left across the high shoulder, descends
through the middle and around the broad lower bowl, then finishes at the lower
left without lifting. The restored entry and fitted median preserve that order.

Gujarati is now **24/42 verified, 18 remaining**. Continue in source order with
**ઢ** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **219.74 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every added source
note; split the batch rather than weakening learner-facing provenance if a
later entry crosses the target.

## HL-C09GC — Gujarati ઠ closes the next consonant inventory gap

The canonical-order audit found **ઠ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses one
populated SVG path: the pen begins at the upper right, sweeps left across the
high shoulder, descends through the middle and around the broad outer lower
bowl, then curls back inward to its terminal without lifting. The restored
inventory entry and fitted median preserve that order and zero-lift evidence.

Gujarati is now **23/41 verified, 18 remaining**. Continue in source order with
**ડ** next; it is present in the source and current inventory but still needs
cited ductus verification. A correctness defect still outranks coverage if one
appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **217.89 kB** in this production
build, below its 250 kB authored-batch target. Keep rechecking it as the cited
inventory grows; split the batch rather than weakening or deleting learner-
facing provenance if a later entry crosses the target.

## HL-C09GB — Gujarati ટ keeps its upper turn and lower bowl continuous

The t30apps.com animation gives **ટ** one populated SVG path: the pen begins at
the upper left, sweeps across the upper turn, bends diagonally down-left through
the middle, then circles the broad lower bowl and finishes on its right side
without lifting. The fitted Noto Sans Gujarati median preserves that zero-lift
order.

Gujarati is now **22/40 verified, 18 remaining**. Continue in source order with
**ઠ** next; it is present in the source and font but absent from the inventory.
A correctness defect still outranks coverage if one appears during fitting or
validation.

The Gujarati-bearing `script-data` batch is now **244.12 kB** against its
250 kB authored-batch target. Recheck the production bundle before publishing
the next source note; split that batch rather than weakening or deleting
learner-facing provenance if the next entry crosses the target.

## HL-C09GA — Gujarati ઞ restores its body, shoulder, and tall spine

The canonical-order audit found **ઞ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: the rounded left body, a short separate rightward
shoulder, and a final tall descending spine that curls into its lower terminal.
The restored entry and fitted medians preserve that order and the two observed
lifts.

Gujarati is now **21/40 verified, 19 remaining**. Continue in source order with
**ટ** next; it already has a conventional inventory entry but still needs cited
ductus verification. A correctness defect still outranks coverage if one
appears during fitting or validation.

## HL-C09FZ — Gujarati ઝ repairs the next consonant inventory gap

The canonical-order audit found **ઝ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: the rounded left body, a separate right loop-and-tail
run, and a final short descending upper stem. The restored entry and fitted
medians preserve that order and the two observed lifts.

Gujarati is now **20/39 verified, 19 remaining**. Continue in source order with
**ઞ** next; it is also absent from the current inventory. A correctness defect
still outranks coverage if one appears during fitting or validation.

## HL-C09FY — Gujarati જ keeps its left loop, crossing body, and right loop joined

The t30apps.com animation gives **જ** one populated SVG path: the pen circles
the upper-left loop, continues diagonally through the crossing body, circles
the lower-right loop, and sweeps into the long upper-right exit without lifting.
The fitted Noto Sans Gujarati median preserves that zero-lift order.

Gujarati is now **19/38 verified, 19 remaining**. Continue in source order with
**ઝ** next; it is absent from the current inventory. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FX — Gujarati છ keeps both rounded bodies in one continuous run

The t30apps.com animation gives **છ** one populated SVG path: the pen circles
the upper-left lobe, turns back through the middle, continues around the broad
lower body, climbs the outer right curve, and finishes by circling the
upper-right lobe without lifting. The fitted Noto Sans Gujarati median preserves
that zero-lift order.

Gujarati is now **18/38 verified, 20 remaining**. Continue in source and
inventory order with **જ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FW — Gujarati ચ joins its upper bowl, middle loop, and lower body

The t30apps.com animation gives **ચ** two populated SVG paths: one continuous
run circles the upper bowl, turns through the small middle loop, and continues
around the broad lower body; after one lift, a separate run descends the full
right spine and turns through its lower foot. The fitted Noto Sans Gujarati
medians preserve that joined-body-before-spine order.

Gujarati is now **17/38 verified, 21 remaining**. Continue in source and
inventory order with **છ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FV — Gujarati ઙ repairs the adjacent consonant inventory gap

The canonical-order audit found **ઙ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses two
populated SVG paths: one long S-like run flows from the upper-right through the
upper turn and rounded lower body to the lower-left; after one lift, a compact
separate loop draws the upper-right dot. The restored entry and fitted medians
preserve that body-before-dot order.

Gujarati is now **16/38 verified, 22 remaining**. With the adjacent consonant
inventory gaps repaired, continue in source order with existing **ચ** next. A
correctness defect still outranks coverage if one appears during validation.

## HL-C09FU — Gujarati ઘ repairs the first consonant inventory gap

The canonical-order audit found **ઘ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses two
populated SVG paths: one continuous run flows through the upper lobe, middle
turn, and rounded lower body; after one lift, a separate run descends the full
right spine and turns through its lower foot. The restored entry and fitted
medians preserve that joined-body-before-spine order.

Gujarati is now **15/37 verified, 22 remaining**. Continue in source order with
**ઙ** next; it is also absent from the current inventory. A correctness defect
still outranks coverage if one appears during fitting or validation.

## HL-C09FT — Gujarati ગ separates its rounded body from the right spine

The t30apps.com animation gives **ગ** two populated SVG paths: one continuous
run circles the rounded left body from its upper-left start to its lower-left
finish; after one lift, a separate run descends the full right spine and turns
through its lower foot. The fitted Noto Sans Gujarati medians preserve that
body-before-spine order.

Gujarati is now **14/36 verified, 22 remaining**. Continue in source and
inventory order with **ઘ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FS — Gujarati ખ separates its joined body from the right spine

The t30apps.com animation gives **ખ** two populated SVG paths: one continuous
run begins at the upper left, descends through the left lobe, curls through the
middle, and finishes beside the right spine; after one lift, a separate run
descends the full right spine and turns through its lower foot. The fitted Noto
Sans Gujarati medians preserve that joined-body-before-spine order.

Gujarati is now **13/36 verified, 23 remaining**. Continue in source and
inventory order with **ગ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FR — Gujarati ક begins consonant coverage; ખ is next

The t30apps.com animation gives **ક** two populated SVG paths: one continuous
run circles the upper loop, crosses diagonally into the rounded lower body, and
finishes at the lower left; after one lift, a separate diagonal sweeps from
lower left to upper right. The fitted Noto Sans Gujarati medians preserve that
joined-body-before-cross-stroke order.

Gujarati is now **12/36 verified, 24 remaining**. Continue in source and
inventory order with **ખ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FQ — Gujarati ઋ closes the canonical-vowel inventory gap; ક is next

The canonical-order audit found **ઋ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: a bent left body, a separately descended central
stem, and a final right loop-and-tail run. The restored entry and fitted medians
preserve that order and the two observed lifts.

Gujarati is now **11/36 verified, 25 remaining**. With the source-backed
independent-vowel gaps repaired, continue to the first existing consonant,
**ક**, next. A correctness defect still outranks coverage if one appears during
fitting or validation.

## HL-C09FP — Gujarati ઔ repairs the second adjacent vowel gap; ઋ is next

The source-and-font audit found **ઔ** missing from `gujarati.json`. Its
t30apps.com animation repeats the four verified **ઓ** runs—joined body, first
stem, trailing stem, and lower high arc—then adds a fifth, higher arc. The new
inventory entry and Noto Sans Gujarati fit preserve all five paths and four
observed lifts.

Gujarati is now **10/35 verified, 25 remaining**. Rechecking canonical vowel
order exposed another inventory gap at **ઋ**, which appears in both the same
teaching source and bundled font but not the current data. Repair **ઋ** before
starting **ક**; a correctness defect still outranks coverage if one appears
during fitting or validation.

## HL-C09FO — Gujarati ઓ combines the verified આ body with one high arc

The t30apps.com animation exposes **ઓ** as four populated SVG paths: the joined
body, two separately descended stems, and a final high arc. The fitted Noto Sans
Gujarati medians therefore reuse the verified **આ** three-run structure before
adding **એ**'s high-mark movement, preserving all three observed lifts.

Gujarati is now **9/34 verified, 25 remaining**. Continue to **ઔ** next: the
source-and-font audit found another missing independent vowel rather than a
merely unverified entry, and its adjacent animation adds a second high arc to
the **ઓ** sequence. Repairing that inventory gap outranks the later consonants;
a correctness defect still outranks coverage if one appears during validation.

## HL-C09FN — Gujarati ઐ repairs an inventory gap; ઓ is next

The source-and-font audit found that **ઐ** was missing from `gujarati.json`, not
merely unverified. The t30apps.com animation gives it the first three **એ** runs
plus a fourth, higher arc; the new entry and Noto fit preserve all four paths
and the resulting three lifts. Gujarati's inventory denominator therefore rises
instead of hiding the omission.

Gujarati is now **8/34 verified, 26 remaining**. Continue to **ઓ** next: its
adjacent animation has four populated paths—one joined body, two separately
descended stems, and one high arc—so it can reuse verified **આ** and **એ**
anatomy. A correctness defect still outranks coverage if one appears during
fitting or validation.

## HL-C09FM — Gujarati એ establishes a three-run pattern; ઐ is next

The t30apps.com animation gives **એ** three populated SVG paths: a joined left
bowl, lower body, and small right arch; a separately descended full-height
right stem; then a separate high arc. The fitted Noto Sans Gujarati medians
preserve that body-before-stem-before-arc order and both observed lifts.

Gujarati is now **7/33 verified, 26 remaining**. Continue to **ઐ** next: its
adjacent animation repeats the same first three runs and adds a fourth high arc,
providing direct evidence for three lifts. Reusing the verified body, stem, and
first arc makes that the lowest-risk next tranche. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FL — Gujarati ઊ extends ઉ without lifting; એ is next

The t30apps.com animation repeats the complete **ઉ** path for **ઊ**, then keeps
the pen down across a high shoulder and along the extended right-side tail to
its lower foot. The fitted Noto Sans Gujarati median reuses the verified bowls
and outer curve before entering that added printed tail.

Gujarati is now **6/33 verified, 27 remaining**. Continue to **એ** next: its
adjacent animation changes the source pattern to three populated SVG paths, so
the next tranche should verify the joined body, separate right-side run, and
high arcing run plus the resulting two lifts. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FK — Gujarati ઉ verifies the bowl-and-return pattern; ઊ is next

The t30apps.com animation gives **ઉ** as exactly one populated SVG path. It
circles the small upper bowl through the middle cusp, sweeps around the broad
lower bowl, then climbs the tall outer-left curve to finish at the upper right.
The fitted Noto Sans Gujarati median preserves that whole zero-lift sequence.

Gujarati is now **5/33 verified, 28 remaining**. Continue to **ઊ** next: its
adjacent animation repeats the complete **ઉ** path and then extends the same
unbroken run into a long right-side tail. That direct reuse makes it the
lowest-risk next provenance and fitting tranche. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FJ — Gujarati ઈ confirms the adjacent source pattern; ઉ is next

The next t30apps.com animation also has exactly one populated SVG path. It
repeats **ઇ** through the upper loop, middle crossing, and lower loop, then
extends the rising motion into **ઈ**'s high clockwise curl. The fitted Noto Sans
Gujarati median stays on the taller printed outline without inventing a lift.

Gujarati is now **4/33 verified, 29 remaining**. Continue to **ઉ** next: it is
the next independent vowel in source order and keeps this tranche inside one
already-audited source before moving to ઊ. A correctness defect still outranks
coverage if one appears during fitting or validation.

## HL-C09FI — Gujarati ઇ is the next verified ductus; ઈ now outranks the rest

The t30apps.com version-1.0 animation gives **ઇ** as exactly one populated SVG
path: small upper-left loop, narrow middle crossing, broad lower loop, then the
rising upper-right hook. Its other path slots are empty, so this is positive
zero-lift evidence rather than a guess based on the finished outline. The fitted
Noto Sans Gujarati median stays on the bundled glyph's ink and preserves that
upper-loop → lower-loop → hook order.

Gujarati is now **3/33 verified, 30 remaining**. The next item is **ઈ**: it is
adjacent in the same source, reuses the newly checked loop-and-hook anatomy, and
therefore has lower provenance and fitting risk than jumping to a different
script or a later Gujarati letter. After ઈ, continue in source order through
ઉ and ઊ unless a correctness defect outranks coverage.

## HL-C213 — build all 22 books LOCALLY before pushing; it takes ~100 seconds

Measured on a 14-core laptop, clean rebuild, 8-way parallel:

```
all 22 books        ~100s wall clock
sequential sum      ~367s
spanish alone         74s  (1337pp — sets the parallel floor)
```

The same work in CI takes **5 to 58 minutes**, and once hung **6 hours** in `apt`
fetching the TeX toolchain, having compiled nothing. So the LaTeX work is not the
cost — provisioning and queueing are.

`data/scripts/build_all_books.sh` runs the lot and checks the four things that
matter. **The exit code alone is not one of them:**

| check | why the exit code misses it |
|---|---|
| missing characters | a font gap prints NOTHING and still exits 0 — telugu once shipped 89 |
| overfull boxes | spanish crossed 1000pp, contents numbers gained a digit, 14 lines overflowed, exit 0 |
| underfull boxes | the fix for overfull can trade one for the other |
| exit code | catches only a hard LaTeX error |

`sh build_all_books.sh --self-test` proves each detector fires on a known-dirty log
and stays silent on a clean one. It is not decoration: **it caught two real bugs in
this script before either ever ran on the corpus** —

1. `grep -c` EXITS 1 when the count is zero, so `|| echo 0` appended a second zero
   and the integer test errored instead of comparing. That silently disabled the
   missing-character check — the very thing the script exists for.
2. The overfull/underfull probes never fired, because `printf '%s'` does not
   interpret backslashes in its ARGUMENT, so `\\hbox` stayed two backslashes.

A rebuild overwrites `book.log`, so a defect planted in a real log cannot survive
long enough to test the detector. That is why classification is factored into
`classify_log` and tested against synthetic logs instead.

## Three rules this work keeps re-deriving

**A measurement that did not happen looks exactly like one that passed.** Check the
magnitude, not just the verdict: a vitest run with a bad `--reporter` flag failed at
startup and EXITED 0; a scan with a wrong cwd read zero files and reported clean;
`grep $'\x00'` is an empty pattern that matches every file. Read the exit code, the
`Test Files N passed (N)` line, AND the count against a known baseline.

**Verify a checker against a known-dirty case before believing a clean result.**
Eleven detectors in this work reported clean while blind — from `\w` excluding the
combining marks under inspection, to an unassigned codepoint splitting a
mixed-script word into two clean halves, to a heredoc mangling a detector's own
fixtures.

**Fix the thing measured, never the threshold.** A gate on the LARGEST eager chunk
was once satisfied by splitting one 502 kB chunk into four — the page still
downloaded the same bytes. The real fix made the data lazy: 502 kB → 287 kB, and no
corpus growth reaches the eager graph at all.


## HL-C214 — class (d) is MECHANIZABLE: glossed-but-never-a-headword

HL-C207 named the collision class no string search reaches — a candidate that
collides with a GLOSS rather than a token — and said mechanising it would need a
schema change. **It does not.** The Hindi round-3 tranche derived it from data
already in the corpus:

1. extract every script token appearing anywhere in the track (Hindi: 587 distinct);
2. subtract the set of tokens that are some lesson's HEADWORD;
3. what remains — **67 tokens for Hindi** — is the set of words the reader has been
   handed a meaning for without ever being taught them.

Every class-(d) collision found by hand on five previous tracks lands in that set.
Hindi's discards from it include अलमारी and चाबी (both glossed inline in a ROOM
lesson listing Portuguese loans), ठंड (glossed as the stock phrase ठंड है), मीठा
(glossed inside another word's etymology), and तिल (glossed in an OIL lesson).
Four of those were on the shortlist and would have shipped.

**This is a report, not a gate.** The set is a candidate list to READ, not to reject
automatically: the same tranche cleared मिट्टी, बीज, अनाज and टोकरी after reading
the sentences, because the gloss was of an English word, a different word, or an
absence. A rule that only rejects is a filter.

**KNOWN LIMIT, found by running it first on sanskrit:** the check operates on
SCRIPT tokens, so a word glossed only in ROMANIZATION is invisible to it. Sanskrit's
कथा was caught by a person reading `SA-C03-katham`, which glosses *kathā* as "a
telling, a story" without ever printing the Devanagari. **Run the same three steps a
second time over romanized forms**, or accept that a human still has to read the
etymology notes. The mechanized pass narrows the field; it does not close it.

Sanskrit's numbers, running it FIRST rather than last: **267 glossed-but-never-taught
tokens out of 495 distinct**. It reshaped the word list instead of invalidating it —
3 discarded (मृगः glossed outright in a ROAD lesson; कथा; शीतम् glossed inside
"himam is snow, and cold generally"), 3 turned INTO teaching (सिंहः, handed over
twice inside the names *Siṁhapura* and *Narasiṁha*; शब्दः, which three earlier
headwords END in without any of them naming it; प्रकाशः, whose काश् root a SKY
lesson had already given away), and 2 examined and kept because the colliding
sentence was an English description rather than a gloss.

Worth building as `report --glossed-not-taught <track>`. Until then, the three
steps above are cheap to run inline and catch the class that nothing else does.

## HL-C215 — two pre-existing mixed-script typos in Hindi, reported not fixed

A per-word purity sweep of all 485 files in the Hindi tree found exactly two, both
shipping into the PDF and the narration:

```
HI-C33-shubh-dopahar.md:40   "the widened सense"   DEVANAGARI SA standing in for Latin s
HI-C32-shubh-sandhya.md:54,76 "रात्रि became राat"  should be रात
```

Both render as plausible wrong text — the same class as the Telugu and Malayalam
defects already fixed under HL-C202, and invisible to any check that looks at one
script at a time.

**Left unfixed deliberately.** They sit in chapters unrelated to the tranche that
found them, and touching them would pull those chapters' book and narration hashes
into an unrelated diff. They want their own small change.

Not a defect, do not "fix": `HI-C25-din.md:56` carries Proto-Slavic `*dьnь`, whose
Cyrillic yers inside a Latin transliteration are correct scholarly notation.

## HL-C216 — the CI "appendix warnings" are multi-pass noise; the artifact is clean

Reported from reading CI logs: lots of warnings around the appendix. **Investigated;
the books are clean. Nothing to fix, and the reason is worth writing down so the
next person does not re-open it.**

### What the CI log actually contains

The books build step emits **8,541 warning lines**. Nearly all are one of four,
once per track:

```
22x  Package rerunfilecheck Warning: File `book.out' has changed
22x  Package hyperref Warning: Rerun to get /PageLabels entry
22x  LaTeX Warning: There were undefined references
22x  LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right
 9x  Package polyglossia Warning: hyphenation patterns for modern Latin
```

**These are INTERMEDIATE-PASS artifacts.** `latexmk` runs xelatex repeatedly; on the
first pass the `.aux` has no labels yet, so every cross-reference is undefined and
LaTeX asks to be re-run. By the final pass they resolve. Checked directly: the
individually-alarming `Reference 'ch:ru-script-eleven' undefined` and
`ch:fa-script-nine` both **are** defined -- one definition each, referenced 13 and 11
times -- and **do not appear undefined in any final `book.log`.**

### Why they look appendix-related

They are not. `latexmk` prints each file as it opens it, so the log interleaves

```
(./chapters/appendix-glossary.tex [97] [98] [99])
```

with warning text from the same pass. The appendix files are simply the LAST and
LONGEST things processed -- spanish's index is 4,739 lines -- so they sit next to the
end-of-pass warning burst. **Proximity in the log, not causation.**

### The measurement that matters

The repo's own scanner reads the FINAL `book.log` and counts six classes. Both
locally and in CI it reports, for all 22 tracks:

```
overfull=0 underfull=0 missing_character=0 hyperref_warning=0
duplicate_destination=0 font_substitution=0
```

The only non-zero values anywhere are spanish `font_substitution=2` (a bold-mono
face CI lacks; cosmetic, already understood) and one russian `underfull`. The
`latin: ... [over baseline]` lines in CI are the scanner's OWN self-test fixtures,
not corpus results -- worth knowing, since they look like a failing track.

### What would be a real finding

A warning that survives into the FINAL log. `data/scripts/build_all_books.sh` reads
exactly that, which is why it reports clean while the raw CI log looks alarming.
**Judge the artifact, not the transcript.**

### The one real defect it surfaced -- FIXED in this change

`ch:how-are-you` was defined twice, in spanish chapters 8 AND 9, because
`chapters.json` carried the same `label` on both. That one DID survive to the final
log, and it was not cosmetic: the index referenced that single target **eight**
times, four of them printed as *"Chapter 8, p.N"* -- but `\pageref` resolves to
whichever definition LaTeX saw last, which is chapter 9. **Every index entry
pointing at chapter 8 printed chapter 9's page number.** A reader following it lands
on the wrong page.

Chapter 9 now carries `ch:answering`. The index splits 4/4 across the two targets,
and the `multiply defined` warning -- the last real warning in the whole corpus -- is
gone. All 22 books rebuild clean.

**Worth noting how it was found:** not by the warning, which had been visible and
ignored for a long time as "pre-existing and cosmetic", but by asking what the
warning would actually DO to a reader. The warning named a duplicate label; the
defect was a wrong page number in an index.
