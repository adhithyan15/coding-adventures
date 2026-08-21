# Changelog

## Unreleased — canonical Gujarati inventory completion (HL-C09GY)

- Remove duplicate conventional **ન** and **પ** rows from the learner inventory.
- Present Gujarati as a complete 44-letter source-verified teaching sequence.

## Unreleased — cited Gujarati હ ductus (HL-C09GX)

- Replace Gujarati **હ**'s conventional metadata with a cited one-run variant.
- Show its upper loop flowing through the broad lower bowl without a lift.

## Unreleased — cited Gujarati સ ductus (HL-C09GW)

- Replace Gujarati **સ**'s conventional metadata with a cited two-run variant.
- Show its rounded loop and shoulder before the lifted tall right spine.

## Unreleased — cited Gujarati શ ductus (HL-C09GV)

- Replace Gujarati **શ**'s conventional metadata with a cited two-run variant.
- Show its upper loop and lower body before the lifted tall right spine.

## Unreleased — cited Gujarati વ ductus (HL-C09GU)

- Replace Gujarati **વ**'s conventional metadata with a cited two-run variant.
- Show its rounded body before the lifted tall right spine.

## Unreleased — cited Gujarati ળ ductus (HL-C09GT)

- Replace Gujarati **ળ**'s conventional metadata with a cited one-run variant.
- Show its left bowl flowing through the high arch into the tall right spine.

## Unreleased — cited Gujarati લ ductus (HL-C09GS)

- Replace Gujarati **લ**'s conventional metadata with a cited three-run variant.
- Show its rounded body before the lifted shoulder and tall right spine.

## Unreleased — cited Gujarati ર ductus (HL-C09GR)

- Replace Gujarati **ર**'s conventional metadata with a cited one-run variant.
- Show its rounded upper body, middle loop, and descending tail without a lift.

## Unreleased — cited Gujarati ય ductus (HL-C09GQ)

- Replace Gujarati **ય**'s conventional metadata with a cited two-run variant.
- Show its rounded body and long shoulder before the lifted tall right spine.

## Unreleased — cited Gujarati મ ductus (HL-C09GP)

- Replace Gujarati **મ**'s conventional metadata with a cited two-run variant.
- Show its left body and inner turn before the lifted tall right spine.

## Unreleased — cited Gujarati ભ ductus (HL-C09GO)

- Replace Gujarati **ભ**'s conventional metadata with a cited two-run variant.
- Show its broad loop and inner turn before the lifted tall right spine.

## Unreleased — cited Gujarati બ ductus (HL-C09GN)

- Replace Gujarati **બ**'s conventional metadata with a cited two-run variant.
- Show its rounded body and compact inner turn before the lifted tall right spine.
- Preserve the teaching app's two-path evidence and variation warning in the learner-visible metadata.

## Unreleased — the curriculum plans leave the first-paint path

CI failed the eager-bundle gate at **502,251 bytes against a 500,000 limit**.
The offender was `curriculum-plans`, a single chunk holding all 22 tracks'
`curriculum.json` — and it was on the first-paint path only because
`src/curriculum.ts` globbed them with `{ eager: true }`.

Nothing on that path reads them. The first frame is the header plus "Loading
the lessons needed for this view…"; every plan consumer runs after the awaited
corpus refresh. So half a megabyte of authored path, extension and spine ledger
was being downloaded and parsed before a screen that shows none of it.

The glob is now lazy and **one chunk per track** (`curriculum-<track>`), fetched
by `loadCurriculumPlans()` inside the corpus refresh the app already awaited.
`main.ts` derives its plan-dependent state in one place, `installCurriculumPlans()`:
the mapped-lesson-id set, and the restored Learn progress (which is pruned
against the authored paths, so it could never have been restored earlier
anyway). Which tracks are *offered* still resolves synchronously, from
`MAPPED_LANGUAGE_IDS` — the glob's KEYS are file paths, not file contents, so
that question costs no plan bytes.

**Largest eager chunk: 502,251 → 287,353 bytes**, and the ceiling stops being a
countdown: no `curriculum.json` byte is eagerly imported any more, so the daily
tranche of lessons cannot walk it back up. Per-track chunking also means a
Telugu tranche re-downloads Telugu's plan alone instead of invalidating one
shared half-megabyte blob. This is the fix HL-C110 named as the next candidate,
done the way it insisted — the bytes left the preload set, rather than being
split across several eager chunks to satisfy a gate that measures the largest
one. It supersedes the `maxSize: 250_000` briefly placed on the old
`curriculum-plans` group, which turned one 502 kB eager chunk into four smaller
ones the browser still downloaded in full before first paint.

Two things the eager import got for free and the lazy one has to earn, both
handled here. Failure is **not** memoised — caching a rejected promise would
make one dropped chunk permanent, leaving the app in its error frame until a
reload, so a failed attempt is forgotten and a retry really retries. And the
picker now offers tracks by DIRECTORY name while every lookup still resolves
them by the plan's own `language` field, so a test pins that the two agree —
a track whose folder disagreed with its `language` would otherwise appear in the
picker and resolve to nothing.

Behaviour unchanged, verified in a browser and not only in tests: 22 lazy
`curriculum-*` chunks fetch on load with no console errors, Learn shows all 22
frontier steps with their per-track lesson counts, the language picker still
reports 3,034 mapped micro-lessons and 851 extensions, Lessons and Concepts
render the full 3,120-lesson corpus, and a seeded `learn-progress` blob still
restores and advances the Spanish frontier.

## Unreleased — handwriting moves into a package

`strokes.ts`, `truetype.ts`, `ductusview.ts` and `data.ts` now live in
`@coding-adventures/script-ductus`, so the book pipeline can build the same
filmstrips as printed figures. Nothing under `code/packages/` may depend on
something under `code/programs/`, which is why they had to move for anything but
this app to use them.

No behaviour change here. `main.ts` and nine test files import from the package;
`types.ts` re-exports `Letter` and `ScriptData` from it, so every existing
`from "./types.ts"` import is unchanged.

One real catch, from `check:bundle`: `vite.config.ts`'s `handwriting-tools`
manual chunk matched those modules **by path**, so moving them silently emptied
the chunk and would have pulled 7,600 lines of handwriting code into the
interactive shell instead of loading it when a learner opens a letter. Repointed
at the package. `scriptdata` stays out of that chunk on purpose — the shell needs
`SCRIPTS` on first paint.

## Unreleased — schema-v2 lesson compatibility

### Added — cited Gujarati ફ ductus (HL-C09GM)

- Restore missing Gujarati **ફ** and render its winding body and tail before the separate diagonal cross-stroke.
- Keep the source's two-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind both filmstrip frames and queue missing **બ** next.

### Added — cited Gujarati પ ductus (HL-C09GL)

- Restore missing Gujarati **પ** and render its hooked left stem and broad lower body plus separate tall spine with one lift.
- Keep the source's two-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind both filmstrip frames and queue missing **ફ** next.

### Added — cited Gujarati ન ductus (HL-C09GK)

- Restore missing Gujarati **ન** and render its loop-and-shoulder body plus separate tall spine with one lift.
- Keep the source's two-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind both filmstrip frames and queue missing **પ** next.

### Added — cited Gujarati ધ ductus (HL-C09GJ)

- Render Gujarati **ધ** as a joined high-entry body and separate tall right spine with one lift.
- Keep the source's two-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind both filmstrip frames and queue missing **ન** next.

### Added — cited Gujarati દ ductus (HL-C09GI)

- Render Gujarati **દ** as one continuous upper-body, middle-turn, and lower-body run with no lift.
- Keep the source's zero-lift evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind the filmstrip frame and queue **ધ** next.

### Added — cited Gujarati થ ductus (HL-C09GH)

- Restore missing Gujarati **થ** and render its looped body and separate tall right spine with one lift.
- Keep the source's two-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind both filmstrip frames and queue **દ** next.

### Added — cited Gujarati ત ductus (HL-C09GG)

- Render Gujarati **ત** as an open left body and separate tall right spine with one lift.
- Keep the source's two-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind both filmstrip frames and queue missing **થ** next.

### Added — cited Gujarati ણ ductus (HL-C09GF)

- Render Gujarati **ણ** as a hooked left body, separate middle bowl, and tall right spine with two lifts.
- Keep the source's three-path evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind all filmstrip frames and queue **ત** next.

### Added — cited Gujarati ઢ ductus (HL-C09GE)

- Render Gujarati **ઢ** as one continuous high-shoulder, outer-bowl, and inner-loop run with no lift.
- Keep the source's zero-lift evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind the filmstrip frame and queue **ણ** next.

### Added — cited Gujarati ડ ductus (HL-C09GD)

- Restore missing Gujarati **ડ** and render its high shoulder, middle descent, and lower bowl as one continuous run.
- Keep the source's zero-lift evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind the filmstrip frame and queue **ઢ** next.

### Added — cited Gujarati ઠ ductus (HL-C09GC)

- Restore missing Gujarati **ઠ** and render its high shoulder, broad outer bowl, and inward terminal as one continuous run.
- Keep the source's zero-lift evidence and variation caveat alongside the learner-facing order.
- Show the exact Noto Sans Gujarati glyph behind the filmstrip frame and queue **ડ** next.

### Added — cited Gujarati ટ ductus (HL-C09GB)

- Render **ટ** as one continuous upper-turn, diagonal-middle, and lower-bowl run with no lift.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind the filmstrip frame and queue missing **ઠ** next.

### Added — cited Gujarati ઞ ductus (HL-C09GA)

- Restore missing **ઞ** to the inventory and render its left body, short rightward shoulder, and tall spine with lower terminal as three runs with two lifts.
- Preserve t30apps.com's version-1.0 three-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue conventional **ટ** for cited verification next.

### Added — cited Gujarati ઝ ductus (HL-C09FZ)

- Restore missing **ઝ** to the inventory and render its left body, right loop-and-tail, and upper stem as three runs with two lifts.
- Preserve t30apps.com's version-1.0 three-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue missing **ઞ** next.

### Added — cited Gujarati જ ductus (HL-C09FY)

- Render **જ** as one continuous left-loop, crossing-body, right-loop, and upper-right-exit run with no lift.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue missing **ઝ** next.

### Added — cited Gujarati છ ductus (HL-C09FX)

- Render **છ** as one continuous upper-left-lobe, lower-body, outer-curve, and upper-right-lobe run with no lift.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind the filmstrip frame and queue **જ** next.

### Added — cited Gujarati ચ ductus (HL-C09FW)

- Render **ચ** as one joined upper-bowl, middle-loop, and lower-body run followed by its separate right spine and lower foot with one lift.
- Preserve t30apps.com's version-1.0 two-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **છ** next.

### Added — cited Gujarati ઙ ductus (HL-C09FV)

- Restore missing **ઙ** to the inventory and render its S-like body before the separate upper-right dot with one lift.
- Preserve t30apps.com's version-1.0 two-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ચ** next.

### Added — cited Gujarati ઘ ductus (HL-C09FU)

- Restore missing **ઘ** to the inventory and render its joined upper-and-lower body before the separate right spine and lower foot with one lift.
- Preserve t30apps.com's version-1.0 two-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue the missing **ઙ** next.

### Added — cited Gujarati ગ ductus (HL-C09FT)

- Render **ગ** as a rounded left-body run followed by its separate right spine and lower foot with one lift.
- Preserve t30apps.com's version-1.0 two-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઘ** next.

### Added — cited Gujarati ખ ductus (HL-C09FS)

- Render **ખ** as a joined left-lobe-and-inner-curl run followed by its separate right spine and lower foot with one lift.
- Preserve t30apps.com's version-1.0 two-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ગ** next.

### Added — cited Gujarati ક ductus (HL-C09FR)

- Render **ક** as a joined upper-loop-to-lower-body run followed by a rising diagonal cross-stroke with one lift.
- Preserve t30apps.com's version-1.0 two-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ખ** next.

### Added — cited Gujarati ઋ ductus (HL-C09FQ)

- Restore missing Gujarati inventory entry **ઋ** and render its bent body, central stem, and right loop-and-tail as three runs with two lifts.
- Preserve t30apps.com's version-1.0 three-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue consonant **ક** next.

### Added — cited Gujarati ઔ ductus (HL-C09FP)

- Restore missing Gujarati inventory entry **ઔ** and render its body, two stems, lower arc, and higher arc as five runs with four lifts.
- Preserve t30apps.com's version-1.0 five-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue missing **ઋ** next.

### Added — cited Gujarati ઓ ductus (HL-C09FO)

- Render **ઓ** in six movements across its joined body, two separate stems, and separate high arc with three lifts.
- Preserve t30apps.com's version-1.0 four-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue missing **ઔ** next.

### Added — cited Gujarati ઐ ductus (HL-C09FN)

- Restore missing Gujarati inventory entry **ઐ** and render its body, stem, lower arc, and higher arc as four runs with three lifts.
- Preserve t30apps.com's version-1.0 four-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઓ** next.

### Added — cited Gujarati એ ductus (HL-C09FM)

- Render **એ** in four movements across its joined body, separate right stem, and separate high arc with two lifts.
- Preserve t30apps.com's version-1.0 three-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઐ** next.

### Added — cited Gujarati ઊ ductus (HL-C09FL)

- Render **ઊ** in three joined movements across the complete **ઉ** body, high shoulder, and long right-side tail with zero lifts.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **એ** next.

### Added — cited Gujarati ઉ ductus (HL-C09FK)

- Render **ઉ** in three joined movements across its small upper bowl, middle cusp, broad lower bowl, and returning outer curve with zero lifts.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઊ** next.

### Added — cited Gujarati ઈ ductus (HL-C09FJ)

- Render **ઈ** in four joined movements across its upper loop, middle crossing, lower loop, and extended top curl with zero lifts.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઉ** next.

### Added — cited Gujarati ઇ ductus (HL-C09FI)

- Render **ઇ** in four joined movements across its small upper loop, middle crossing, broad lower loop, and rising hook with zero lifts.
- Preserve t30apps.com's version-1.0 one-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઈ** next.

### Added — cited Gujarati આ ductus (HL-C09FH)

- Render **આ** in five movements across the complete **અ** sequence and added trailing ā stem with two lifts.
- Preserve t30apps.com's version-1.0 three-path teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **ઇ** next.

### Added — cited Gujarati અ ductus (HL-C09FG)

- Render **અ** in four movements across its joined body and separately descended right stem with one lift.
- Preserve t30apps.com's version-1.0 teaching order while retaining the source's explicit variation warning.
- Show the exact Noto Sans Gujarati glyph behind every filmstrip frame and queue **આ** next.

### Added — cited Cyrillic я ductus (HL-C09FF)

- Render **я** in four joined movements across its rising right stem, counterclockwise upper bowl, lower join, and diagonal leg with zero lifts.
- Preserve RussianIrina's 12:13–12:21 school-hand order while fitting the bundled printed outline.
- Show the exact Noto Sans Cyrillic glyph behind every filmstrip frame, complete the lowercase Cyrillic inventory, and queue Gujarati next.

### Added — cited Cyrillic ю ductus (HL-C09FE)

- Render **ю** in five joined movements across its left stem, middle connector, and clockwise oval with zero lifts.
- Preserve RussianIrina's 11:44–11:58 school-hand order while fitting the bundled printed outline.
- Show the exact Noto Sans Cyrillic glyph behind every filmstrip frame and queue **я** next.

### Added — cited Cyrillic э ductus (HL-C09FD)

- Render **э** in four movements across its outer backwards-C curve and lifted right-to-left middle tongue.
- Preserve RussianIrina's 11:25–11:32 outer-before-tongue order while fitting the bundled printed outline.
- Show the exact Noto Sans Cyrillic glyph behind every filmstrip frame and queue **ю** next.

### Added — cited Cyrillic ь ductus (HL-C09FC)

- Render **ь** in four joined movements across its descending stem and counterclockwise lower bowl with zero lifts.
- Preserve RussianIrina's 11:16–11:20 stem-before-bowl order while fitting the bundled printed outline.
- Show the exact Noto Sans Cyrillic glyph behind every filmstrip frame and queue **э** next.

### Added — cited Cyrillic ы ductus (HL-C09FB)

- Render **ы** in five movements across a joined left body and separately descended right stem with one lift.
- Descend the left stem, circle and close its lower bowl, lift, then descend the right stem.
- Preserve the native-teacher lesson's narrow entry loop and curled exit while fitting Noto Sans Cyrillic's straight uprights and wide closed lower bowl, and reduce measured HL-C09 debt to 74 entries.

### Added — cited Cyrillic ъ ductus (HL-C09FA)

- Render **ъ** in five joined movements across one source-aligned stroke with zero lifts.
- Sweep across the top flag, descend the main stem, then circle and close the joined lower bowl.
- Preserve the native-teacher lesson's narrow entry loop and rounded shoulder while fitting Noto Sans Cyrillic's broad top flag, straight stem, and closed lower bowl, and reduce measured HL-C09 debt to 75 entries.

### Added — cited Cyrillic щ ductus (HL-C09EZ)

- Render **щ** in six joined movements across one source-aligned stroke with zero lifts.
- Descend the left stem, traverse the joined middle and right stems, cross the tail shoulder, then descend the short right tail.
- Preserve the native-teacher lesson's rounded diagonal joins and looped exit while fitting Noto Sans Cyrillic's three straight stems, baseline bars, and short right descender, and reduce measured HL-C09 debt to 76 entries.

### Added — cited Cyrillic ш ductus (HL-C09EY)

- Render **ш** in five joined movements across one source-aligned stroke with zero lifts.
- Descend the left stem, cross the first base and rise then retrace the middle stem, cross the second base and rise then retrace the right stem.
- Preserve the native-teacher lesson's rounded diagonal joins and rising exit while fitting Noto Sans Cyrillic's three straight stems and horizontal baseline bars, and reduce measured HL-C09 debt to 77 entries.

### Added — cited Cyrillic ч ductus (HL-C09EX)

- Render **ч** in three joined movements across one source-aligned stroke with zero lifts.
- Descend the short left stem, sweep through the shallow bowl and rise along the right stem, then descend the full right stem.
- Preserve the native-teacher lesson's narrow rounded bridge and rising exit while fitting Noto Sans Cyrillic's shorter left stem, shallow bowl, and full-height right stem, and reduce measured HL-C09 debt to 78 entries.
- Cap canonical script-data batches at 250 kB so the growing cited metadata remains below the enforced 500 kB eager-chunk budget.

### Added — cited Cyrillic ц ductus (HL-C09EW)

- Render **ц** in four joined movements across one source-aligned stroke with zero lifts.
- Descend the left stem, sweep along the base and rise through the right stem, retrace to the tail shoulder, then descend the short tail.
- Preserve the native-teacher lesson's rounded stem-to-stem-to-looped-tail order while fitting Noto Sans Cyrillic's printed squared form, and reduce measured HL-C09 debt to 79 entries.

### Added — cited Cyrillic х ductus (HL-C09EV)

- Render **х** in four movements across two source-aligned strokes with one lift.
- Draw the left pair of arms through the centre crossing, lift, then draw the right pair through that same crossing.
- Preserve the native-teacher lesson's two-facing-curve school-hand order while fitting Noto Sans Cyrillic's printed X-like form, and reduce measured HL-C09 debt to 80 entries.

### Added — cited Cyrillic ф ductus (HL-C09EU)

- Render **ф** in five movements across two source-aligned strokes with one lift.
- Descend the long central stem below the baseline, lift to circle the left bowl, cross through the centre, and continue around the right bowl.
- Preserve the native-teacher lesson's stem-first linked-loop school-hand order while fitting Noto Sans Cyrillic's printed phi-like form, and reduce measured HL-C09 debt to 81 entries.

### Added — cited Cyrillic у ductus (HL-C09ET)

- Render **у** in four joined movements across one source-aligned stroke.
- Descend the left arm, rise through the right arm, retrace through the junction into the long descender, then curve left through its terminal without lifting.
- Preserve the native-teacher lesson's loop-descender school-hand order while fitting Noto Sans Cyrillic's printed y-like form, and reduce measured HL-C09 debt to 82 entries.

### Added — cited Cyrillic т ductus (HL-C09ES)

- Render **т** in three joined movements across one source-aligned stroke.
- Descend the central stem, retrace to the top junction and sweep left, then retrace through the junction and sweep to the right tip without lifting.
- Preserve the native-teacher lesson's initial descent and two-arch school-hand order while fitting Noto Sans Cyrillic's printed T-shaped form, and reduce measured HL-C09 debt to 83 entries.

### Added — cited Cyrillic с ductus (HL-C09ER)

- Render **с** in two joined movements across one source-aligned stroke.
- Curve left over the top and descend the left side, then sweep through the bottom and rise to the lower-right tip without lifting.
- Preserve the native-teacher lesson's counterclockwise open-curve order while fitting Noto Sans Cyrillic's wider upright form, and reduce measured HL-C09 debt to 84 entries.

### Added — cited Cyrillic р ductus (HL-C09EQ)

- Render **р** in three joined movements across one source-aligned stroke.
- Descend below the baseline, retrace to the upper shoulder and curve right, then sweep around the printed bowl and return to the stem without lifting.
- Preserve the native-teacher lesson's stem-before-bowl order while fitting Noto Sans Cyrillic's closed printed bowl, and reduce measured HL-C09 debt to 85 entries.

### Added — cited Cyrillic п ductus (HL-C09EP)

- Render **п** in three joined movements across one source-aligned stroke.
- Descend the left stem, retrace to the top shoulder and sweep right, then descend the right stem without lifting.
- Preserve the native-teacher lesson's rounded shoulder order while fitting Noto Sans Cyrillic's squared printed arch, and reduce measured HL-C09 debt to 86 entries.

### Added — cited Cyrillic о ductus (HL-C09EO)

- Render **о** in two joined movements across one source-aligned stroke.
- Curve left over the top and descend the left side, then sweep through the bottom and rise to close the oval without lifting.
- Preserve the native-teacher lesson's counterclockwise closure order while fitting Noto Sans Cyrillic's upright printed form, and reduce measured HL-C09 debt to 87 entries.

### Added — cited Cyrillic н ductus (HL-C09EN)

- Render **н** in three joined movements across one source-aligned stroke.
- Descend the left stem, retrace to the middle bridge and rise to the upper right, then descend the right stem without lifting.
- Preserve the native-teacher lesson's rounded bridge order while fitting Noto Sans Cyrillic's H-like printed form, and reduce measured HL-C09 debt to 88 entries.

### Added — cited Cyrillic м ductus (HL-C09EM)

- Render **м** in four joined movements across one source-aligned stroke.
- Rise through the left stem, descend to the central valley, rise to the second apex, then descend the right stem without lifting.
- Preserve the native-teacher lesson's rounded two-arch order while fitting Noto Sans Cyrillic's angular printed form, and reduce measured HL-C09 debt to 89 entries.

### Added — cited Cyrillic л ductus (HL-C09EL)

- Render **л** in three joined movements across one source-aligned stroke.
- Curve from the baseline hook up the left leg, sweep along the top shoulder, then descend the right stem without lifting.
- Preserve the native-teacher lesson's pointed hook-to-apex-to-right-leg order while fitting Noto Sans Cyrillic's block-like printed form, and reduce measured HL-C09 debt to 90 entries.

### Added — cited Cyrillic к ductus (HL-C09EK)

- Render **к** in three joined movements across one source-aligned stroke.
- Descend the left stem, rise through the upper arm and return to the middle, then continue through the lower arm without lifting.
- Preserve the native-teacher lesson's looped stem-to-upper-arm-to-lower-arm order while fitting Noto Sans Cyrillic's angular printed form, and reduce measured HL-C09 debt to 91 entries.

### Added — cited Cyrillic й ductus (HL-C09EJ)

- Render **й** in four movements across two source-aligned strokes.
- Complete the joined **и** body first, lift once, then draw the breve above from left to right.
- Preserve the native-teacher lesson's body-before-breve order while fitting Noto Sans Cyrillic's printed backwards-N body and separate curved mark, and reduce measured HL-C09 debt to 92 entries.

### Added — cited Cyrillic и ductus (HL-C09EI)

- Render **и** in three joined movements across one source-aligned stroke.
- Descend the left stem, rise through the diagonal, then descend the right stem without lifting.
- Preserve the native-teacher lesson's rounded stem-to-diagonal-to-stem order while fitting Noto Sans Cyrillic's printed backwards-N glyph, and reduce measured HL-C09 debt to 93 entries.

### Added — cited Cyrillic з ductus (HL-C09EH)

- Render **з** in two joined movements across one source-aligned stroke.
- Circle the smaller upper lobe and descend through the middle, then continue around the larger lower lobe and finish at the lower right without lifting.
- Preserve the native-teacher lesson's upper-lobe-to-lower-lobe order while fitting Noto Sans Cyrillic's compact printed double-lobe glyph, and reduce measured HL-C09 debt to 94 entries.

### Added — cited Cyrillic ж ductus (HL-C09EG)

- Render **ж** in two joined movements across one source-aligned stroke.
- Trace the left wings and rise through the centre, then retrace the central upright and continue through the right wings without lifting.
- Preserve the native-teacher lesson's rounded left-to-centre-to-right order while fitting Noto Sans Cyrillic's straight upright and diagonal arms, and reduce measured HL-C09 debt to 95 entries.

### Added — cited Cyrillic ё ductus (HL-C09EF)

- Render **ё** in four movements across three source-aligned strokes.
- Complete the joined е body, lift for the left dot, then lift again for the right dot.
- Preserve the native-teacher lesson's body-before-left-dot-before-right-dot order while fitting its tall school hand through Noto Sans Cyrillic's compact printed e and circular dots, and reduce measured HL-C09 debt to 96 entries.

### Added — cited Cyrillic е ductus (HL-C09EE)

- Render **е** in two joined movements across one source-aligned stroke.
- Curve around the upper bowl and sweep through the middle, then reverse through the junction and continue counterclockwise around the lower bowl without lifting.
- Preserve the native-teacher lesson's zero-lift looped order while fitting its tall school hand through Noto Sans Cyrillic's compact printed e, and reduce measured HL-C09 debt to 97 entries.

### Added — cited Cyrillic д ductus (HL-C09ED)

- Render **д** in two joined movements across one source-aligned stroke.
- Circle the closed body counterclockwise, then descend through the right foot, sweep across the base shelf, retrace the left foot, and finish rightward without lifting.
- Preserve the native-teacher lesson's zero-lift cursive body-to-descender order while documenting the block-font fit's shelf-and-feet replacement, and reduce measured HL-C09 debt to 98 entries.

### Added — cited Cyrillic г ductus (HL-C09EC)

- Render **г** in two joined movements across one source-aligned stroke.
- Climb from the baseline through the upright and top bar, then retrace the top and descend without lifting.
- Preserve the native-teacher lesson's zero-lift cursive evidence while documenting the block-font fit's omitted exit arch, and reduce measured HL-C09 debt to 99 entries.

### Added — cited Cyrillic в ductus (HL-C09EB)

- Render **в** in two joined movements across one source-aligned stroke.
- Climb from the baseline through the upper loop, descend to the baseline, then continue counterclockwise around the lower bowl without lifting.
- Preserve the native-teacher lesson's zero-lift school-hand order, fit its tall cursive ascender through Noto Sans Cyrillic's compact printed upper bowl and left stem, and reduce measured HL-C09 debt to 100 entries.

### Added — cited Cyrillic б ductus (HL-C09EA)

- Render **б** in two joined movements across one source-aligned stroke.
- Circle the lower body counterclockwise, then continue through the rising shoulder and rightward top flag without lifting.
- Preserve the native-teacher lesson's zero-lift body-to-flag order, fit its handwritten diagonal transition to Noto Sans Cyrillic's printed upper-left shoulder, and reduce measured HL-C09 debt to 101 entries.

### Added — cited Cyrillic а ductus (HL-C09DZ)

- Render **а** in two joined movements across one source-aligned stroke.
- Sweep through the upper shoulder and counterclockwise lower body before continuing down the right-hand finishing stem without lifting.
- Preserve the native-teacher lesson's zero-lift single-storey school hand, fit it to Noto Sans Cyrillic's double-storey printed outline, and reduce measured HL-C09 debt to 102 entries.

### Added — cited Devanagari ह ductus (HL-C09DY)

- Render **ह** in three source-aligned movements across three strokes.
- Join the descending right stem, leftward shoulder, and clockwise hooked body before the restarted down-left outer curve and down-right tail, then the final left-to-right shirorekhā.
- Preserve the animation's two lifts, fit the path to Noto Sans Devanagari, complete the source-verified Devanagari starter set, and reduce measured HL-C09 debt to 103 entries.

### Added — cited Devanagari स ductus (HL-C09DX)

- Render **स** in four source-aligned movements across four strokes.
- Join the descending left stem, central hook, and down-right diagonal tail before the restarted middle crossbar, top-to-bottom right stem, and final left-to-right shirorekhā.
- Preserve the animation's three lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 104 entries.

### Added — cited Devanagari श ductus (HL-C09DW)

- Render **श** in three source-aligned movements across three strokes.
- Join the clockwise upper loop, descending outer curve, lower loop, and down-right diagonal tail before the restarted top-to-bottom right stem and final left-to-right shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 105 entries.

### Added — cited Devanagari व ductus (HL-C09DV)

- Render **व** in three source-aligned movements across three strokes.
- Circle counterclockwise around the left loop before the restarted top-to-bottom right stem and final left-to-right shirorekhā.
- Preserve the animation's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 106 entries.

### Added — cited Devanagari ल ductus (HL-C09DU)

- Render **ल** in four source-aligned movements across four strokes.
- Curve up and clockwise around the open left loop before the restarted up-right diagonal arm, top-to-bottom right stem, and final shirorekhā.
- Preserve the loop-first animation/deskbook agreement and JackPotte's stem-first variation, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 107 entries.

### Added — cited Devanagari र ductus (HL-C09DT)

- Render **र** in three source-aligned movements across three strokes.
- Descend the stem and curl clockwise around the lower loop before the restarted down-right diagonal tail and final shirorekhā.
- Preserve the three-run animation/deskbook agreement and JackPotte's joined-body variation, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 108 entries.

### Added — cited Devanagari य ductus (HL-C09DS)

- Render **य** in four source-aligned movements across four strokes.
- Curve clockwise around the inner curl before the restarted lower bowl, top-to-bottom right stem, and final shirorekhā.
- Preserve the four-run animation/deskbook agreement and JackPotte's joined-body variation, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 109 entries.

### Added — cited Devanagari म ductus (HL-C09DR)

- Render **म** in three source-aligned movements across three strokes.
- Descend the left stem, curl clockwise around the lower loop, and sweep right through the crossbar before the top-to-bottom right stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 110 entries.

### Added — cited Devanagari भ ductus (HL-C09DQ)

- Render **भ** in three source-aligned movements across three strokes.
- Keep the clockwise upper loop, descending trunk, clockwise lower bowl, and rightward crossbar in one continuous body before the top-to-bottom right stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 111 entries.

### Added — cited Devanagari ब ductus (HL-C09DP)

- Render **ब** in four source-aligned movements across four strokes.
- Circle counterclockwise around the oval before the top-to-bottom right stem, down-right inner diagonal, and final shirorekhā.
- Preserve the source's three lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 112 entries.

### Added — cited Devanagari प ductus (HL-C09DO)

- Render **प** in three source-aligned movements across three strokes.
- Descend the left stem and curve right around the lower bowl before the top-to-bottom right stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 113 entries.

### Added — cited Devanagari न ductus (HL-C09DN)

- Render **न** in three source-aligned movements across three strokes.
- Circle clockwise around the left loop and continue right along its shoulder before the top-to-bottom right stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 114 entries.

### Added — cited Devanagari ध ductus (HL-C09DM)

- Render **ध** in four source-aligned movements across four strokes.
- Draw the upper spiral and shoulder before the separate lower bowl, top-to-bottom right stem, and final shirorekhā.
- Preserve the source's three lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 115 entries.

### Added — cited Devanagari द ductus (HL-C09DL)

- Render **द** in three source-aligned movements across three strokes.
- Descend the short stem before one continuous outer-body, inward-curl, and down-right-tail run, then finish with the left-to-right shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 116 entries.

### Added — cited Devanagari त ductus (HL-C09DK)

- Render **त** in three source-aligned movements across three strokes.
- Sweep the upper shoulder right-to-left and curve down to the open lower tip before the top-to-bottom right stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 117 entries.

### Added — cited Devanagari च ductus (HL-C09DJ)

- Render **च** in three source-aligned movements across three strokes.
- Join the short left-to-right upper bar directly to the rounded open body before the top-to-bottom right stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 118 entries.

### Added — cited Devanagari ग ductus (HL-C09DI)

- Render **ग** in three source-aligned movements across three strokes.
- Carry the counterclockwise left loop directly up its joined stem before the top-to-bottom right stem and final left-to-right shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 119 entries.

### Added — cited Devanagari क ductus (HL-C09DH)

- Render **क** in four source-aligned movements across four strokes.
- Draw the counterclockwise left bowl before the top-to-bottom central stem, clockwise right-hand arch, and final left-to-right shirorekhā.
- Preserve the source's three lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 120 entries.

### Added — cited Devanagari औ ductus (HL-C09DG)

- Render **औ** in eight source-aligned movements across seven strokes.
- Reuse आ's joined left body, separate shoulder, and two stems before the two separate upper arcs and final shirorekhā.
- Preserve the source's six lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 121 entries.

### Added — cited Devanagari ओ ductus (HL-C09DF)

- Render **ओ** in seven source-aligned movements across six strokes.
- Reuse आ's joined left body, separate shoulder, and two stems before the separate upper arc and final shirorekhā.
- Preserve the source's five lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 122 entries.

### Added — cited Devanagari ऐ ductus (HL-C09DE)

- Render **ऐ** in five source-aligned movements across four strokes.
- Reuse ए's long stem and tail plus its shorter hooked stem before the separate upper arc and final shirorekhā.
- Preserve the source's three lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 123 entries.

### Added — cited Devanagari ए ductus (HL-C09DD)

- Render **ए** in four source-aligned movements across three strokes.
- Join the long left stem to its curved shoulder and descending tail before the separate inward-hooked stem and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 124 entries.

### Added — cited Devanagari ऊ ductus (HL-C09DC)

- Render **ऊ** in four source-aligned movements across three strokes.
- Reuse उ's continuous upper-bowl/lower-loop body before the separate right-hand loop and final shirorekhā.
- Preserve the source's two lifts, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 125 entries.

### Added — cited Devanagari उ ductus (HL-C09DB)

- Render **उ** in three source-aligned movements across two strokes.
- Keep the upper bowl and lower loop in one continuous run before the separate left-to-right shirorekhā.
- Preserve the source's single lift, fit the path to Noto Sans Devanagari, and reduce measured HL-C09 debt to 126 entries.

### Added — cited Devanagari ई ductus (HL-C09DA)

- Render **ई** in six source-aligned movements across three strokes.
- Reuse इ's continuous upright, bowls, and tail before the separate upper curl and final left-to-right shirorekhā.
- Preserve the source's two lifts in Noto Sans Devanagari while identifying the modern printed sequence as one teaching form.

### Added — cited Devanagari इ ductus (HL-C09CZ)

- Render **इ** in five source-aligned movements across two strokes.
- Keep its upright, both bowls, and down-right tail in one continuous run before the final left-to-right shirorekhā.
- Preserve the source's single lift in Noto Sans Devanagari while identifying the modern printed sequence as one teaching form.

### Added — cited Devanagari आ ductus (HL-C09CY)

- Render **आ** in six source-aligned movements across five strokes.
- Preserve its joined upper-and-lower left body, separate shoulder, inner and trailing descending stems, final left-to-right shirorekhā, and four lifts in Noto Sans Devanagari.
- Carry the published traditional base-अ variation forward rather than presenting the joined modern body as universal.

### Added — cited Devanagari अ ductus (HL-C09CX)

- Render **अ** in five source-aligned movements across four strokes.
- Preserve its joined upper-and-lower left body, separate shoulder, descending stem, left-to-right shirorekhā, and three lifts in Noto Sans Devanagari.
- Record the six-stroke traditional Sanskrit form as explicit source variation.

### Added — cited Chinese 上 ductus (HL-C09CW)

- Render **上** in three source-aligned movements across three strokes.
- Preserve its vertical-first, short-before-long horizontal order and two lifts in Noto Sans SC.

### Added — cited Chinese 早 ductus (HL-C09CV)

- Render **早** in seven source-aligned movements across six strokes.
- Preserve its complete 日-before-十 order, joined top-right turn, and five lifts in Noto Sans SC.

### Added — cited Chinese 么 ductus (HL-C09CU)

- Render **么** in four source-aligned movements across three strokes.
- Preserve its joined falling-to-rightward sweep and two lifts in Noto Sans SC.

### Added — cited Chinese 什 ductus (HL-C09CT)

- Render **什** in four source-aligned movements across four strokes.
- Preserve its complete 亻-before-十 order and three lifts in Noto Sans SC.

### Added — cited Chinese 见 ductus (HL-C09CS)

- Render **见** in seven source-aligned movements across four strokes.
- Preserve the frame-before-legs order, all three joined turns, and three lifts in Noto Sans SC.

### Changed — Spanish reaches 207 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 205 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 201 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 195 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 194 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 191 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 190 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 189 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 186 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Changed — Spanish reaches 182 chapters

- Re-pin the Spanish chapter table from the generated manifest.

### Fixed — the bundle staleness guard no longer trips on book builds

- Count only files the bundler can import when deciding whether `dist/` is stale.
  A local LaTeX run rewrites `.log`, `.aux` and `.pdf` under the corpus tree, which
  left the guard permanently tripped — teaching the exact habit it exists to prevent.
- Re-pin the Spanish chapter table from the generated manifest: 181 chapters.
### Added — cited Chinese 再 ductus (HL-C09CR)

- Render **再** in eight source-aligned movements across six strokes.
- Preserve the joined frame, close-last order, and five lifts in Noto Sans SC.

### Added — cited Chinese 请 ductus (HL-C09CQ)

- Render **请** in fourteen source-aligned movements across ten strokes.
- Preserve 讠-before-青 order, all four joined turns, and nine lifts in Noto Sans SC.
### Added — cited Chinese 谢 ductus (HL-C09CP)

- Render **谢** in seventeen source-aligned movements across twelve strokes.
- Preserve 讠-before-身-before-寸 order, all five joined turns, and eleven lifts in Noto Sans SC.

### Added — cited Chinese 字 ductus (HL-C09CO)

- Render **字** in nine source-aligned movements across six strokes.
- Preserve 宀-before-子 order, all three joined turns, and five lifts in Noto Sans SC.

### Added — cited Chinese 名 ductus (HL-C09CN)

- Render **名** in eight source-aligned movements across six strokes.
- Preserve 夕-before-口 order, both joined turns, and five lifts in Noto Sans SC.

### Added — cited Chinese 不 ductus (HL-C09CM)

- Render **不** in four separately placed source-aligned strokes.
- Preserve the top-horizontal-first order and three lifts in Noto Sans SC.
### Added — cited Chinese 是 ductus (HL-C09CL)

- Render **是** in ten source-aligned movements across nine strokes.
- Preserve 日-first order, its joined corner, and eight lifts in Noto Sans SC.

### Added — cited Chinese 我 ductus (HL-C09CK)

- Render **我** in nine source-aligned movements across seven strokes.
- Preserve the hooked vertical, six lifts, and the Noto Sans SC outline fit.

### Changed — Spanish runs 1..178 (HL-C88)

- Re-pin the Spanish book-hash table for the five friends chapters inserted at
  23; chapters after them shift by five.

### Changed — Spanish runs 1..173 (HL-C107)

- Re-pin the Spanish book-hash table for the pronoun-evidence chapter inserted
  at 122; chapters after it shift by one.

### Added — cited Chinese 好 ductus (HL-C09CJ)

- Render **好** in nine source-aligned frames: all three 女 strokes first,
  followed by 子's joined top turn, joined vertical hook, and middle horizontal.
- Fit all six source runs to Noto Sans SC while preserving three internal
  joins, five pen lifts, and component order.
- Reduce Chinese's remaining verified-ductus inventory to 13 entries.

### Added — voice playback of a lesson (HL-C87, HL10 §10.2)

- Add `voicescript.ts`: flatten a chapter's generated narration into an ordered
  list of speak / wait / respond instructions. It parses nothing — the
  narration generator already emits typed segments.
- Add `voiceplayer.ts`: drive that script through `speechSynthesis`, with a
  per-track BCP-47 tag and a cancel token, because `cancel()` fires pending
  `onend` handlers and a naive player resumes after being stopped.
- Add `narration-sources.ts`, loading one chapter's narration on demand so 344
  files per track stay out of the eager chunk.
- Show a "Play this lesson aloud" control on the frontier card, with a line of
  what is being said; leaving Learn mode stops the audio.
- Recognition is deliberately not wired: it needs a microphone permission and a
  device to verify against, and a `respond` step waits its authored budget
  instead.

### Added — cited Chinese 你 ductus (HL-C09CI)

- Render **你** in nine source-aligned frames: write 亻, then 尔's falling
  stroke, two joined hooks, and separately placed lower dots.
- Fit all seven source runs to Noto Sans SC while preserving both joined hooks,
  six pen lifts, and component order.
- Reduce Chinese's remaining verified-ductus inventory to 14 entries.

### Added — synthesis drills generated from held atoms (HL-C87, HL10 §10.3)

- Add `synthesisdrill.ts`: 2–4 pieces the learner holds, drawn from different
  domains, with an instruction to combine them. The pieces have been practised;
  the combination has not.
- Gate on mastery — the forward-reference rule run backwards: no drill uses an
  atom the learner does not currently hold.
- Derive domains from the corpus's own `concept_tag` prefixes, and exclude
  grammar-only tags, because "use a rule in a sentence" is not an instruction.
- Check honestly: report which pieces the answer contained, and say plainly
  that whether the surrounding sentence is good Spanish is not something the
  check can judge.
- Load the full corpus in the background before offering a drill. Learn mode
  holds only the frontier and completed lessons — two Spanish lessons for a
  beginner — so a drill built from those can never find two domains.

### Added — cited Chinese 宀 ductus (HL-C09CH)

- Render **宀** in four source-aligned frames: draw its top dot, restart for the
  left-side drop, then restart for a roof that hooks down-left without breaking.
- Fit all three source runs to Noto Sans SC while preserving the joined final
  hook, two pen lifts, and the source order.
- Reduce Chinese's remaining verified-ductus inventory to 15 entries.

### Added — the app schedules from atom strength (HL-C87, HL10 §10.1)

- Add `atomschedule.ts`: greedy set cover over the learner's due atoms, picking
  the completed lessons that refresh the most of them. Deterministic — ties
  break on lesson id, so the queue never reshuffles between renders.
- Show a **Due for review** section at the top of Learn, naming what each pick
  refreshes rather than asking for review without a reason.
- Fix a credit/schedule mismatch found by driving the built app: scheduling read
  only activity `assesses` while a meaning check credits `introducesAtoms`, so a
  lesson with no authored activity — including the first lesson of the Spanish
  course — could come due and never be clearable. `refreshesOf()` now unions
  both, with a test pinning the invariant.

### Added — cited Chinese 氵 ductus (HL-C09CG)

- Render **氵** in four source-aligned frames: draw its two falling dots, then
  restart for a bottom stroke that turns slightly left and rises to the right.
- Fit all three source runs to Noto Sans SC while preserving the joined final
  turn, two pen lifts, and the upper-to-lower order.
- Reduce Chinese's remaining verified-ductus inventory to 16 entries.

### Added — per-atom mastery, recorded (HL-C87, HL10 §10.1)

- Add `atommastery.ts`: a pure engine for what this learner holds atom by atom.
  Strength moves asymptotically on a hit and multiplicatively on a miss, decays
  with a 10-day half-life, and schedules on a cubic interval.
- Add `masterystore.ts`, persisting it under its own key in the `reviewstore.ts`
  style — untrusted blob, every field clamped, bad rows dropped.
- Carry `introducesAtoms` on the browser `Lesson`, so a meaning check can credit
  what the lesson exists to teach.
- Credit both answer paths: an authored activity credits its own `assesses`
  list; a meaning check credits the lesson's introduced atoms.
- Nothing schedules from this yet. The record is deliberately made trustworthy
  before anything is allowed to depend on it.

### Added — cited Chinese 讠 ductus (HL-C09CF)

- Render **讠** in four source-aligned frames: draw the dot, then restart for a
  horizontal that turns down and rises to the upper right without breaking.
- Fit both source runs to Noto Sans SC while preserving both turns inside the
  second stroke and one pen lift.
- Reduce Chinese's remaining verified-ductus inventory to 17 entries.

### Changed — Spanish runs 1..172 (HL-C105)

- Re-pin the Spanish book-hash table for the five `hay` chapters appended at
  168, completing the HL10 rung audit.

### Added — cited Chinese 日 ductus (HL-C09CE)

- Render **日** in five source-aligned frames: descend the left side, cross the
  top and turn down the right without lifting, draw the middle, then close.
- Fit all four source runs to Noto Sans SC while preserving the joined corner,
  three pen lifts, and the inside-before-close rule.
- Reduce Chinese's remaining verified-ductus inventory to 18 entries.

### Changed — Spanish runs 1..167 (HL-C105)

- Re-pin the Spanish book-hash table for the five relative-clause chapters
  appended at 163.

### Added — cited Chinese 子 ductus (HL-C09CD)

- Render **子** in five source-aligned frames: cross the top and turn down-left,
  restart for the central descent and joined base hook, then cross the middle.
- Fit all three source runs to Noto Sans SC while preserving both internal
  turns, two pen lifts, and the left-to-right final héng.
- Reduce Chinese's remaining verified-ductus inventory to 19 entries.

### Changed — Spanish runs 1..162 (HL-C105)

- Re-pin the Spanish book-hash table for the five impersonal-se chapters
  appended at 158.

### Added — cited Chinese 女 ductus (HL-C09CC)

- Render **女** in four source-aligned frames: descend left, turn and sweep
  down-right without lifting, restart for the left-falling stroke, then cross
  the middle with a separately started horizontal.
- Fit all three source runs to Noto Sans SC while preserving the first internal
  turn, two pen lifts, and left-to-right final héng.
- Reduce Chinese's remaining verified-ductus inventory to 20 entries.

### Changed — Spanish runs 1..157 (HL-C105)

- Re-pin the Spanish book-hash table for the five por/para chapters appended at
  153. No renumbering: the arc lands at the end of the course.

### Added — cited Chinese 口 ductus (HL-C09CB)

- Render **口** in four source-aligned frames: left side, top bar, the joined
  turn down the right side, and a separately closing bottom bar.
- Fit all three source runs to Noto Sans SC while preserving the héngzhé corner,
  two pen lifts, and close-last rule.
- Reduce Chinese's remaining verified-ductus inventory to 21 entries.

### Changed — Spanish runs 1..152 (HL-C105)

- Re-pin the Spanish book-hash table for the six command chapters inserted at
  109; chapters after them shift by six.

### Added — cited Chinese 亻 ductus (HL-C09CA)

- Render **亻** in two source-aligned frames: draw the long left-falling piě,
  then lift once for the vertical shù from the central junction.
- Fit the radical's own pinned medians independently to its narrow Noto Sans SC
  outline instead of deriving them from the full 人 form.
- Reduce Chinese's remaining verified-ductus inventory to 22 entries.

### Changed — Spanish runs 1..146 (HL-C105)

- Re-pin the Spanish book-hash table for the six present-perfect chapters
  inserted at 94; chapters after them shift by six.

### Added — cited Chinese 人 ductus (HL-C09BZ)

- Render **人** in two evidence-aligned frames: left-falling piě first, then
  lift once and draw right-falling nà from the central junction.
- Pin the per-character Hanzi Writer Data record derived from Make Me a Hanzi's
  documented PRC stroke order, while fitting its medians to Noto Sans SC.
- Establish a reusable source convention for the remaining Chinese inventory.

### Fixed — the lesson-source map leaves the eager chunk (HL-C110)

- Move `import.meta.glob` for lesson sources into `src/lesson-sources.ts` and
  reach it with `import()`. The map compiles to one entry per lesson — a full
  path plus a preload wrapper, ~200 bytes each — and at 1,793 lessons it was
  **363,818 bytes of the eager chunk**, growing with every lesson added.
- `bundledLessonIds()` and `loadBundledLessons()` are now async. The single
  synchronous consumer was already inside an async function.
- Largest eager chunk: **498,326 → 381,819 bytes**. It is now `script-data`,
  which is per-script and does not grow with the lesson corpus.

### Added — cited Hebrew Tav ductus (HL-C09BY)

- Render **ת** in four evidence-aligned frames: draw the top bar and right side
  together, then lift once for the separate left leg and its small foot.
- Preserve the adjacent purple cursive form's continuous retrace and right arch
  while keeping the printed form's two runs explicit.
- Close the Hebrew inventory and reprioritize to the smallest actionable script.

### Changed — Spanish runs 1..140 (HL-C105)

- Re-pin the Spanish book-hash table for the five preterite/imperfect chapters
  inserted at 89; chapters after them shift by five.

### Added — cited Hebrew Shin ductus (HL-C09BX)

- Render **ש** in three evidence-aligned frames: descend the right branch and
  round through the base into the left branch, then lift once for the middle.
- Preserve the adjacent purple cursive form's compact one-run loop while keeping
  the printed form's two runs explicit.
- Queue the adjacent Tav demonstration as the final Hebrew inventory entry.

### Fixed — the eager bundle, and the check that could not see it (HL-C110)

- Load the book-hash manifest (136 kB) and all 22 chapter ledgers (580 kB)
  **lazily**. They were statically imported to compute one diagnostic word in a
  metadata line, and together they were the largest eager chunk in the app.
- Add `whenBookHashesReady()`; the status reads `not-generated` until the data
  lands, and `main.ts` re-renders when it does.
- Derive the eager chunk set from `dist/index.html` — the entry script plus
  every `modulepreload` — instead of a hardcoded name pattern that went stale
  the moment a chunk became lazy.
- Refuse to report on a stale `dist/`: if any source or corpus file is newer
  than the built entry HTML, `check:bundle` exits non-zero.
- Largest eager chunk: 497,216 bytes against the 500,000 ceiling.

### Changed — Spanish runs 1..135 (HL-C105)

- Re-pin the Spanish book-hash table for the five double-object chapters
  inserted at 76; chapters after them shift by five.

### Added — cited Hebrew Resh ductus (HL-C09BW)

- Render **ר** in two evidence-aligned frames: draw the top bar left-to-right,
  round its outer corner, and descend the right side without lifting.
- Preserve the adjacent purple cursive form's rounder one-run hook while keeping
  the printed form's continuous motion explicit.
- Queue the adjacent Shin demonstration as the next counted inventory entry.

### Changed — Spanish runs 1..130 (HL-C105)

- Re-pin the Spanish book-hash table for the five indirect-object chapters
  inserted at 71; chapters after them shift by five.

### Added — cited Hebrew Qof ductus (HL-C09BV)

- Render **ק** in three evidence-aligned frames: draw the top and slanted right
  body together, then lift once for the separate below-line stem.
- Preserve the adjacent purple cursive form's one-run hook while keeping the
  printed form's lift and descender explicit.
- Queue the adjacent Resh demonstration as the next counted inventory entry.

### Changed — Spanish runs 1..125 (HL-C105)

- Re-pin the Spanish book-hash table for the five plural object-pronoun
  chapters inserted at 66; chapters after them shift by five.

### Added — cited Hebrew Tsadi ductus (HL-C09BU)

- Render **צ** in three evidence-aligned frames: descend the long diagonal into
  the leftward base, then lift once for the short upper-right arm.
- Preserve the adjacent purple cursive form's compact one-run shape while
  keeping the printed form's lift explicit.
- Record that intervening final Tsadi is already a form, then queue the later
  Qof demonstration as the next counted inventory entry.

### Added — cited Hebrew Pe ductus (HL-C09BT)

- Render **פ** in four evidence-aligned frames: draw the top, right side, and
  returning base in one run, then lift once for the short inner curl.
- Preserve the adjacent purple cursive form's one-run inward spiral while
  keeping the printed form's lift explicit.
- Record that the intervening final Pe is already a form, then queue the later
  Tsadi demonstration as the next counted inventory entry.

### Added — cited Hebrew Ayin ductus (HL-C09BS)

- Render **ע** in three evidence-aligned frames: descend the right branch into
  the base, sweep left, then turn back and climb the left branch without lifting.
- Preserve the adjacent purple cursive form's compact loop while keeping the
  source's single printed run explicit.
- Queue Pe from the same full-alphabet source.

### Added — cited Hebrew Samekh ductus (HL-C09BR)

- Render **ס** in four evidence-aligned frames: draw the flat top, round down
  the right side, sweep left along the base, and climb to close without lifting.
- Preserve the adjacent purple cursive form's rounder oval while keeping the
  source's single clockwise printed run explicit.
- Repair the README's Hebrew path inventory and queue Ayin from the same source.

### Added — cited Hebrew Nun ductus (HL-C09BQ)

- Replace the queued expository Nun video with Aural Writing's auditable
  print/cursive source.
- Render **נ** in three evidence-aligned frames: draw the small head, continue
  down the right side, and turn left along the base without lifting.
- Preserve the rounder purple cursive hook and queue Samekh from the same source.

### Changed — Spanish runs 1..120 (HL-C108)

- Re-pin the Spanish book-hash table for the five plural-article chapters
  inserted at 56; chapters after them shift by five.

### Added — cited Hebrew Mem ductus (HL-C09BP)

- Render **מ** in five evidence-aligned frames: draw the detached angled left
  part, lift once, then join the upper shoulder, right side, and base.
- Keep the source's open two-stroke print order explicit while preserving its
  narrow N-like handwritten alternative.
- Queue the independently published Nun lesson (`3gYCaDgB-Nk`) next.

### Changed — Spanish runs 1..115 (HL-C106, HL-C105)

- Re-pin the Spanish book-hash table for the three noun chapters and the five
  object-pronoun chapters inserted at 53; chapters after them shift by eight.

### Added — cited Hebrew Lamed ductus (HL-C09BO)

- Render **ל** in three evidence-aligned frames: descend the tall left stroke,
  continue right along the middle bar, and turn diagonally down-left without lifting.
- Keep the source's one-stroke angular print order explicit while preserving its
  rounded looping handwritten alternative.
- Queue the same lesson's Mem demonstration next.

### Changed — Spanish runs 1..107 (HL-C105)

- Re-pin the Spanish book-hash table for the consolidation chapter that derives
  the remaining irregular plurals; chapters after it shift by one.

### Added — cited Hebrew Kaf ductus (HL-C09BN)

- Render **כ** in three evidence-aligned frames: draw the top bar left-to-right,
  continue down the rounded right side, and turn left along the base without lifting.
- Keep the source's one-stroke printed corners explicit while preserving its
  rounded handwritten half-circle alternative.
- Queue the series' Lamed/Mem lesson (`CBU6aSCcPrE`) next.

### Changed — Spanish runs 1..106 (HL-C105)

- Regenerate the `bookhashes` chapter-lesson pin: `tener` and `ir` gained plural
  chapters, so old Spanish chapters 43–104 shifted to 45–106.
- Eager bundle 487,797 / 500,000 bytes.

### Added — cited Hebrew Yod ductus (HL-C09BM)

- Render **י** in two evidence-aligned frames: draw its tiny head left-to-right
  and continue down the short stem without lifting.
- Keep the source's one-stroke printed angle explicit while preserving its
  comma-like handwritten alternative.
- Queue the series' dedicated Kaf lesson (`EcQ0gL-NM-k`) next.

### Changed — Spanish runs 1..104 (HL-C105)

- Regenerate the `bookhashes` chapter-lesson pin: `estar` gained a plural
  chapter beside `ser`'s, so old Spanish chapters 42–103 shifted to 43–104.
- Eager bundle 486,989 / 500,000 bytes.

### Added — cited Hebrew Tet ductus (HL-C09BL)

- Render **ט** in four evidence-aligned frames: descend the left side and turn
  right along the base, then restart once at the lower-right, climb, and hook inward.
- Keep the source's printed two-stroke order explicit while preserving its unusual
  bottom-up, one-run rounded handwritten alternative.
- Queue the same lesson's Yod demonstration next.

### Added — cited Hebrew Heit ductus (HL-C09BK)

- Render **ח** in three evidence-aligned frames: draw the top bar left-to-right
  and continue down the right side, then restart once for the joined left leg.
- Keep the source's sharp printed two-stroke order explicit while preserving its
  rounded handwritten alternative in the source variation note.
- Queue the series' Tet/Yod lesson (`NBUtBPVKchk`) next.

### Added — cited Hebrew Zayin ductus (HL-C09BJ)

- Render **ז** in two evidence-aligned frames: draw the short head left-to-right
  and continue down through the curved stem without lifting.
- Preserve the source's rounded handwritten single run while adapting it to Noto
  Sans Hebrew and recording its contrast with mirrored Gimel and narrower Vav.
- Queue the same lesson's Heit demonstration next.

### Added — cited Hebrew Vav ductus (HL-C09BI)

- Render **ו** in two evidence-aligned frames: draw the small head left-to-right
  and continue straight down the stem without lifting.
- Keep the source's explicit one-stroke, top-to-bottom order while recording its
  simpler handwritten variation and excluding later vowel marks from base ו.
- Queue the series' Zayin/Heit lesson (`XTqG_1dsFSU`) next.

### Changed — Spanish runs 1..103 (HL-C105)

- Regenerate the `bookhashes` chapter-lesson pin: `ser` gained a plural chapter,
  so old Spanish chapters 41–102 shifted to 42–103.
- Eager bundle 486,164 / 500,000 bytes.

### Added — cited Hebrew Hei ductus (HL-C09BH)

- Render **ה** in three evidence-aligned frames: join the left-to-right top bar
  to the right descent, then restart once for the detached left leg.
- Keep the source's printed two-stroke, one-lift order explicit while recording
  its curved handwritten alternative.
- Queue the series' dedicated Vav/Hirik/Shuruk lesson next.

### Changed — Spanish runs 1..102, and a sentinel that kept rotting (HL-C105)

- Regenerate the `bookhashes` chapter-lesson pin: the `-er`/`-ir` plurals added
  four chapters, so old Spanish chapters 34–98 shifted to 38–102.
- The "no such chapter" sentinel moves 99 → 9999. It had already moved 42 → 99
  for the same reason; 9999 cannot become a real chapter.
- Eager bundle 485,353 / 500,000 bytes.

### Added — cited Hebrew Dalet ductus (HL-C09BG)

- Render **ד** in two evidence-aligned frames: draw the top bar left-to-right,
  then continue around the sharp right heel and down without lifting.
- Preserve the source's explicit one-curve, zero-lift cursive order while
  recording its adaptation to Noto Sans Hebrew's angular block outline.
- Queue the series' dedicated Hei lesson next.

### Changed — Spanish runs 1..98 (HL-C105)

- Regenerate the `bookhashes` chapter-lesson pin: the `-ar` present plural added
  five chapters, so old Spanish chapters 29–93 shifted to 34–98.
- Eager bundle 484,346 / 500,000 bytes.

### Added — cited Hebrew Gimel ductus (HL-C09BF)

- Render **ג** in four evidence-aligned frames: join the short top bar, right
  stem, and short lower-right leg, then restart once for the longer left leg.
- Keep the source's printed two-stroke, one-lift order explicit while recording
  its visibly different rounded cursive Gimel as a handwriting variation.
- Queue the same lesson's one-curve Dalet demonstration next.

### Changed — Spanish runs 1..93 (HL-C100)

- Regenerate the `bookhashes` chapter-lesson pin: a synthesis chapter now closes
  the fourteen-chapter vocabulary run, so old chapters 70–92 shifted to 71–93.
- Eager bundle 483,296 / 500,000 bytes.

### Added — cited Hebrew Bet ductus (HL-C09BE)

- Render **ב** in three evidence-aligned frames: draw the top bar into the right
  descent without lifting, then restart once for the left-to-right baseline.
- Keep the optional dagesh outside base U+05D1's one-lift count while fitting
  the source's block-style handwriting to the vendored Noto Sans Hebrew outline.
- Queue the series' dedicated Gimel/Dalet lesson as the next source-recovery
  tranche.

### Changed — Spanish runs 1..92 (HL-C104)

- Regenerate the `bookhashes` chapter-lesson pin: `un`/`una` entered as a new
  chapter 3, so old Spanish chapters 3–91 shifted to 4–92.
- Eager bundle 483,055 / 500,000 bytes.

### Added — cited Hebrew Alef ductus (HL-C09BD)

- Render **א** in three evidence-aligned frames: draw the main descending
  diagonal, lift once, then carry the opposing run from the upper-right arm
  through the crossing and down the lower-left leg.
- Keep the dedicated lesson's two pen-down runs and one lift explicit while
  fitting its compact X-like handwriting to the vendored Noto Sans Hebrew
  block outline.
- Record the blocked Arabic Mim and Nun sources and the recovered future Faa
  inventory source before reprioritizing to Hebrew.

### Changed — Spanish runs 1..91 (HL-C100)

- Regenerate the `bookhashes` chapter-lesson pin: the preterite split into three,
  so old Spanish chapters 40–89 shifted to 42–91.
- Eager bundle 482,464 / 500,000 bytes.

### Added — cited Arabic independent-waw ductus (HL-C09AZ)

- Render **و** in two evidence-aligned frames: close the small head loop from
  its lower-right junction, then continue down and left through the tail without
  lifting.
- Keep the directly linked MOV's one pen-down run, zero lifts, one-way-connector
  context, and consonant/long-vowel roles explicit while fitting Noto Naskh.
- Preserve Arabic Waw independently of the existing Persian record for the same
  Unicode glyph, and queue source recovery for independent Arabic **م** next.

### Changed — Spanish runs 1..89 (HL-C100)

- Regenerate the `bookhashes` chapter-lesson pin: the imperfect split into four,
  so old Spanish chapters 41–86 shifted to 44–89.
- Eager bundle 482,224 / 500,000 bytes.

### Changed — Spanish runs 1..86 (HL-C100)

- Regenerate the `bookhashes` chapter-lesson pin: the subjunctive chapter split
  into five, so old Spanish chapters 46–82 shifted to 50–86.
- Eager bundle 482,015 / 500,000 bytes.

### Changed — Spanish runs 1..82 (HL-C100)

- Regenerate the `bookhashes` chapter-lesson pin: the future+conditional chapter
  split into four, so old Spanish chapters 42–79 shifted to 45–82.
- Eager bundle 481,805 / 500,000 bytes.

### Added — cited Arabic independent-heh ductus (HL-C09AY)

- Render **ه** in three evidence-aligned frames: close the lower counter, thread
  through the centre into the upper-right counter, then sweep left along the
  baseline without lifting.
- Keep the directly linked MOV's one pen-down run, zero lifts, and
  two-way-connector context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic Heh independently of the existing Persian record for the same
  Unicode glyph, and queue the page's directly linked independent **و** source.

### Added — cited Arabic independent-yaa ductus (HL-C09AX)

- Render **ي** in four evidence-aligned frames: descend into the independent
  bowl, sweep left without lifting, then restart for the lower-left and
  lower-right dots in that order.
- Keep the directly linked MOV's three pen-down runs, two lifts, and
  two-way-connector context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic Yaa U+064A independently of Urdu Ye U+06CC, whose isolated
  body has no lower dots and retains separate provenance.

### Added — cited Arabic independent-lam ductus (HL-C09AW)

- Render **ل** in two evidence-aligned frames: descend the tall upright, then
  continue left through the base bowl without lifting.
- Keep the directly linked MOV's one pen-down run, zero lifts, and
  two-way-connector context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic Lam independently of the existing Persian and Urdu records
  for the same Unicode glyph, and queue the page's directly linked **ي** source.

### Changed — Spanish runs 1..79 (HL-C101)

- Regenerate the `bookhashes` chapter-lesson pin: `espanol` and the first built
  sentence moved ahead of the `-ar` synthesis chapter.

### Changed — Spanish runs 1..78 (HL-C99)

- Regenerate the `bookhashes` chapter-lesson pin: `trabajar` and `estudiar` took
  a chapter each, so old Spanish chapters 21–76 shifted to 23–78.

### Added — cited Arabic independent-kaf ductus (HL-C09AV)

- Render **ك** in three evidence-aligned frames: descend the main upright, turn
  left along the baseline without lifting, then restart once for the inner arm.
- Keep the directly linked MOV's two pen-down runs, one lift, and
  two-way-connector context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic Kaf independently of Urdu **ک**'s different Unicode glyph and
  provenance, and queue the same page's directly linked independent **ل** source.

### Changed — Spanish runs 1..76 (HL-C99)

- Regenerate the `bookhashes` chapter-lesson pin: chapter 30 split into five, so
  old Spanish chapters 31–72 shifted to 35–76.
- Eager bundle 481,588 / 500,000 bytes.

### Added — cited Arabic independent-ayn ductus (HL-C09AU)

- Render **ع** in two evidence-aligned frames: shape its open head from the
  upper-right tip, then continue down and around the broad lower bowl.
- Keep the directly linked MOV's one pen-down run, zero lifts, and
  two-way-connector context explicit while fitting the vendored Noto Naskh outline.
- Preserve Ayn independently of adjacent dotted Ghayn and queue the directly
  linked independent **ك** source next.

### Changed — Spanish runs 1..72 (HL-C99)

- Regenerate the `bookhashes` chapter-lesson pin: chapter 21 split into four, so
  old Spanish chapters 22–69 shifted to 25–72.
- Eager bundle 481,152 / 500,000 bytes.

### Changed — Spanish runs 1..69 (HL-C99)

- Regenerate the `bookhashes` chapter-lesson pin: chapter 62 split into six, so
  old Spanish chapters 63–64 shifted to 68–69.
- Eager bundle 480,127 / 500,000 bytes.

### Added — cited Arabic independent-daad ductus (HL-C09AT)

- Render **ض** in four evidence-aligned frames: its clockwise oval, joined
  shoulder, separately restarted trailing bowl, and separately placed upper dot.
- Keep the embedded lesson's three pen-down runs, two lifts, and
  two-way-connector context explicit while fitting the vendored Noto Naskh outline.
- Record the direct short MOV's audit-time 403 honestly, preserve the accessible
  embedded primary evidence, and queue the source-backed **ع** lesson next.

### Changed — Spanish runs 1..64 (HL-C99)

- Regenerate the `bookhashes` chapter-lesson pin: chapter 53 split into six, so
  old Spanish chapters 54–59 shifted to 59–64.
- Eager bundle 479,716 / 500,000 bytes.

### Changed — Spanish runs 1..59 (HL-C99)

- Regenerate the `bookhashes` chapter-lesson pin: chapter 47's four mind-verbs
  each took a chapter, plus a review and a synthesis chapter, so old Spanish
  chapters 48–54 shifted to 53–59.
- Eager bundle 479,308 / 500,000 bytes.

### Added — cited Arabic independent-saad ductus (HL-C09AS)

- Render **ص** in three evidence-aligned frames: its clockwise oval, the joined
  rise into the short shoulder, and the separately restarted trailing bowl.
- Keep the source's two pen-down runs, one lift, and two-way-connector context
  explicit while fitting the vendored Noto Naskh outline.
- Preserve Saad's evidence independently of adjacent Seen and Shiin, with the
  same page's directly linked Daad MOV queued next.

### Changed — Spanish runs 1..54 (HL-C98)

- Regenerate the `bookhashes` chapter-lesson pin from the lesson files: the
  first paradigm became five chapters (one grammar cell each, plus a review and
  a synthesis chapter), so old Spanish chapters 16–50 shifted to 20–54. This pin
  lives in the consumer, so the data package's own suite passes while this one
  fails — which is exactly why the app is built in CI.
- Eager bundle 478,900 / 500,000 bytes.

### Added — cited Arabic independent-shiin ductus (HL-C09AR)

- Render **ش** in five evidence-aligned frames: two joined body movements,
  followed by the lower-left, lower-right, and centered upper dots.
- Keep the source's four pen-down runs, three lifts, and two-way-connector
  context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic provenance independently of Urdu **ش**, with the same page's
  directly linked Saad MOV queued next.

### Added — cited Arabic independent-seen ductus (HL-C09AQ)

- Render **س** in two evidence-aligned frames: three close teeth shaped
  right-to-left, then the connected flow into the final bowl.
- Keep the source's single pen-down run, zero lifts, and two-way-connector
  context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic provenance independently of Persian and Urdu **س**, with the
  same page's directly linked Shiin MOV queued next.

### Added — cited Arabic independent-raa ductus (HL-C09AP)

- Render **ر** in two evidence-aligned frames: the upper-tip descent through its
  short stroke, then the connected sweep left through the lower curve.
- Keep the source's single pen-down run, zero lifts, and one-way-connector
  context explicit while fitting the vendored Noto Naskh outline.
- Preserve Arabic provenance independently of Urdu **ر**, with the next
  measured Arabic **س** queued from its page's directly linked MOV source.

### Added — cited Arabic independent-daal ductus (HL-C09AO)

- Render **د** in two evidence-aligned frames: the upper-tip descent through its
  curved shoulder, then the connected turn left along the baseline.
- Preserve the source's single pen-down run with zero lifts while fitting the
  isolated Noto Naskh outline.
- Keep one-way-connector context and script-scoped Arabic provenance, with the
  same page's directly linked Raa demonstration queued next.

### Added — cited Arabic independent-khaa ductus (HL-C09AN)

- Resolve the page's Khaa QuickTime attachment and render **خ** in three
  evidence-aligned frames: its short upper head, continuous bowl, and lifted
  upper dot.
- Keep Khaa's body-first motion distinct from Haa's restarted stem while fitting
  the same isolated Noto Naskh skeleton as adjacent Jeem and Haa.
- Preserve two-way-connector context and script-scoped Arabic provenance, with
  Daal queued from the next directly linked source.

### Added — cited Arabic independent-haa ductus (HL-C09AM)

- Resolve the source page's hidden Haa attachment and render **ح** in three
  evidence-aligned frames: its short left stem, a lifted restart near the stem's
  top, and the continuous dotless bowl.
- Keep the one-lift, stem-first motion distinct from adjacent Jeem's body-first
  path while fitting the same isolated Noto Naskh skeleton.
- Preserve two-way-connector context and script-scoped Arabic provenance, with
  Khaa queued from the same attachment ledger.

### Added — cited Arabic independent-jeem ductus (HL-C09AL)

- Record that the page's linked Thaa asset actually writes another two-dot Taa,
  leaving **ث** on its conventional fallback rather than inventing source data.
- Reprioritize to **ج** and render its body-first motion as three frames: a short
  left-to-right upper head, the continuous descent and rounded bowl, then one
  lifted dot below.
- Keep two-way-connector context, Noto Naskh geometry, and Arabic-scoped
  provenance distinct from Urdu's dot-first Jeem path.

### Added — cited Arabic independent-taa ductus (HL-C09AJ)

- Verify independent **ت** with the page's separately demonstrated Arabic bowl
  followed by left and right upper dots as two individually lifted strokes.
- Keep the evidence split explicit because the Taa clip opens on the completed
  body, while preserving two-way-connector context and Arabic-scoped provenance.
- Render a real three-frame filmstrip whose source, prose, Noto Naskh geometry,
  two-lift summary, and distinct identity agree without inheriting Persian ت.

### Added — cited Arabic independent-baa ductus (HL-C09AI)

- Verify independent **ب** from the University of Oregon's adjacent video as a
  continuous right-to-left bowl followed by one lift and the dot below.
- Preserve the lesson's two-way-connector context and isolate Arabic provenance
  from the Persian record for the same Unicode glyph.
- Render a real two-frame filmstrip whose source, prose, Noto Naskh geometry,
  one-lift summary, and Arabic-scoped identity agree.

### Added — cited Arabic independent-alif ductus (HL-C09AH)

- Verify independent **ا** from the University of Oregon's *Introduction to
  Arabic* video as one continuous top-to-bottom stroke with zero lifts.
- Preserve the lesson's one-way-connector context and isolate Arabic provenance
  from the Persian and Urdu records for the same Unicode glyph.
- Render a real one-frame filmstrip whose source, prose, Noto Naskh geometry,
  zero-lift summary, and Arabic-scoped identity agree.

### Added — cited Urdu independent-baṛī-ye ductus (HL-C09AG)

- Add Urdu independent ے from Northwestern's *Zer o Zabar* animations as one
  continuous folded bowl from the upper right through the leftward sweep and
  far-left curl, then rightward along the lower fold.
- Keep the zero-lift path and all three learner movements on the vendored Noto
  Naskh fallback while preserving the source's positional distinction.
- Render a real three-frame filmstrip whose one stroke, zero lifts, source,
  prose, geometry, and summary agree, completing the Urdu starter inventory.

### Added — cited Urdu independent-nun-ghunna ductus (HL-C09AF)

- Add Urdu independent ں from Northwestern's *Zer o Zabar* animations as the
  same right-to-left, below-baseline bowl as ن, without its dot or a pen lift.
- Preserve the source's ordinary-nūn initial/medial forms and verify that Noto
  Naskh U+06BA exactly shares U+0646's body contour with the dot removed.
- Render a real one-frame filmstrip whose one stroke, zero lifts, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-ye ductus (HL-C09AE)

- Add Urdu independent ی from Northwestern's *Zer o Zabar* animations as one
  dotless S-shaped stroke from the upper right through the below-baseline bowl.
- Preserve the source's initial/medial two-dot distinction while fitting the
  independent zero-lift motion to Noto Naskh.
- Render a real two-frame filmstrip whose one stroke, zero lifts, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-he ductus (HL-C09AD)

- Add Urdu independent ہ from Northwestern's *Zer o Zabar* animations as one
  counterclockwise teardrop loop that closes without a lift.
- Preserve the source's oval-or-teardrop prose and its distinct initial,
  medial, and final forms while fitting the independent motion to Noto Naskh.
- Render a real one-frame filmstrip whose one stroke, zero lifts, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-nun ductus (HL-C09AC)

- Add Urdu independent ن from Northwestern's *Zer o Zabar* animations as one
  right-to-left bowl below the baseline, then one lifted dot.
- Preserve the near-baseline dot guidance and distinct initial/medial tooth
  form while fitting the independent two-stroke motion to Noto Naskh.
- Render a real two-frame filmstrip whose two strokes, one lift, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-mim ductus (HL-C09AB)

- Add Urdu independent م from Northwestern's *Zer o Zabar* animations as one
  round-head-then-tail stroke that descends below the baseline without a lift.
- Preserve the source's counterclockwise handwritten loop and calligraphic
  contrast while fitting their shared head-to-tail motion to Noto Naskh.
- Render a real two-frame filmstrip whose one stroke, zero lifts, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-lam ductus (HL-C09AA)

- Add Urdu independent ل from Northwestern's *Zer o Zabar* animations as one
  top-down stroke through the tall upright and below-baseline leftward bowl.
- Preserve the chapter's connector and final-bowl distinctions while fitting
  the independent zero-lift motion to Noto Naskh.
- Render a real two-frame filmstrip whose one stroke, zero lifts, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-kaf ductus (HL-C09Z)

- Add Urdu independent ک from Northwestern's *Zer o Zabar* animations and prose
  as one main-line body stroke followed by the separately lifted long slash.
- Preserve the source's flatter independent bowl, pronounced final hook, and
  explicit warning not to write kāf in one penstroke while fitting the learner
  path to Noto Naskh.
- Render a real three-frame filmstrip whose two strokes, one lift, source,
  prose, geometry, and summary agree.

### Added — cited Urdu independent-shin ductus (HL-C09Y)

- Add Urdu independent ش from Northwestern's *Zer o Zabar* animations as the
  full two-movement س body followed by lower-left, lower-right, and centered
  upper dot strokes, with three sourced pen lifts.
- Preserve the chapter's two-below/one-above arrangement and optional toothless
  body while fitting the standard toothed form to Noto Naskh.
- Render a real five-frame filmstrip whose four strokes, lift boundaries,
  source, prose, geometry, and summary agree.

### Added — cited Urdu independent-sin ductus (HL-C09X)

- Add Urdu independent س from Northwestern's *Zer o Zabar* animations as three
  close teeth flowing directly into the final bowl in one zero-lift stroke.
- Preserve the source's optional long gentle curve as an especially common
  handwriting alternative while fitting the standard toothed form to Noto Naskh.
- Render a real two-frame filmstrip whose movements stay in the same pen-down
  run, with script-scoped source, prose, geometry, and summary agreement.

### Added — cited Urdu independent-re ductus (HL-C09W)

- Add Urdu independent ر from Northwestern's *Zer o Zabar* animation and prose
  as one downward line that curves left in a zero-lift stroke.
- Preserve the source's separate final-form motion and final-re Naskh/Nastaliq
  distinction while fitting the independent path to Noto Naskh.
- Render a real two-frame filmstrip whose movements stay in the same pen-down
  run, with source, prose, geometry, and summary agreement.

### Added — cited Urdu independent-jim ductus (HL-C09V)

- Add Urdu independent ج from Northwestern's *Zer o Zabar* animation as a
  dot-first two-stroke path with one lift and three learner movements.
- Keep the pointed hooked head, descent, and bowl in one continuous body run;
  record the chapter's flat-head alternative as purely aesthetic.
- Render the canonical Noto Naskh fallback behind a real three-frame filmstrip
  whose body frames retain the completed dot.

### Added — cited Urdu independent-alif ductus (HL-C09U)

- Add Urdu independent ا from Northwestern's *Zer o Zabar* animation as one
  top-to-bottom movement with zero lifts, retaining the lesson's explicit
  contrast with bottom-to-top final ـا.
- Make ductus lookup script-aware and scope the Urdu key so its Northwestern
  source remains independently addressable from Persian ا's UT Austin source.
- Keep source/data/geometry agreement on the canonical Noto Naskh fallback and
  render a real one-frame Urdu filmstrip in Browse and Practice.
- Keep both narrow shared-glyph filmstrips readable with concise movement
  captions while their full script-specific instructions remain visible below.

### Added — cited Persian looping-body ductus (HL-C09T)

- Add Persian ه from UT Austin Persian Online's 02:47–02:50 demonstration as
  one unbroken isolated looping-body movement with zero lifts.
- Fit that single run to the vendored Noto Naskh form's wider two-counter
  outline and leftward baseline finish, with source/data/geometry agreement.
- Render a real one-frame filmstrip using the Persian font route established by
  HL-C09S, completing the nine-letter Persian starter inventory.

### Added — cited Persian loop-and-tail ductus (HL-C09S)

- Correct the source-adjacent queue after confirming that و appears between ن
  and ه in UT Austin Persian Online's full-alphabet lesson.
- Add its 02:43–02:45 demonstration as one unbroken two-movement filmstrip: the
  small head loops, then flows directly into the leftward curving tail.
- Pin zero lifts, Noto Naskh geometry, and source/data/rendering agreement.
- Route each authored path to its owning script font at runtime; Persian paths
  now load Noto Naskh instead of silently retaining the prose fallback after a
  failed Tamil-font glyph lookup.

### Changed — restore handwriting bundle headroom (HL-C09R)

- Split the handwriting model, SVG renderer, and font parser into one named,
  independently cacheable production chunk while keeping the app's synchronous
  startup and relative asset URLs.
- Require that chunk in the bundle gate and apply the unchanged 500,000-byte
  eager limit to it, so a missing split cannot hide inside a passing build.

### Added — cited Persian bowl-and-dot ductus (HL-C09Q)

- Add Persian ن from UT Austin Persian Online's 02:37–02:43 demonstration: one
  continuous right-to-left Naskh bowl, then one lift to place the dot above.
- Pin one lift, two learner movements, Noto Naskh geometry, and a real two-frame
  filmstrip that preserves the completed bowl while the dot is placed.

### Added — cited Persian head-and-tail ductus (HL-C09P)

- Add Persian م from UT Austin Persian Online's 02:33–02:36 demonstration: one
  continuous Naskh stroke shapes the round head and flows directly into the
  descending tail without lifting.
- Pin zero lifts, two learner movements, Noto Naskh geometry, and a real
  two-frame filmstrip that keeps both movements in the same pen-down run.

### Added — cited Persian upright-and-base ductus (HL-C09O)

- Add Persian ل from UT Austin Persian Online's 02:29–02:32 demonstration: one
  continuous Naskh stroke descends the tall upright and turns directly into the
  leftward base curve without lifting.
- Pin zero lifts, two learner movements, Noto Naskh geometry, and a real
  two-frame filmstrip that keeps both movements in the same pen-down run.
- Reuse each Persian letter's canonical source object from the script data
  instead of rebundling a second citation and variation copy.

### Added — cited Persian teeth-and-bowl ductus (HL-C09N)

- Add Persian س from UT Austin Persian Online's 01:29–01:35 demonstration: one
  continuous right-to-left Naskh stroke forms all three teeth and flows into
  the final bowl without lifting.
- Pin zero lifts, two learner movements, Noto Naskh geometry, and a real
  two-frame filmstrip that keeps both movements in the same pen-down run.

### Added — cited Persian bowl-and-two-dots ductus (HL-C09M)

- Add Persian ت from UT Austin Persian Online's 00:22–00:27 demonstration: its
  isolated Naskh bowl sweeps right-to-left, then separate runs place the left
  and right dots above in the source's order.
- Pin two lifts, three movements, Noto Naskh geometry, and a real three-frame
  filmstrip; record the intervening پ row as deferred inventory work.

### Added — cited Persian bowl-and-dot ductus (HL-C09L)

- Add Persian ب from UT Austin Persian Online's adjacent freehand demonstration:
  the isolated Naskh bowl sweeps right-to-left before one lift places the dot.
- Pin the 00:11–00:15 source window, two learner movements, Noto Naskh geometry,
  and real two-frame filmstrip so the bowl remains visible during the dot.

### Added — first cited Persian ductus (HL-C09K)

- Add Persian ا from UT Austin Persian Online's opening freehand demonstration:
  one top-to-bottom movement forms the isolated Naskh stem without a pen lift.
- Resolve each verified glyph against its owning script font, check the complete
  path against the vendored Noto Naskh outline, and pin the source timestamp,
  downward motion, zero lifts, and real one-frame filmstrip.

### Added — first multi-stroke cited ductus (HL-C09A)

- Add Tamil அ from Radhakrishnan's *Tamil Script Learners Manual*, Frame 4:
  four connected movements trace the body, then one verified lift precedes the
  separate right upright.
- Check both strokes against the vendored Noto Sans Tamil outline, keep the
  learner prose and citation identical to the authored path, and exercise the
  real two-stroke filmstrip instead of relying only on a synthetic fixture.

### Added — connected long-vowel loop for Tamil ஆ (HL-C09B)

- Add Frame 4's next row, Tamil ஆ, reusing the source's four-movement அ body,
  then preserving one lift before a second pen-down run that joins the right
  upright directly to the long-vowel loop.
- Check all six movements against the full Noto Sans Tamil outline and pin the
  real filmstrip's one-lift transition so list numbering cannot invent a second.

### Added — cited two-stroke ductus for Tamil இ (HL-C09C)

- Add Frame 4's third row, Tamil இ: five joined movements build its inner curl
  and crossing lower loops, then one verified lift precedes the outer-left climb
  and final arch.
- Check all seven movements against the full Noto Sans Tamil outline and pin the
  real filmstrip's sole lift so its numbered parts cannot be mistaken for seven
  separate strokes.

### Added — cited three-stroke ductus for Tamil க (HL-C09D)

- Add Frame 3's final row, Tamil க: three joined movements form its upper frame,
  two joined movements form the lower-left bowl, and a final movement forms the
  lower-right bowl, with exactly two verified lifts between those runs.
- Check all six movements against the full Noto Sans Tamil outline and pin both
  lift transitions in the real filmstrip and learner prose.

### Added — cited unbroken ductus for Tamil வ (HL-C09E)

- Add Frame 9's first row, Tamil வ: five joined movements carry its spiral body
  into the bottom bar and right upright without a pen lift.
- Check the complete path against the Noto Sans Tamil outline and pin its zero
  lifts in the real five-frame filmstrip and learner prose.

### Added — cited unbroken ductus for Tamil ல (HL-C09F)

- Add Frame 9's second row, Tamil ல: four joined movements carry its outward
  spiral through the middle descent and deep right-hand turn to the open tip
  without a pen lift.
- Check the complete path against the Noto Sans Tamil outline and pin its zero
  lifts in the real four-frame filmstrip and learner prose.

### Added — cited three-stroke ductus for Tamil ற (HL-C09G)

- Add Frame 10's Tamil ற row: two joined movements form the left arch and first
  middle descent, movement 3 restarts on the adjacent descent, and movements
  4–5 join the right arch to the below-baseline sweep and descender.
- Check all five movements against the Noto Sans Tamil outline and pin both
  lift transitions in the real filmstrip and learner prose.

### Added — cited two-stroke ductus for Tamil ன (HL-C09H)

- Add Frame 13's first Tamil nasal row, ன: five joined movements carry its left
  spiral through the single inner arch and top bar, then one verified lift
  precedes the separate right upright.
- Check all six movements against the Noto Sans Tamil outline and pin the sole
  lift transition in the real filmstrip and learner prose.

### Added — cited two-stroke ductus for Tamil ண (HL-C09I)

- Add Frame 13's adjacent Tamil nasal row, ண: six joined movements carry its
  left spiral through both inner arches and the top bar, then one verified lift
  precedes the separate right upright.
- Check all seven movements against the Noto Sans Tamil outline and pin the sole
  lift transition in the real filmstrip and learner prose.

### Added — cited three-stroke ductus for Tamil ந (HL-C09J)

- Add Frame 12's Tamil dental nasal, ந: three joined opening movements, one
  verified lift before the joined middle rise and top bar, and a second before
  the right-hand descent and below-baseline tail.
- Adapt the primer's looped handwritten form to the vendored Noto outline and
  pin all six movements, both lift transitions, and the source variation in
  geometry, prose, and filmstrip tests.

### Added — canonical lesson figures

- Preserve standalone Markdown images as typed lesson blocks instead of flattening
  them into spoken alt text, and render their captions through safe DOM APIs.
- Bundle generated per-track SVGs from the same committed figure files the LaTeX
  books convert to PDF, beginning with Spanish *café*'s etymology route.
- Resolve only track-local `figures/*.svg` destinations and reject missing,
  remote, absolute, or traversing figure paths.

### Added — public Language Ladder

- Carry canonical ordered `patternSlots` from parsed curriculum lessons into the
  app lesson model, so mixed practice can instantiate productive frames from the
  same known-filler contract as the books and validators.
- Publish the validated production build to the repository's existing GitHub
  Pages site at `/coding-adventures/language-ladder/` whenever the app, shared
  data package, or human-language curriculum changes.
- Keep relative asset URLs and deploy only the `language-ladder/` subdirectory,
  so the app works at its project-site path without replacing the published
  books or the repository's other Pages artifacts.
- Render Urdu learning, focused-check, mixed-review, concept, browse, and
  script-practice surfaces with the vendored static Noto Nastaliq Urdu family;
  keep Naskh as the CSS fallback rather than the normal Urdu face.

### Changed — frontier-sized production loading

- Keep the 1,669-lesson Markdown corpus behind lazy imports. Learn initially
  parses only completed lessons and the current frontier of each selected local
  path; Lessons and Concepts opt into the complete corpus when opened.
- Split script data, curriculum plans, and generated-book ledgers into stable
  cacheable chunks. The largest eager production chunk is now 475.10 kB and
  Vite's 500 kB warning is clean, down from one 7,115.81 kB JavaScript file.
- Group lazy lessons into track-local batches capped near 32 kB. Corpus-wide
  modes now fetch 278 batches instead of 1,669 per-lesson modules, while Learn
  still downloads only the small batches containing its active frontiers.
- Enforce the production budgets in BUILD: fewer than 400 lesson requests, no
  lesson batch above 33 kB, and no eager chunk above 500 kB.
- Preserve pure relative asset URLs for local and sub-path publication, and
  show an explicit loading or failure state while a requested corpus tranche
  arrives.

### Fixed — standalone TypeScript validation

- Restore a clean strict typecheck across the application and all tests by
  typing the shared DOM factory, making the untrusted review-log conversion
  explicit, and removing stale test symbols.
- Use the repository-owned Node test types and ESM-safe fixture paths for
  vendored-font tests.
- Run `npm run typecheck` in the standalone BUILD before the production bundle
  and test suite, and keep the deterministic dependency install in its intended
  subshell so those gates run in Language Ladder rather than its data package or
  rewrite that package's lockfile.
- Refresh the transitive Nano ID lock entry to 3.3.18, clearing the high-severity
  development audit finding reported by the standalone install.

### Added — the stroke-order filmstrip (HL-C08)

- Add `src/ductusview.ts`: a pure SVG renderer that composes the authored pen
  path from `strokes.ts` with the real glyph outline from `truetype.ts` into a
  frame-by-frame build-up of how a letter is written. It returns a tree of plain
  objects (`SvgNode`) plus a serialiser, and touches no DOM — the same posture
  every other pure module in this app takes.
- Apply exactly **one** shared `scale(1,-1)`: the glyph outline and the pen path
  are both in font units (y-up) and SVG is y-down, so they are flipped together
  in a single group and cannot end up disagreeing about which way is up. Only
  the captions sit outside it, because flipped text reads backwards.
- Render the finished glyph as a pale background behind each frame, earlier
  strokes in a settled grey, the current stroke in ink up to that frame's
  fraction, and a dot at the pen tip. Captions are word-wrapped into `<tspan>`
  lines so a long instruction cannot run off the panel.
- Show the stroke ORDER's citation and its variation caveat in the UI. The
  path's *shape* is machine-checked against the font by `strokes.test.ts`; its
  *order* can only be vouched for by a source, so the source is visible.
- Wire it into the "Write it — stroke order" section of the Browse detail panel.
  **Only Tamil ம has an authored pen path**, so every other letter keeps the
  existing numbered prose list; the filmstrip is additive and never load-bearing.
  If the font fails to load or the glyph is missing, the prose stays.
- Fetch the Tamil font lazily, once, and only when a letter with authored ductus
  is opened.
- Build the SVG in `main.ts` through `createElementNS`/`setAttribute`/
  `textContent` — no `innerHTML`. The string serialiser escapes every value that
  reaches an attribute or a text node, so a label can only ever become text.
- Reject illegal or `on*` attribute and tag NAMES in both the string serialiser
  and the DOM builder. A name cannot be escaped — there is nowhere to put the
  entity — so it is either a legal XML name or it is dropped, and `onload` is
  refused by prefix rather than by a blocklist. Every name the module emits is
  a literal today, but `SvgNode` is a public type and the serialiser is meant
  to be reused by the book pipeline.
- Add `tests/ductusview.test.ts` (42 tests): path emission straight from
  `penPathD`, the y-flip and a control proving an unflipped box would fail,
  progressive build-up fractions, shared viewBox across a letter's frames,
  graceful `undefined` for letters with no ductus (including inherited
  `Object` properties), a multi-stroke letter rendering without throwing, and
  attribute-escaping against a hostile label. `ductusview.ts` reaches 100%
  statement and line coverage; it is now listed in `vitest.config.ts`.

This closes the gap HL06 §"the glyph monopoly" calls out: the pen-path model was
built, font-validated to sub-2-font-unit join tolerance, and imported by nothing
but its own test.

### Changed — Persian and Urdu Chapter 5 frontiers

- Load eight new prerequisite-safe take-leave steps across Persian and Urdu,
  including each track's local etymology, script, register, and consolidation
  extensions.
- Keep the shared historical phrase locally readable before mixed comparison:
  Persian joined **خداحافظ** and Urdu spaced **خدا حافظ** remain distinct
  focused answers.
- Use authored checks on every new step, raising objective non-lexical coverage
  to 25 of 119 mapped lessons across 18 tracks while retaining independent
  focused-before-mixed progression.

### Changed — Persian and Urdu Chapter 3 frontiers

- Load ten new prerequisite-safe name-exchange steps across Persian and Urdu,
  including each track's inline register, script, grammar, culture, and
  consolidation extensions.
- Use the authored cumulative-practice checks for both tracks, raising objective
  coverage to 21 of 115 mapped non-lexical lessons across 18 tracks while
  retaining independent focused-before-mixed progression.

### Changed — objective Russian naming progression

- Replace temporary confirmation on both mapped Russian non-lexical frontiers
  with typed checks for polite *вы* and the cross-language naming frame.
- Load their six-lesson schema-v2 prerequisite chain without coupling Russian
  progress to any other selected language; objective coverage is now 19 of 113
  mapped non-lexical lessons across 16 tracks, with 94 remaining.

### Changed — cross-language objective focused checks

- Replace temporary confirmation with authored retrieval in one ready lesson
  for each of 15 tracks that has schema-v2 non-lexical activity debt, covering
  script, grammar, etymology, culture, and cumulative practice.
- Expand objective non-lexical coverage from 2 to 17 of 113 mapped lessons;
  96 remain, including 18 legacy lessons that require schema-v2 migration first.

### Changed — typed objective focused activities

- Load compiled `hl-activity` contracts from the shared lesson AST and prefer
  their authored prompt, canonical answer, accepted variants, feedback, and
  response budget over lexical inference or self-confirmation.
- Hide answer-bearing lesson summaries while an authored activity is active,
  show corrective feedback without advancing after a wrong answer, and require
  an explicit continue after correct feedback before completing that language's
  frontier.
- Keep both knowledge and activity metadata out of the readable lesson view;
  Spanish grammatical gender and punctuation-span lessons establish the
  objective non-lexical contract now reused across 16 tracks.

### Changed — independent local frontiers and focused-before-mixed review

- Replace the global concept cursor and unrestricted jump controls with one
  prerequisite-safe next lesson per selected language, driven by the 20
  validated realization maps.
- Persist completed prefixes by stable lesson id independently per language;
  corrupt, unknown, out-of-order, and stale saved ids fail closed at the first
  missing local prerequisite.
- Require a focused check before advancing: compiled activities when authored,
  objective English-meaning retrieval for other lexical lessons, and temporary
  final-recall confirmation for remaining support lessons. Wrong answers do not
  advance.
- Build mixed SRS review only from independently passed shared lessons and wait
  for two visually distinct eligible answers, preventing unseen lessons and
  identical one-option quizzes from entering the pool.
- Introduce script notes at explicit local script-extension nodes, retain
  grounded root links among simultaneously ready paths, and include the new
  Learn-progress key in the two-click reset.

- Load all 20 per-track realization maps at build time and admit Learn/script
  steps through their explicit mapped lesson sets while keeping legacy material
  available in Lessons mode.
- Show mapped micro-lesson and language-specific extension totals for the
  learner's selected mix, including Persian and Urdu's required inline script
  entry nodes.
- Expose the browser-safe independent frontier planner now consumed by Learn's
  per-language progression.
- Hide canonical `hl-knowledge` comments from learner-facing lesson sections;
  the shared parser and source hash still retain their block-boundary meaning.
- Keep the browser's lightweight dataset adapter compatible with the canonical
  typed lesson AST.
- Derive the lesson-card minute label from schema-v2 `duration.max_seconds`,
  while retaining `est_minutes` for unmigrated tracks.
- Independently combine loaded lesson fingerprints and show `book synced` for a
  generated chapter only when the app AST matches the committed book manifest;
  Spanish Chapters 1–6 now verify all 51 migrated lessons this way; Gujarati,
  Marathi, and Punjabi Chapter 6 verify both canonical lessons, while Sanskrit
  Chapter 6 verifies all three and Bengali Chapter 6 verifies its canonical
  lesson, independently of the book generator.

## 0.25.0 — the same syllable in its sister scripts (syllabary, PR 9)

- **Telugu కి, Kannada ಕಿ, Malayalam കി — side by side.** The three Dravidian
  cousins write one sound three ways, and once you can read one the others are a
  short hop. The Browse detail panel now shows, under **"Same sound, sister
  scripts,"** the selected syllable as the *other* syllabaries write it — turning
  "learn Telugu" into "learn the family" by making the connection visible (the
  spiral model's whole premise: the links between languages are the memory hooks).
- **Grounded, nothing invented.** A new pure `crossScriptSiblings` matches the
  syllable's romanization *exactly* across scripts. That is safe because Telugu /
  Kannada / Malayalam are all emitted by the one generator from the same
  ISO-15919 scheme, so "ki" is byte-identical everywhere; every sibling glyph is
  a real letter already in another script's data, pulled out by that match.
- **Restricted to the fully-syllabic trio.** Only scripts where every letter is a
  `syllable` (the `isSyllabary` predicate) contribute siblings, so Tamil /
  Devanagari / Gujarati — abugidas that model a consonant and a vowel-sign
  separately — are never mis-matched, and alphabets get no sibling row at all. A
  Malayalam-only row (the alveolar **ṉa**) correctly shows no siblings.
- **Control test.** Telugu "ki" resolves to the real Kannada ಕಿ + Malayalam കി
  (and never Telugu itself); an alphabet and the Malayalam-only ṉa row yield
  none; and — the control — the helper is read-only: `letters`, `isSyllabary` and
  the matrix are untouched.

## 0.24.0 — the script's numerals (syllabary, PR 8)

- **The syllabaries now carry their own digits.** Reading a language means
  reading its numbers, and Telugu / Kannada / Malayalam write them with distinct
  glyphs, not Western 0-9 (Telugu ౦౧౨౩౪౫౬౭౮౯). Browse now shows a **"Numerals
  (0–9)"** strip for the three Dravidian scripts, each digit tile the glyph over
  its value.
- **Grounded, additive, same pattern as the independent vowels.** The generator
  composes each from `<SCRIPT> DIGIT <ZERO..NINE>` and romanizes it as the digit
  value (the Unicode name fixes it unambiguously — these are decimal digits, no
  guessing). They live in a **separate `digits` field**, not mixed into `letters`,
  so the consonant syllabary and the gate/matrix that key on it being
  all-syllables stay untouched (the generated `letters` and `independentVowels`
  are byte-for-byte unchanged).
- **Control test.** The real Telugu digits are the 10 expected glyphs mapped to
  "0"…"9" (role `digit`, no fabricated ductus), all three scripts carry them, and
  — the control — none leak into `letters`, so `isSyllabary` still holds and the
  matrix is unaffected.

## 0.23.0 — flag the special-consonant rows in the matrix (syllabary, PR 7)

- **The matrix now marks its tricky rows.** In the consonant × vowel grid the
  retroflex **ḷa** and alveolar **ṟa / ṉa** rows — the ones a reader confuses
  with the ordinary *la / ra / na* — now carry a **★** and the same teal tint the
  Browse tiles give them, so in the full grid the confusable rows stand out at a
  glance. Connects the special-consonant flag (PR 4) to the matrix (PR 5).
- **No new judgement.** `buildSyllableMatrix` gains a `special` flag per row,
  computed by reusing the already-tested `specialConsonant` classifier on the
  row's base syllable — so the matrix flags exactly the same rows the tiles do
  (Telugu ḷa / ṟa; Malayalam also ṉa; Telugu has no ṉa). Control-tested: a "ḷa"
  row is flagged, "ka"/"la"/"ra" are not, and the real Telugu grid flags exactly
  the ḷa / ṟa rows. Zero new data.

## 0.22.0 — the independent (word-initial) vowels (syllabary, PR 6)

- **The syllabaries now carry their standalone vowels.** Everything so far was
  consonant syllables (a consonant + a vowel *sign*); but a word that *begins*
  with a vowel writes a different letter — the **independent vowel** (అ *a*,
  ఆ *ā*, ఇ *i* … ఔ *au*, ఋ *r̥*). Without them a vowel-initial word can't be read.
  Browse now shows an **"Independent vowels (word-initial)"** strip above the
  grid for the three Dravidian scripts.
- **Still Unicode-grounded, still additive.** The generator composes each from
  `<SCRIPT> LETTER <V>` (the inherent /a/ is `LETTER A`), romanized in ISO-15919
  from the same vetted vowel table as the signs — never re-typed, so the vocalic
  R is r̥ (r + U+0325), not IAST ṛ. They live in a **separate `independentVowels`
  field**, not mixed into `letters`, so the consonant syllabary — and the
  slow-unlock gate and the matrix that key on it being all-syllables — is
  completely untouched (the generated `letters` are byte-for-byte unchanged).
- **Control test.** Asserts the real Telugu independent vowels are the 13 expected
  glyphs + ISO-15919 romans (role `vowel`, no fabricated ductus, r̥ = r + U+0325),
  all three scripts carry them, and — the control — none leak into `letters`, so
  `isSyllabary` still holds and the matrix still builds its full 35 × 13 grid.

## 0.21.0 — Browse a syllabary as its consonant × vowel matrix (syllabary, PR 5)

- **The syllabaries now offer a grid view.** A Dravidian abugida isn't a flat
  list of ~450 signs; it's a table — every consonant marching across the same
  vowel row (ka kā ki … , kha khā khi … , ga gā gi …). Browse gains a **List /
  Matrix** toggle (syllabaries only; alphabets stay a plain list): Matrix lays
  the syllables out as **rows = consonants, columns = vowels**, so the abugida's
  regularity is the first thing you see. Clicking any cell selects that syllable
  and opens the existing "break it apart" detail panel. No new data — the same
  generated syllables, re-arranged.
- **New pure helper `buildSyllableMatrix(letters)` in `matrix.ts`.** It reuses
  the grounded consonant boundary from `syllabary.ts` (a new row at each bare
  consonant) and reads the column vowels off the first consonant's own row (its
  base syllable's sound minus its inherent vowel gives the consonant prefix;
  stripping that off each syllable yields the vowel it carries — kā → "ā",
  kr̥ → "r̥"). Nothing is invented. If the rows don't all span the same vowels it
  returns **null** rather than risk a syllable sitting under the wrong vowel
  header. Unit-tested with a **control** that a ragged input yields no matrix,
  plus a check against the real Telugu data (a full 35 × 13 grid; the vocalic-R
  column header is ISO-15919 r̥ = r + U+0325, not the IAST dot-below ṛ).

## 0.20.0 — flag the special consonants: retroflex ḷ, alveolar ṟ / ṉ (syllabary, PR 4)

- **The three Dravidian "special" consonants now carry a contrast hint.** To an
  outsider ల vs ళ (*la* vs *ḷa*) is the kind of near-miss that stalls reading, so
  the app now flags the **retroflex ḷ** and the **alveolar ṟ / ṉ** the same way
  it flags Latin false friends: a **★ special consonant** badge on the Browse
  detail, a *"Special letter — tell it apart from 'l/r/n'"* section with a
  grounded note on how it differs, and a tinted grid tile. No new data — these
  letters were already generated (LLA / RRA / NNNA); this only surfaces them.
- **New pure helper `specialConsonant(letter)` in `core.ts`** (mirrors
  `isFalseFriend`): it keys on the syllable's ISO-15919 romanization, which is
  script-agnostic — the leading code point ḷ (U+1E37, dot below) / ṟ (U+1E5F) /
  ṉ (U+1E49, line below) is the retroflex/alveolar marker. Those marks appear
  *only* on these consonants in our data — the vocalic-R vowel uses a different
  code point (ring-below r̥, U+0325) — so the test is exact, not heuristic.
  `LetterView` gains a `special` field. Unit-tested with a **control** that keeps
  the ordinary l / r / n and the vocalic r̥ un-flagged, plus a check that exactly
  the 26 LLA+RRA rows of the real Telugu data are marked (Telugu has no ṉ).

## 0.19.0 — the full vowel row: ai, au & vocalic R (syllabary, PR 3)

- **Each consonant now carries three more syllables.** The generator's core
  vowel row was the ten short/long vowels (a ā i ī u ū e ē o ō); it now also
  composes the two **diphthongs** (ai, au) and the **vocalic R** that
  Sanskrit-derived words carry — కృ = *kr̥*, as in కృష్ణ *kr̥ṣṇa* "Krishna". So a
  consonant's row grows from 10 to 13, and the regenerated data goes **Telugu
  350 → 455, Kannada 350 → 455, Malayalam 360 → 468** syllables.
- **Still Unicode-grounded.** The three new syllables are composed from the
  `VOWEL SIGN AI` / `AU` / `VOCALIC R` code points of each block, verified to
  exist by their official Unicode names before use — nothing hand-typed. The
  vocalic-R romanization is **ISO-15919 `r̥`** (a plain *r* with a combining ring
  below), deliberately *not* IAST's dot-below `ṛ` — in ISO-15919 that dot-below
  form is the *retroflex* ṛ, a different sound, so using it would be wrong.
- **Flows through the slow-unlock gate unchanged.** The new syllables are signed
  (two components), so `consonantGroups` keeps them inside their consonant's
  group automatically: Practice on Telugu now reads *"mastered 0 / 13"* and the
  first row is `ka kā ki kī ku kū ke kē ko kō kai kau kr̥`. No app code changed —
  only the generator and the regenerated JSON (plus the one data-dependent test
  assertion, 10 → 13). Still recognition only (`strokeOrder: []`).

## 0.18.0 — introduce syllables slowly, one consonant at a time (syllabary, PR 2)

- **The Dravidian drill no longer dumps 350 syllables at once.** Practice on
  Telugu / Kannada / Malayalam now opens with a *single consonant's vowel row*
  (ka kā ki kī ku kū ke kē ko kō) and unlocks the next consonant only once the
  current row is mastered — the "ka, ki, ku … kha, khi, khu" build-up the app is
  meant to teach. This is recognition pattern-building, done the slow way.
- **New pure module `src/syllabary.ts`** is the gate: `consonantGroups` segments
  the consonant-major syllabary at each bare consonant (a grounded boundary — a
  base syllable has one component, a signed one has two), `unlockedConsonantCount`
  counts how many rows are open given the SRS state (a Leitner box ≥ 3 marks a
  syllable mastered; a gap holds everything after it locked), and
  `unlockedLetterIndices` returns the currently drillable subset. No DOM, no
  globals; unit-tested with a control that keeps the 2nd consonant locked until
  the 1st row is fully mastered, plus a check against the real generated Telugu
  data (35 consonants, a 10-syllable first row).
- **Practice wiring** (`main.ts`): on a syllabary in single-script scope, the
  scheduler picks the next question *only from unlocked syllables*, distractors
  are drawn *only from unlocked syllables* (a not-yet-introduced consonant never
  appears as a decoy), the mastery read-out is scoped to the unlocked rows
  (`mastered 0 / 10`, not `0 / 350`), and a cue reads **"Learning consonant N of
  M — master this vowel row to unlock the next."** The alphabets and Mixed mode
  are untouched (the gate is null for them).

## 0.17.0 — Telugu, Kannada & Malayalam letters (syllabary, PR 1)

- **The three Dravidian scripts now have letters.** Browse and Practice covered
  only Arabic / Devanagari / Tamil; Telugu, Kannada, and Malayalam — the tail of
  the language chain — had none. They're **abugidas**, so each "letter" is a
  syllable: a base consonant carries an inherent *a*, and a vowel sign turns it
  into ka → ki → ku, kha → khi → khu. All three now appear as Browse tabs (350
  Telugu / 350 Kannada / 360 Malayalam syllables) and drill in Practice, reusing
  the existing letter engine unchanged (a syllable is just a `Letter`).
- **Generated from Unicode, not hand-typed.** New
  `data/scripts/generate_syllabary.py` composes every syllable from Unicode code
  points, taking each base consonant / vowel-sign's identity and ISO-15919
  romanization from its official Unicode character name (`TELUGU LETTER KA`, …) —
  a letter it can't name from the standard is skipped, never guessed. The three
  `*.json` files are its regenerable output; the generator is the provenance.
- **Recognition only — no fabricated stroke order.** These carry `strokeOrder:
  []` (their ductus is a separate, source-gated effort, still paused). The Browse
  detail now **hides the "Write it — stroke order" section when there is none**,
  rather than showing an empty one — so we never imply data we don't have. The
  grounded consonant⊕vowel-sign decomposition (`క ka + ి "i" sign`) still shows.
- 10 new tests (238 total) grounding the glyphs to code points, with controls
  that bite: `ka` must equal the block's KA code point, `ki` must be KA + the
  i-sign, and every syllable's `strokeOrder` must be `[]`. Verified in a real
  browser — Telugu and Malayalam grids render real glyphs, no tofu. Slowly
  unlocking the syllables one consonant at a time is the next slice (PR 2).

## 0.16.0 — a spine progress bar (HL03 polish)

- **A slim progress bar under the "Concept N of 186" line**, showing how far
  along the whole spine the walk has reached — a sense of the journey's scale
  that the bare count doesn't convey at a glance (a thin sliver at concept 1, a
  half-full bar at ~93, full at the end).
- New pure `spineProgress(cursor, length)` in `sequence.ts` returns the fraction
  reached in `[0, 1]`, counting the current concept (cursor 0 of 10 → 0.1),
  clamping an out-of-range cursor and returning 0 for an empty spine. 3 new tests
  (228 total) with a control that bites: a naive `cursor/length` would read 0 at
  the start, so the test pins 0.1. Width is set via `style.width` (no innerHTML).
  Verified in a real browser (seeded to concept 93 → the bar sits at ~50%).

## 0.15.0 — jump to any concept (HL03 polish)

- **A "jump to concept" picker in the Learn nav.** Walking 186 concepts one
  Next-click at a time is a long way to get anywhere; the nav row is now
  `← Previous | [jump ▾] | Next →`, where the picker is a native `<select>` of
  the whole book-ordered spine (`1. courtesy · thanks`, `2. farewell`, …) with
  free keyboard type-ahead. Selecting one jumps the cursor straight there.
- All three controls now funnel through one `jumpToConcept(index)` — it clamps,
  resets the review draw (the covered set changed), persists the cursor (so the
  jump is where you resume next visit, via the existing `cursorstore`), and
  re-renders. No new persistence surface; it reuses the tested cursor save/clamp.
- **A slice was abandoned, honestly:** the planned "romanization under the review
  options" turned out un-grounded — only ~54 of ~700 lessons populate a
  `romanization` field, and the Indic vocabulary (where script-shape guessing is
  the real gap) carries its romanization inside the *gloss* text instead. Showing
  it would render inconsistent subtext for <8% of options, so it was dropped
  rather than faked; this jump-picker was built instead. 225 tests still pass;
  verified in a real browser.

## 0.14.0 — start over: a "Reset progress" control (HL03 polish)

- **You can now clear your progress.** The app persists a lot — the review
  quiz's SRS state and answer log, the teaching cursor, and the lesson schedule —
  but had no way to wipe it (handing the tab to someone else, or re-walking from
  the top). A quiet **"Reset progress"** control sits at the foot of the Learn
  view; it's a **two-click confirm** (first click arms *"Clear all progress…?"*
  with Yes / Cancel, second executes) so a stray tap can't erase everything.
- New pure `src/reset.ts`: `OWNED_STORAGE_KEYS` sources the three keys from the
  modules that own them (so the list can't drift from what's actually written),
  and `clearProgress(storage)` removes exactly those — **only keys this app
  owns**, guarded per-key so one locked key can't turn "start over" into a crash.
  Executing also resets the in-memory session to concept 0 with an empty review.
- 6 new tests (225 total) with a control that bites: a `clearProgress` that
  missed any owned key leaves it behind and fails; unowned keys are left
  untouched; a throwing `removeItem` still clears the rest. Verified in a real
  browser — both the link and the armed *"Yes, reset / Cancel"* state render.

## 0.13.0 — the Learn walk resumes where you left off (HL03 phase 7b)

- **The teaching cursor now persists.** Review progress and the lesson schedule
  already survived reloads; the Learn cursor didn't — walk to concept 40, close
  the tab, and you were dumped back at "thanks". New `src/cursorstore.ts` saves
  the concept index to `localStorage` on every Prev/Next and restores it at
  startup, so the app resumes exactly where you were. The restored index is
  **clamped to the current spine** (the curriculum grows and shrinks), and a
  corrupt / wrong-version / out-of-range blob falls back to concept 0 rather than
  throwing or pointing off the end.
- 11 new tests (219 total) with controls that bite: strip the version gate and a
  stale blob resurfaces; drop the `getItem` guard and a throwing storage breaks
  startup; a saved index past a now-shorter spine clamps to the last concept.
  Verified in a real browser — seeding the cursor and reloading opens on
  "Concept 5 · Greeting · Hello", not concept 1.
- **Slice 7b (grammar introduction) was reframed:** the curriculum's grammar
  signal is a single concept tag (`GRAMMAR-THE`, articles) with no dedicated
  explanation field — too thin to ground an honest "new grammar" note the way
  scripts have `signature` data. Rather than fabricate one, this slice does the
  more valuable, fully-grounded resume-cursor work. Grammar introduction can
  return if the curriculum grows richer grammar metadata.

## 0.12.0 — introduces a script the first time you meet it (HL03 phase 7a)

- **New writing systems are now introduced as-needed in the Learn sweep.** When
  the walk first reaches a non-Latin script — Arabic, then Devanagari (Hindi),
  then Tamil — that step's card carries a compact **"New script"** note: the
  script's name, its system (abjad / abugida), and *how to recognise it*, pulled
  straight from the script data's `signature` field (e.g. Devanagari's "a
  horizontal head-line runs across the top; letters hang beneath it like laundry
  on a line"). It appears **once**, at the earliest concept in book order that
  teaches the script, and never again.
- New pure `src/scriptintro.ts`: `LANGUAGE_SCRIPT`/`scriptOf` map each chain
  language to its writing system, `firstIntroductionByScript` computes the intro
  concept per script from the spine + lessons, and `scriptIntroFor` returns the
  note for a step or null. **Grounded, never invented:** a script with no JSON
  data (Kannada / Telugu / Malayalam today) gets no note — the mapping still
  knows the language's script, but the note is gated on having real data.
- 13 new tests (208 total) with controls that bite: the *second* appearance of a
  script must not re-introduce it; a script absent from the available-data set
  yields no note. Verified in a real browser — concept #1 (COURTESY-THANKS)
  shows the note on its Arabic / Devanagari / Tamil stops and nothing on the
  data-less Dravidian stops. Grammar-intro is the next slice (7b).

## 0.11.0 — the review quiz remembers you (HL03 phase 6, persistence)

- **The Learn-mode review quiz now persists between visits.** New
  `src/reviewstore.ts` saves the review `Progress` (per-cell Leitner state + the
  answer log) and the SRS session clock to `localStorage` after every answer,
  and restores them at startup — so promotions, demotions, and logged confusions
  survive a reload. Mirrors `progress.ts` (which does this for the lesson
  schedule): the engine stays pure, all (de)serialization is pure and
  unit-tested, and the untrusted blob is validated field-by-field — a corrupt,
  wrong-version, or wrong-shaped payload restores as **empty rather than
  throwing** (a study app that won't start over one bad key is worse than lost
  progress). States are stored as `[cellKey, QuizState]` entry pairs, sidestepping
  the `__proto__`/key-escaping hazards of an object map.
- 9 new tests (194 total), including controls that bite: strip the version gate
  and a stale blob surfaces; drop the `getItem` guard and a throwing storage
  breaks startup. Verified in a real browser — seeding a saved review and
  reloading restores "1 answered" (fresh is "0").
- **Slice 6d (retire the standalone artifacts) was a no-op:** the HL03 spec's
  "script field-guide, spot-the-script quiz, letter-reading trainer" were only
  ever ephemeral exploratory Artifacts, never committed to the repo (no matching
  files or history), and their capabilities already live in Browse / Practice /
  Concepts. Nothing to remove — so this slice does the persistence work instead.

## 0.10.0 — renamed to **language-ladder** (HL03 phase 6, slice 6c)

- **The app is renamed `script-writing-visualizer` → `language-ladder`.** The
  name no longer described it: what began as the HL02 "break a script apart and
  write it" MVP has become the HL03 unified curriculum app, with Learn (the
  teaching sweep + review quiz) as its default and the old script/lesson/concept
  modes folded in as facets. `language-ladder` names what it now is — the
  language chain, climbed rung by rung.
- Directory `git mv`d (history preserved); `package.json`/`package-lock.json`
  name, `index.html` title, the in-app `<h1>`, and the `BUILD` header updated;
  cross-references in the HL03 spec and the Arabic/Russian curriculum docs
  repointed. No source logic changed — the engine and all five modes are byte-
  for-byte the same; 185 tests still pass and the app builds and renders
  unchanged (verified in a real browser). Earlier changelog entries keep the old
  name: they are the accurate record of what happened under it.

## 0.9.6 — the Learn session, review quiz (HL03 phase 6, slice 6b-2)

- **The review pass, wired into Learn mode** — the second of the app's two
  mechanisms. Below the teaching sweep, a randomised cumulative quiz draws over
  everything covered so far (`plan.reviewGrid`, the concept×language grid up to
  the cursor), SRS-weighted by the engine's `pickNext` so missed/overdue items
  resurface and mastered ones fade.
- A cell is asked as **"‹meaning› — in ‹language›?"** and the options are the
  **same concept in other languages** (plus the answer) — the cross-language
  look-alikes the interleaving exists to expose (Telugu ధన్యవాద vs Hindi
  धन्यवाद, both from `dhanya`). If a concept lives in only one language, the
  remaining option slots are filled from elsewhere in the grid so there is
  always a real choice.
- Answering threads through the tested engine: `applyAnswer` **promotes** a hit
  (comes back later) or **demotes** a miss (resurfaces soon) and logs which wrong
  word was picked; the SRS clock advances. A **"what you keep confusing"** panel
  rolls those up from `confusions(log)`, showing the actual words (e.g. "Picked
  ధన్యవాద (telugu) for धन्यवाद (hindi)").
- Moving the concept cursor redraws the review from the new covered set. Progress
  lives in a module-level `let` for now (persistence is a later slice). DOM-only
  shell over the tested engine; 185 tests still pass. Verified in a real browser:
  the quiz renders below the sweep with four real-script options, no tofu.

## 0.9.5 — the Learn session, teaching view (HL03 phase 6, slice 6b-1)

- **New "Learn" mode, now the default** — the curriculum walked the way the book
  does: one concept at a time, forward along the language chain. It renders the
  engine's *teaching pass* (`planSession(...).teaching`) as a numbered sweep of
  cards, one per active language that teaches the concept, in chain order.
- Each card shows the word in its own script, its gloss/romanization, its
  etymology hook, and — the point of the whole app — the **connections back** to
  earlier languages that share a root (e.g. Telugu *ధన్యవాదములు* → Hindi and
  Kannada via `dhanya, vada`). Connections are grounded and backward-only; the
  first stop, where the concept enters, wears an "introduced here" badge.
- Prev / Next walk the **concept spine** (`sweepableConcepts`) — the concepts in
  book order (earliest chapter first). Consolidation lessons (`practice`,
  `practice-mix`, `review` — placeholder headwords, no roots, `reviews_of`
  links) are filtered OUT of the spine: that kind of revisiting is what the
  review quiz is for, so the learner walks real words, not "(practice)". 205 →
  186 concepts.
- DOM-only shell; all sequencing stays in the tested engine. Verified in a real
  browser: every script renders (no tofu), the ten-language THANKS sweep shows
  its `dhanya`/`nal` threads. The review quiz (`pickNext`/`applyAnswer`) is the
  next slice (6b-2).

## 0.9.4 — the session view-model (HL03 phase 6, slice 6a)

- `src/sessionplan.ts` — the seam that assembles the four engine modules into
  one session, with no DOM (the UI slice renders what this returns):
  `planSession(current, covered, lessons, activeCount)` returns the **teaching
  pass** (the current concept swept across the active chain, with connections)
  and the **review pass** (the covered grid the quiz draws from).
- `applyAnswer(progress, cell, correct, session, chosenKey?)` threads the state
  that makes review adaptive: a hit **promotes** the cell (comes back later), a
  miss **demotes** it (box 0, due now) and logs the confusion — so the next
  `pickNext` leans on what was just missed. Immutable; `initProgress` seeds it.
- Controls bite (fault-injected): a review pass that only covered the current
  concept fails the "spans every covered concept" test; a no-op `applyAnswer`
  fails the "missed cell outweighs a mastered one" test. Verified against the
  real curriculum (COURTESY-THANKS teaches across all ten, reviews alongside
  GREETING-HELLO). Pure, deterministic. Next: slice 6b renders it.

## 0.9.3 — the mistakes store (HL03 phase 5)

- `src/mistakes.ts` — records each quiz answer and, crucially, WHAT THE LEARNER
  CHOSE when wrong (the confusion — e.g. picking the French cognate's meaning
  for the Spanish word). `recordAnswer` appends immutably; `demote` feeds a miss
  back into the SRS (box→0, due now, lapse++) so the item resurfaces sooner in
  `pickNext`; `confusions` rolls the wrong answers into ranked "what you keep
  mixing up" pairs.
- Grounded: a confusion only ever appears if the learner actually made it — no
  pair is inferred. Pair keys use `JSON.stringify`, not delimiter-joining, so an
  id containing a comma can't collapse two distinct confusions into one.
- Controls bite (fault-injected): a no-op demote fails the "missed cell
  resurfaces" test (its draw weight must jump above a mastered cell's); a
  fabricated pair fails the "nothing invented" control. Pure, deterministic, no
  I/O — the caller passes the session index in.
- This completes the pure-logic layers of HL03 (phases 2–5). Next: phase 6, the
  UI that unifies the four modes into one curriculum-driven session.

## 0.9.2 — the SRS-weighted draw (HL03 phase 4, part 2 — quiz complete)

- Extends `src/quiz.ts` with the randomised cumulative quiz's draw:
  `pickNext(grid, states, session, rng)` selects a cell from the covered grid
  weighted by `cellWeight` — never-seen cells rank high, DUE cells rank higher
  the more overdue / lower-box / lapsed they are (the missed material review
  exists for), and not-yet-due cells sink to a floor so review stays
  interleaved. Per-cell Leitner state (`QuizState`, keyed by `cellKey`) reuses
  scheduler.ts's box/interval math. Deterministic via a seeded LCG (`makeRng`);
  the app never depends on `Math.random`.
- Two controls, both verified by injection: over many draws the sample spans
  MULTIPLE concepts AND languages (a collapsed draw fails); and the draw biases
  toward a missed/overdue cell over a mastered one by a wide margin (injecting
  uniform weighting fails it). This is the primary review mechanism from HL03 —
  "what is 5 in Telugu? 12 in Latin?" — now complete and pure.

## 0.9.1 — the covered grid (HL03 phase 4, part 1)

- `src/quiz.ts` — `coveredGrid(covered, lessons, activeCount)` enumerates every
  (concept × language) cell the learner has studied, each tied to the real
  lesson that answers it. This is the pool the randomised cumulative quiz will
  draw from ("what is 5 in Telugu? 12 in Latin?").
- Built by **reusing the teaching sweep**, not re-deriving the concept→language
  join: a cell exists exactly where a covered concept's sweep has a stop in an
  active language — so the review side can only ever ask about a (concept,
  language) the teaching side actually presents. Deterministic (concepts sorted,
  then chain order, then chapter/id). Plus `conceptsIn` / `languagesIn` and a
  stable `cellKey` for the SRS to track state per item.
- Verified against the real curriculum: COURTESY-THANKS covers all ten chain
  languages; two covered concepts interleave across both concepts and many
  languages. Controls bite — mislabelling a cell fails the grounding test, and
  collapsing the grid to one concept fails the interleave control. Pure, no UI.
- Next (part 2): the SRS-weighted `pickNext` draw over this grid.

## 0.9.0 — the session orchestrator: sweep + grounded connections (HL03 phase 3)

- `src/session.ts` — `buildSession(concept, lessons, activeCount)` takes the
  teaching sweep (phase 2) and annotates each stop with the **connections back
  to earlier languages in the sweep**: where two languages' lessons for the
  concept share an etymological root, the link is surfaced. This is the payoff
  of interleaving — meeting "thank you" in Telugu right after Kannada and Hindi,
  and being shown all three carry the Sanskrit root *dhanya*.
- **The grounding rule, enforced in code**: a connection exists *iff* the two
  stops' lessons literally share a root string (from `lesson.roots`). Nothing is
  inferred or invented; the reported `sharedRoots` is the exact set intersection,
  sorted. Connections always point backward in chain order.
- Verified against the real curriculum: Kannada and Telugu "thank you" link back
  to Hindi via `dhanya`. Controls bite — asserting a connection without a shared
  root fails the grounding test; over-reporting (union instead of intersection)
  fails the "never from thin air" control; a concept sharing no roots surfaces
  no link. Pure, deterministic, no UI.

## 0.8.1 — carry etymological roots on each lesson (HL03 phase 3 prerequisite)

- `Lesson` now carries `roots: string[]` — the etymological roots a lesson cites
  (e.g. `["bonus", "dies"]`). `toLesson` maps them from the frontmatter the same
  way it maps `prerequisites`; the human-language-data parser already extracted
  them, they just were not threaded into the app's `Lesson`.
- This is the **join key for cross-language connections** (the next phase): two
  lessons in different languages that share a root are etymologically linked.
- Tests: roots parse through (a lesson citing none gets `[]`), and — against the
  real curriculum — the Sanskrit root `dhanya` is carried by lessons in more
  than one chain language (Hindi/Kannada/Telugu). Both fail if roots aren't
  plumbed, confirmed by injection.

## 0.8.0 — the language chain and the teaching sweep (HL03 phase 2)

- First implementation piece of the unified language-learning app
  ([HL03](../../../specs/HL03-unified-language-learning-app.md)): a pure
  `sequence.ts` module encoding the fixed **language chain**
  (Spanish → Latin → French → German → Arabic → Hindi → Tamil → Kannada →
  Telugu → Malayalam) and the **teaching sweep** — for one concept, the active
  languages that teach it, walked in chain order.
- `teachingSweep(concept, lessons, active)` filters to the concept, restricts to
  the active chain prefix, skips languages that do not teach it, and orders the
  result by the chain (never by input order). `sweepableConcepts` lists concepts
  in book order (earliest chapter first). No UI — sequencing logic only.
- Verified against the real curriculum: `GREETING-HELLO` sweeps all ten
  languages in exact chain order. Every honesty check is paired with a control
  that fails on broken input — and writing this caught a redundant active-filter
  that would have made the "only active languages" test vacuous; removed so the
  test can actually fail.

## 0.7.1 — each script's at-a-glance identification signature

- Every script now carries a **`signature`** — the one visual feature that gives
  it away at a glance (Devanagari's head-line; Gujarati being Devanagari with
  that line erased; Arabic's joined right-to-left ribbon vs Hebrew's blocky
  separate letters; Cyrillic's Я/Ж/Д tells; Chinese's dense square blocks).
- Added to all seven script data files (`data/scripts/*.json`) and to the
  `ScriptData` type; a test asserts every script ships a non-empty signature.
- Each signature was written **against the rendered font**, not from memory —
  the same verify-by-looking discipline the stroke data uses. This is the data
  backbone for a future "spot the script" identification mode.

## 0.7.0 — read the letter's TRUE shape out of the font

- **`src/truetype.ts`** — a zero-dependency TrueType reader: table directory,
  `cmap` (formats 4 and 12), `loca`, `glyf`, simple and composite glyphs, the
  delta-encoded coordinate flags, and the on-curve midpoint TrueType implies
  between consecutive off-curve points. Outlines come back in font units
  (y-up, baseline 0); the renderer applies one `scale(1,-1)`.
- **Why not hand-drawn SVG paths.** A subtly wrong ண looks fine to anyone who
  cannot already read Tamil — the entire audience — so the error would ship as
  the lesson. Extracting from the vendored font makes shapes correct by
  construction and keeps them identical to what the app renders text with.
- **Hostile input is bounded.** Every count and offset in a font file is
  attacker-controlled if this is ever pointed at an untrusted font, and it runs
  in the browser. `cmap` ranges clamp to U+10FFFF; a single decrementing budget
  bounds total mapping ITERATIONS across both cmap readers (capping the map's
  size alone is not enough — re-mapping groups and format 4's BMP-bounded keys
  both cost work without growing it); a component budget bounds composite
  FAN-OUT, which the depth cap does not (N components at each of 6 levels is
  N⁶ visits — minutes of frozen main thread from a 632-byte file);
  non-ascending contour end points and scaled components are refused rather
  than drawn wrong.
- **Tests rasterise the font** — flatten the quadratics, scan-convert with the
  non-zero winding rule — so shape assertions are checked against what the
  glyph actually looks like. **The raster window is derived from the glyphs'
  own bounding boxes and the rasteriser throws if a glyph would be clipped.**
  A second guard checks the metric's INPUT: the final-stroke measure reports
  its sample count, and the assertion requires samples before believing the
  answer. Without it the measure anchored on the top bar — which overhangs the
  final vertical — collected nothing, and `Math.max([]) - Math.min([])` is
  `-Infinity`, which satisfies any upper bound. It measured nothing and
  reported agreement.
  The window guard exists because a hard-coded window (x ≤ 1030, against ண's true
  extent of 1631) silently amputated 37% of the letter and produced a
  confident, wrong description of its final stroke. A clipped raster does not
  look like an error; it looks like a letter.

## 0.6.1 — Tamil and Gujarati join the script list

- **Tamil** is new: `data/scripts/tamil.json` ships with the first handwriting
  lessons for **any Dravidian language** (`TA-W01`–`W04`). 11 letters and 4
  marks, `complete: false`.
- **Gujarati was already there and simply never wired in.** `gujarati.json` has
  existed since the Gujarati track was authored, but `SCRIPTS` in `src/data.ts`
  listed five scripts while `data/scripts/` held six. Both are now included, so
  Browse and Practice cover **seven** scripts.
- No logic changed — two imports and two array entries. `tests/core.test.ts` uses
  `arrayContaining`, so the script list is not pinned by count.

## 0.6.0 — Concepts mode: one idea, every language that has it

The app could drill letters, and drill lessons. It could not yet do the thing
the curriculum's shared `concept_tag`s exist for: **compare**.

- **New "Concepts" mode.** Canonical concept tags are deliberately identical
  across tracks, which makes them a join key — *gracias / merci / danke /
  धन्यवाद / നന്ദി* are one concept realized eighteen ways. The mode lists every
  concept **two or more languages** share (**39** of them, from **701** lessons)
  and expands each into a side-by-side table.
- **It calls the package's own `languagesForConcept`.** That function has
  shipped since HL01, tested and documented as "what the companion app calls,"
  with **no caller**. This is the caller — and `buildDataset` beside it, so the
  join is the package's tested logic rather than a second implementation that
  could drift from it.
- Each row carries **headword**, **romanization** (only when it differs from the
  headword — for Latin-script tracks the package sets them equal, and repeating
  it is noise), and gloss. The **etymology hooks** follow the comparison, which
  is where they earn their keep: *gracias* ← *gratia* "favour", *merci* ←
  *mercēs* "wages, price", *danke* ← *denken* "to think", *спасибо* ← *спаси
  Бог* "God save you". One courtesy, four unrelated ideas.
- **Concepts only one language realizes are dropped.** Not a special case for
  namespaced tags — a language floor removes them naturally, because a card with
  nothing to compare against isn't a card.

### Prerequisite gating (the other half)

`scheduler.ts` is generic over a numeric index and has no idea that "the
preterite of *comer*" presupposes *comer*. New `concepts.ts` supplies that
knowledge before the scheduler ever sees the pool:

- A lesson unlocks when every id in its `prerequisites` has been **seen**.
- **Unknown prerequisite ids count as satisfied.** A curriculum typo, or a
  prerequisite pointing at an unwritten lesson, should degrade to "shown
  slightly early" — never to "silently unreachable forever," which is the
  failure nobody notices.
- **The gate fails open.** `unlockedOrAll` falls back to every lesson if the
  gated pool is empty (a prerequisite cycle would do it), because practice
  stalling completely is worse than practising something early. Tested with an
  actual cycle.
- "Seen" is computed from **review history** (`reps`/`lapses`/`box`), never from
  `dueAtSession` — the 0.5.0 bug that reported the whole curriculum as started
  after one reload.
- `reviewTargets` maps `reviews_of` onto scheduler indices and is tested, but
  **nothing calls it yet** — it is groundwork for having the app follow the
  syllabus's own "answering this should refresh those" instead of waiting on a
  Leitner interval. Said plainly because an earlier draft of this entry claimed
  the app already did that, which was false. (`ConceptCard.namespaced` is
  likewise computed and not yet rendered.)

### The bug this shipped with, and how it was caught

The first cut gated the **pick**: choose from the rotation, and if the chosen
lesson is locked, substitute the first unlocked one. That is wrong in a way that
is invisible by inspection — the same pick is rejected every turn, so the
substitute is served over and over. A review simulation of the real curriculum
measured it serving **one Arabic lesson 34 times in 40**, wiping out both the
0.5.0 rotating cursor and cross-language interleaving.

The fix gates the **pool**: `nextDue` now takes an `accept` predicate and skips
locked indices *during* the scan, so the cursor keeps advancing; the
nothing-due fallback runs `pickNext` over the unlocked states rather than
grabbing `open[0]`.

The regression test took three attempts to make honest, which is worth
recording. Versions 1 and 2 **passed against the broken implementation** — the
fixture's chain order happened to match its pool order, so the rotation landed
on unlocked lessons anyway. Only a fixture whose dependency order runs
*counter* to pool order (as the real curriculum's does) reproduces it. The test
now fails on the broken version and passes on the fixed one, and both were
verified by actually injecting the old code.

### Notes

- Tests: **97**, up from 75 — `tests/concepts.test.ts`, including checks against
  the **real curriculum** (every card genuinely spans ≥2 languages; gating opens
  a non-empty but non-total pool on a fresh profile; everything is reachable
  once everything is seen).
- `Lesson` gained `romanization`, `script` and `etymologyHook`, which the cards
  need. `tests/lessons.test.ts` now builds fixtures through a defaulting helper
  so adding a field doesn't touch every test.
- **Verified in a browser**, not just built: the 0.5.0 `process is not defined`
  bug was a successful build that died on load, and only a real page load
  catches that class of error. Both new deep imports (`parse.ts`, `queries.ts`)
  are pure modules; console is clean.
- **Known cost, unchanged:** the eager `import.meta.glob` still inlines every
  lesson, so the bundle is ~1.72 MB (542 kB gzipped). Concepts mode makes the
  parsed corpus more valuable, not less, but a lazy glob or a build-time index
  remains the right fix. Deferred, not hidden.

## 0.5.0 — Lessons mode: the whole curriculum, and a memory that survives reloads

The app could already schedule you. It could not **remember** you, and it had
never read a single lesson. Both are fixed.

- **New "Lessons" mode** drills the **written curriculum** — all **679 lessons
  across 18 languages** — instead of only script letters. It reads them from
  `@coding-adventures/human-language-data`, the package that has always shipped
  `frontmatter.ts` / `loader.ts` / `parse.ts` / `queries.ts` for exactly this
  purpose and had **zero consumers** until now.
- **Progress now persists** (`src/progress.ts`). Previously there was no storage
  layer at all, so every Leitner box reset on reload and nothing was ever really
  "tracked". State is saved to `localStorage` keyed by **lesson id**, never by
  array index — indices shift every time a lesson is added, and saving by
  position would silently reattribute your progress to the wrong lesson. Adding
  a lesson now simply means one more unseen item.
- **Cross-language interleaving comes free.** `interleave.buildPool` already
  round-robins across groups for scripts; grouping lessons by language and
  feeding it the same way yields Arabic → Bengali → French → German → Gujarati →
  Hindi in consecutive reviews. A **rotating cursor** walks that order, which
  also fixes the obvious failure mode: box 0/1 fall due again after one session,
  so a scan-from-the-front picker would hand you the lesson you just answered,
  forever.
- **`scheduler.ts`, `interleave.ts` and `drill.ts` are unchanged.** They are
  generic over a numeric index and never needed to know what an item is — which
  is precisely why lessons could reuse them. The new *logic* is pure and tested
  (`lessons.ts`, `progress.ts`, including the `nextDue` cursor scan); the only
  impure edges are a tiny `StorageLike` port and `main.ts`'s DOM shell, which
  remains untested as before.
- **Defensive loading.** Saved state is untrusted input (hand-edited,
  half-written by another tab, left over from an older build). Every field is
  validated, unknown ids are treated as fresh, `__proto__`/`constructor` keys are
  skipped, the item map is `Object.create(null)`, and a throwing or absent
  `localStorage` (Safari private mode, quota) degrades instead of crashing.
- Tests: **67**, up from 57 — `tests/lessons.test.ts` and `tests/progress.test.ts`.

### Known cost

`import.meta.glob(…, { eager: true })` inlines the full text of all 679 lesson
files, so the bundle is ~1.56 MB (480 kB gzipped) and every startup parses them
all — even in Browse mode, which doesn't need them. Only the frontmatter
survives parsing; the bodies are discarded. A lazy glob or a build-time JSON
index would fix both; deferred rather than hidden.

### Three bugs worth recording

- **`process is not defined` at startup.** Importing the package's barrel
  (`index.ts`) pulled `cli.ts` and `loader.ts` — `process`, `node:fs` — into the
  browser bundle. The build *succeeded* and the app then died on load with a
  blank page. Fixed by deep-importing the pure module
  (`.../src/parse.ts`). Caught only by opening the app in a browser; no test or
  build step would have found it.
- **`vitest.config.ts` does not inherit `vite.config.ts`'s `server.fs.allow`.**
  The lesson glob reaches outside the package root, so tests failed with "Denied
  ID" until the same allowance was declared in both configs.
- **The "don't persist untouched items" guard failed open after one reload.**
  It tested `dueAtSession <= 0`, but fresh items are seeded with the *current*
  session, so from session 1 onward every unseen lesson looked touched: the
  payload grew from ~100 bytes to ~48 kB and the "started" count reported the
  whole curriculum. Now keyed on review history (`reps`/`lapses`/`box`) only,
  with a reload round-trip test that would have caught it — the original test
  only covered session 0, the one case where the bug was invisible.

## 0.4.0 — Cross-script interleaving ("Mixed") practice

- **New "Mixed (all scripts)" practice scope** alongside "This script": Practice
  can now **interleave letters from every script in one session** — HL02's
  interleaving principle ("mixing forces discrimination and transfers better").
  The scheduler picks the next due item across the whole combined pool, so a
  Cyrillic prompt is followed by a Hebrew one, then Devanagari, and so on;
  distractors always come from the **target's own script**. Mastery reads across
  the full pool (e.g. "mastered N / 128").
- **New pure module `src/interleave.ts`** — `buildPool(counts)` lays every letter
  of every script into one **round-robin-interleaved** pool (letter 0 of each
  script, then letter 1 of each, …) so mixing starts on the first pass; the
  generic scheduler drives it unchanged. **6 new unit tests (42 total)** incl. an
  integration proving the scheduler alternates scripts and resurfaces a missed
  letter amid the others.
- UI: a scope toggle in Practice; the per-script tabs hide during a mixed
  session; the prompt shows a small script tag. Still zero runtime deps.

## 0.3.0 — Spaced-repetition scheduler wired into Practice

- **New pure module `src/scheduler.ts`** — a Leitner / SM-2-lite scheduler
  measured in **sessions** (no wall-clock, no `Date`), the "core module" of HL02.
  Each item tracks a Leitner box, `dueAtSession`, lapses, and reps; a correct
  answer promotes the box and expands the interval (1 → 3 → 7 → 15 → 30 sessions),
  a wrong answer drops it to box 0 (due again immediately). `pickNext` returns the
  most-overdue item deterministically (ties → fewest reps → lowest index), falling
  back to the soonest-due so practice never stalls.
- **Practice mode now uses it**: instead of a random letter each question, the
  **scheduler chooses** which letter to ask, so missed letters resurface soon and
  mastered ones fade back — real spaced repetition. Each answer advances the
  session clock and feeds `review`. A **"mastered N / total"** read-out joins the
  score line. (Randomness stays only in distractor choice + answer position.)
- **14 new unit tests (36 total)** covering promotion/expansion, lapse-reset,
  `pickNext` ordering + tie-breaks + the never-stall fallback, immutability, and
  an integration check (a correct streak masters an item; a miss resurfaces).

## 0.2.0 — Practice mode (recall drill)

- **New "Practice" mode** alongside "Browse": a recall drill that shows a
  letter's **sound** and asks the learner to pick the matching **glyph** from
  four options, then reveals right/wrong, shows the answer's decomposition, and
  tracks a running **score** (correct / total / %). Recognition builds reading;
  recall (sound → glyph) is the harder second half.
- **Confusable distractors**: wrong answers are drawn from the same script and
  ranked by confusability (same role / same false-friend status rank higher), so
  the choices are meaningfully hard rather than random noise.
- **New pure module `src/drill.ts`** — `buildDrillQuestion`, `confusabilityOrder`,
  `checkAnswer`, and immutable scoring (`record`/`accuracy`). All randomness is
  **injected by the UI** (target, distractor pick, answer position), so the core
  stays deterministic; **10 new unit tests** (22 total) incl. edge cases
  (small inventories, sloppy choosers, clamping) and a real-data check.
- UI toggle wired in `main.ts` with vanilla DOM (still **zero runtime deps**);
  `main.ts` holds the only `Math.random`.

## 0.1.0 — HL02 MVP: break a script apart and write it

- **New app** (`script-writing-visualizer`): the companion "how to write it"
  surface for the Human Languages curriculum. Renders each non-Latin letter with
  its **component pieces**, a numbered **stroke order**, and a **false-friend**
  badge, for pen-and-paper practice.
- **Reads the canonical script data directly** from
  `code/learning/human-languages/data/scripts/*.json` (no copy — the app cannot
  drift from the curriculum). Ships Cyrillic, Hebrew, Chinese, Arabic, Devanagari.
- **Pure core** (`src/core.ts`) covered by unit tests, including an integration
  check against the real curriculum data (every letter has a glyph + pieces +
  stroke order; Cyrillic flags в/р/с/н as false friends).
- Framework-free vanilla-DOM UI; zero runtime dependencies.
- **Scope:** v1 is read + decompose only (no handwriting capture, no scheduler
  yet) — the first slice of the `HL02` spec.
