import{t as e}from"./rolldown-runtime-DK3Fl9T5.js";var t=e({default:()=>n}),n=`---
schema_version: 2
id: PA-W06-mixed-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1470
delivery: script
chapter: 23
type: writing
headword: "ਭਾਸ਼ਾ · ਰਿਹਾਇਸ਼ · ਕੰਮ"
romanization: "three-field one-dimension repair"
gloss: "identify and repair only the first differing dimension on three lines"
prerequisites: [PA-W06-placement-repair]
sounds: []
roots: []
duration:
  max_seconds: 175
requires:
  knowledge: [PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01]
introduces:
  knowledge: [PA-FORM-THREE-MIXED-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01, PA-FORM-THREE-MIXED-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W06-selection-repair, PA-W06-spelling-repair, PA-W06-spacing-repair, PA-W06-placement-repair]
---

# Repair one dimension, then stop

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01] -->

Name the four passes in order: selection, spelling, spacing, placement.

## Writing — one mixed repair
<!-- hl-knowledge: introduces=[PA-FORM-THREE-MIXED-REPAIR-01]; assesses=[PA-FORM-THREE-SPACING-REPAIR-01] -->

Inspect these otherwise correct lines:

> **ਭਾਸ਼ਾ: ਪੰਜਾਬੀ**
>
> **ਰਿਹਾਇਸ਼:ਸ਼ਹਿਰ**
>
> **ਕੰਮ: ਖੇਤੀ**

The cue selections, spellings, and placements are correct. Repair the first differing dimension, then stop.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-MIXED-REPAIR-01] -->

Say which pass found the difference. Do not recopy the two unchanged lines.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-MIXED-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W06-mixed-repair-check","kind":"text","assesses":["PA-FORM-THREE-MIXED-REPAIR-01"],"prompt":"Name the single dimension that differs in the middle line.","answer":"spacing","accepted":[],"feedback":{"correct":"Only the boundary after the colon needs repair.","incorrect":"The city value and its field are already correct; inspect the colon boundary."},"response_seconds":16} -->

The next lesson asks for fresh writing, not correction of a visible answer.
`,r=e({default:()=>i}),i=`---
schema_version: 2
id: PA-W06-placement-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1460
delivery: script
chapter: 23
type: writing
headword: "ਭਾਸ਼ਾ: ਪੰਜਾਬੀ · ਕੰਮ: ਖੇਤੀ"
romanization: "three-field placement repair"
gloss: "move known correctly spelled values back to their matching fields"
prerequisites: [PA-W06-spacing-repair]
sounds: []
roots: []
duration:
  max_seconds: 165
requires:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-WORK-FARMING-01]
introduces:
  knowledge: [PA-FORM-THREE-PLACEMENT-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-WORK-FARMING-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-language-no-model, PA-W05-work-no-model, PA-W06-three-field-labels, PA-W06-spacing-repair]
---

# Repair only placement

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-WORK-FARMING-01] -->

Both values are correctly selected and spelled, but these lines are crossed:

> **ਭਾਸ਼ਾ: ਖੇਤੀ**
>
> **ਕੰਮ: ਪੰਜਾਬੀ**

## Writing — move, do not respell
<!-- hl-knowledge: introduces=[PA-FORM-THREE-PLACEMENT-REPAIR-01]; assesses=[PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-SPACING-REPAIR-01] -->

Rewrite the two values on their matching fields. Preserve every letter and both spaces.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-PLACEMENT-REPAIR-01] -->

Say “placement” after the repair. Selection and spelling were already correct.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-PLACEMENT-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W06-placement-repair-check","kind":"text","assesses":["PA-FORM-THREE-PLACEMENT-REPAIR-01"],"prompt":"Name the dimension that changes when two correct values move back to their matching labels.","answer":"placement","accepted":[],"feedback":{"correct":"The values stayed intact; only their fields changed.","incorrect":"No value or letter changed, so inspect where each value sits."},"response_seconds":14} -->

All four dimensions have now received their own repair pass.
`,a=e({default:()=>o}),o=`---
schema_version: 2
id: PA-W06-selection-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1430
delivery: script
chapter: 23
type: writing
headword: "B — ਭਾਸ਼ਾ: ਹਿੰਦੀ"
romanization: "three-field selection repair"
gloss: "repair only a cue-to-value selection on the combined form"
prerequisites: [PA-W06-three-field-supported]
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: [PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-THREE-SUPPORTED-01, PA-FORM-LANGUAGE-HINDI-01]
introduces:
  knowledge: [PA-FORM-THREE-SELECTION-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-LANGUAGE-HINDI-01, PA-FORM-THREE-SELECTION-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-language-select, PA-W06-three-field-cues, PA-W06-three-field-supported]
---

# Repair only selection

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-LANGUAGE-HINDI-01] -->

The language bank still uses A — **ਪੰਜਾਬੀ**, B — **ਹਿੰਦੀ**. Read it once, then cover it.

## Writing — one selection repair
<!-- hl-knowledge: introduces=[PA-FORM-THREE-SELECTION-REPAIR-01]; assesses=[PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-HINDI-01] -->

The cue is B, but the line says **ਭਾਸ਼ਾ: ਪੰਜਾਬੀ**. Replace only the selected value. The label, colon, space, and placement remain unchanged.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SELECTION-REPAIR-01] -->

Name the repaired dimension: selection. Do not judge letter shape or spacing in this pass.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SELECTION-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W06-selection-repair-check","kind":"text","assesses":["PA-FORM-THREE-SELECTION-REPAIR-01"],"prompt":"For cue B, repair only the selected value in ਭਾਸ਼ਾ: ਪੰਜਾਬੀ.","answer":"ਭਾਸ਼ਾ: ਹਿੰਦੀ","accepted":["ਭਾਸ਼ਾ:ਹਿੰਦੀ"],"feedback":{"correct":"Only the cue-to-value selection changed.","incorrect":"Keep the language label and replace the A value with the B value."},"response_seconds":22} -->

The next pass keeps selection fixed and inspects spelling only.
`,s=e({default:()=>c}),c=`---
schema_version: 2
id: PA-W06-spacing-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1450
delivery: script
chapter: 23
type: writing
headword: "ਕੰਮ: ਖੇਤੀ"
romanization: "three-field spacing repair"
gloss: "repair only the label-value boundary on one combined-form line"
prerequisites: [PA-W06-spelling-repair]
sounds: []
roots: []
duration:
  max_seconds: 145
requires:
  knowledge: [PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-WORK-FARMING-01, PA-FORM-WORK-SPACING-01]
introduces:
  knowledge: [PA-FORM-THREE-SPACING-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-WORK-FARMING-01, PA-FORM-WORK-SPACING-01, PA-FORM-THREE-SPACING-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W05-work-spacing, PA-W05-work-repair, PA-W06-spelling-repair]
---

# Repair only spacing

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-WORK-FARMING-01, PA-FORM-WORK-SPACING-01] -->

Compare **ਕੰਮ:ਖੇਤੀ** with **ਕੰਮ: ਖੇਤੀ**. Every letter and the selected value match.

## Writing — one boundary repair
<!-- hl-knowledge: introduces=[PA-FORM-THREE-SPACING-REPAIR-01]; assesses=[PA-FORM-WORK-SPACING-01] -->

Rewrite **ਕੰਮ:ਖੇਤੀ** with one clear space after the colon. Change nothing else.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SPACING-REPAIR-01] -->

Point to the repaired boundary. Ignore selection, spelling, and line order.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SPACING-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W06-spacing-repair-check","kind":"text","assesses":["PA-FORM-THREE-SPACING-REPAIR-01"],"prompt":"Repair only the spacing in ਕੰਮ:ਖੇਤੀ.","answer":"ਕੰਮ: ਖੇਤੀ","accepted":[],"feedback":{"correct":"Only the label-value boundary changed.","incorrect":"Keep every letter and add one space after the colon."},"response_seconds":18} -->

The next pass keeps every value intact and repairs its line placement.
`,l=e({default:()=>u}),u=`---
schema_version: 2
id: PA-W06-spelling-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1440
delivery: script
chapter: 23
type: writing
headword: "ਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ"
romanization: "three-field spelling repair"
gloss: "repair only one selected value's spelling on the combined form"
prerequisites: [PA-W06-selection-repair]
sounds: []
roots: []
duration:
  max_seconds: 155
requires:
  knowledge: [PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-RESIDENCE-CITY-01]
introduces:
  knowledge: [PA-FORM-THREE-SPELLING-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-RESIDENCE-CITY-01, PA-FORM-THREE-SPELLING-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W04-city, PA-W04-residence-repair, PA-W06-selection-repair]
---

# Repair only spelling

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-RESIDENCE-CITY-01] -->

Read **ਸ਼ਹਿਰ** once. Touch **ਸ਼**, then **ਹਿ**, then **ਰ**. Cover the model.

## Writing — one spelling repair
<!-- hl-knowledge: introduces=[PA-FORM-THREE-SPELLING-REPAIR-01]; assesses=[PA-FORM-RESIDENCE-CITY-01] -->

The selected residence value is correct, but **ਰਿਹਾਇਸ਼: ਸ਼ਹਰ** is missing one spelling piece. Repair only the value. Keep its selection, line, label, colon, and space.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SPELLING-REPAIR-01] -->

Reveal **ਸ਼ਹਿਰ** and compare one piece at a time. Do not change another line.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SPELLING-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W06-spelling-repair-check","kind":"text","assesses":["PA-FORM-THREE-SPELLING-REPAIR-01"],"prompt":"Repair only the spelling in ਰਿਹਾਇਸ਼: ਸ਼ਹਰ.","answer":"ਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ","accepted":["ਰਿਹਾਇਸ਼:ਸ਼ਹਿਰ"],"feedback":{"correct":"The missing spelling piece returned and every other dimension stayed fixed.","incorrect":"Keep the selected city value and restore the ਿ placement in ਸ਼ਹਿਰ."},"response_seconds":24} -->

The next pass ignores spelling and repairs one boundary.
`,d=e({default:()=>f}),f=`---
schema_version: 2
id: PA-W06-three-field-cues
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1400
delivery: script
chapter: 22
type: writing
headword: "A · B · A"
romanization: "three remembered bank cues"
gloss: "select one value from each already practised closed bank"
prerequisites: [PA-W06-three-field-labels]
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-LANGUAGE-CUE-MAP-01, PA-FORM-RESIDENCE-CUE-MAP-01, PA-FORM-WORK-CUE-MAP-01]
introduces:
  knowledge: [PA-FORM-THREE-CUE-SELECTION-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-LANGUAGE-CUE-MAP-01, PA-FORM-RESIDENCE-CUE-MAP-01, PA-FORM-WORK-CUE-MAP-01, PA-FORM-THREE-CUE-SELECTION-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-language-select, PA-W04-residence-select, PA-W05-work-select, PA-W06-three-field-labels]
---

# Select three values before writing

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LANGUAGE-CUE-MAP-01, PA-FORM-RESIDENCE-CUE-MAP-01, PA-FORM-WORK-CUE-MAP-01] -->

Read each closed practice bank once:

> ਭਾਸ਼ਾ: A — **ਪੰਜਾਬੀ** · B — **ਹਿੰਦੀ**
>
> ਰਿਹਾਇਸ਼: A — **ਪਿੰਡ** · B — **ਸ਼ਹਿਰ**
>
> ਕੰਮ: A — **ਖੇਤੀ** · B — **ਨੌਕਰੀ**

## Script — one cue per field
<!-- hl-knowledge: introduces=[PA-FORM-THREE-CUE-SELECTION-01]; assesses=[PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-LANGUAGE-CUE-MAP-01, PA-FORM-RESIDENCE-CUE-MAP-01, PA-FORM-WORK-CUE-MAP-01] -->

For the cue row **A · B · A**, point to the selected language, residence, and work values in that order. Do not copy them.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-CUE-SELECTION-01] -->

Cover all three banks and name the selections once. Reveal and check only selection; spelling and form placement wait.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-CUE-SELECTION-01] -->
<!-- hl-activity: {"id":"PA-W06-three-field-cues-check","kind":"text","assesses":["PA-FORM-THREE-CUE-SELECTION-01"],"prompt":"For A · B · A, write the selected value sequence.","answer":"ਪੰਜਾਬੀ · ਸ਼ਹਿਰ · ਖੇਤੀ","accepted":[],"feedback":{"correct":"Each cue was read inside its own closed bank.","incorrect":"Return to one bank at a time; A and B do not carry one value across all fields."},"response_seconds":18} -->

Selection is now ready for a two-line writing bridge.
`,p=e({default:()=>m}),m=`---
schema_version: 2
id: PA-W06-three-field-labels
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1390
delivery: script
chapter: 22
type: writing
headword: "ਭਾਸ਼ਾ · ਰਿਹਾਇਸ਼ · ਕੰਮ"
romanization: "three known form labels"
gloss: "read and place the known language, residence, and work labels"
prerequisites: [PA-W05-work-no-model]
sounds: []
roots: []
duration:
  max_seconds: 130
requires:
  knowledge: [PA-FORM-LABEL-LANGUAGE-01, PA-FORM-LABEL-RESIDENCE-01, PA-FORM-LABEL-WORK-01]
introduces:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-LANGUAGE-01, PA-FORM-LABEL-RESIDENCE-01, PA-FORM-LABEL-WORK-01, PA-FORM-THREE-LABEL-ORDER-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-language-label, PA-W04-residence-label, PA-W05-work-label]
---

# Put three known labels on one small form

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-LANGUAGE-01, PA-FORM-LABEL-RESIDENCE-01, PA-FORM-LABEL-WORK-01] -->

Read the three labels once: **ਭਾਸ਼ਾ**, **ਰਿਹਾਇਸ਼**, **ਕੰਮ**. Each label is old; only seeing all three together is new.

## Script — one placement pass
<!-- hl-knowledge: introduces=[PA-FORM-THREE-LABEL-ORDER-01]; assesses=[PA-FORM-LABEL-LANGUAGE-01, PA-FORM-LABEL-RESIDENCE-01, PA-FORM-LABEL-WORK-01] -->

Point from the first blank line to the third. Place **ਭਾਸ਼ਾ** first, **ਰਿਹਾਇਸ਼** second, and **ਕੰਮ** third. Do not choose or write any value.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-LABEL-ORDER-01] -->

Cover the labels. Say which known label belongs on the middle line, then reveal and check only its placement.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-LABEL-ORDER-01] -->
<!-- hl-activity: {"id":"PA-W06-three-field-labels-check","kind":"text","assesses":["PA-FORM-THREE-LABEL-ORDER-01"],"prompt":"Write the known label that belongs on the middle line.","answer":"ਰਿਹਾਇਸ਼","accepted":[],"feedback":{"correct":"The residence label holds the middle line in this practice form.","incorrect":"Read the fixed order once more: language, residence, work."},"response_seconds":12} -->

Stop after the labels are in place. Values arrive in the next lesson.
`,h=e({default:()=>g}),g=`---
schema_version: 2
id: PA-W06-three-field-no-model
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1480
delivery: script
chapter: 23
type: writing
headword: "ਭਾਸ਼ਾ: ______ · ਰਿਹਾਇਸ਼: ______ · ਕੰਮ: ______"
romanization: "three-field no-model checkpoint"
gloss: "fill three known fields from remembered closed-bank cues with no model"
prerequisites: [PA-W06-mixed-repair]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01, PA-FORM-THREE-MIXED-REPAIR-01]
introduces:
  knowledge: [PA-FORM-THREE-NO-MODEL-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-LANGUAGE-01, PA-FORM-LABEL-RESIDENCE-01, PA-FORM-LABEL-WORK-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01, PA-FORM-WORK-FARMING-01, PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01, PA-FORM-THREE-MIXED-REPAIR-01, PA-FORM-THREE-NO-MODEL-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-language-no-model, PA-W04-residence-no-model, PA-W05-work-no-model, PA-W06-mixed-repair]
---

# Three-field no-model checkpoint

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01] -->

Close every field lesson. There is no value bank, support-language label, romanization, or copyable answer below.

## Writing — independent controlled choice
<!-- hl-knowledge: introduces=[PA-FORM-THREE-NO-MODEL-01]; assesses=[PA-FORM-LABEL-LANGUAGE-01, PA-FORM-LABEL-RESIDENCE-01, PA-FORM-LABEL-WORK-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01, PA-FORM-WORK-FARMING-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-THREE-SELECTION-REPAIR-01, PA-FORM-THREE-SPELLING-REPAIR-01, PA-FORM-THREE-SPACING-REPAIR-01, PA-FORM-THREE-PLACEMENT-REPAIR-01] -->
<!-- hl-writing-stage: controlled-composition -->

Complete the three requested fields:

> A — **ਭਾਸ਼ਾ: __________**
>
> B — **ਰਿਹਾਇਸ਼: __________**
>
> A — **ਕੰਮ: __________**

Use the three remembered closed banks. Finish all three attempts before reopening any earlier lesson.

## Guided Practice — separate repair passes
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-MIXED-REPAIR-01, PA-FORM-THREE-NO-MODEL-01] -->

After writing, compare selection, spelling, spacing, then placement. Repair only the first differing dimension and stop.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-NO-MODEL-01] -->
<!-- hl-activity: {"id":"PA-W06-three-field-no-model-check","kind":"text","assesses":["PA-FORM-THREE-NO-MODEL-01"],"prompt":"With no bank, romanization, or copyable answer, complete the three fields for cues A · B · A.","answer":"ਭਾਸ਼ਾ: ਪੰਜਾਬੀ\\nਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ\\nਕੰਮ: ਖੇਤੀ","accepted":[],"feedback":{"correct":"All three remembered cues produced values on their matching fields.","incorrect":"Finish all three attempts first; then reopen one bank at a time and repair one dimension."},"response_seconds":55} -->

This checkpoint integrates only language, residence, and work. It does not claim the later six-field form is complete.
`,_=e({default:()=>v}),v=`---
schema_version: 2
id: PA-W06-three-field-supported
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1420
delivery: script
chapter: 22
type: writing
headword: "ਭਾਸ਼ਾ · ਰਿਹਾਇਸ਼ · ਕੰਮ"
romanization: "three supported form lines"
gloss: "join all three known fields while every closed bank remains visible"
prerequisites: [PA-W06-two-field-supported]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-FORM-THREE-TWO-LINE-SUPPORTED-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-WORK-FARMING-01]
introduces:
  knowledge: [PA-FORM-THREE-SUPPORTED-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-THREE-TWO-LINE-SUPPORTED-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01, PA-FORM-WORK-FARMING-01, PA-FORM-THREE-SUPPORTED-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W05-work-supported, PA-W06-three-field-cues, PA-W06-two-field-supported]
---

# Add the third supported line

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01, PA-FORM-WORK-FARMING-01] -->

Keep all three old banks visible. Select the cue row **A · B · A** without writing.

## Writing — all models remain visible
<!-- hl-knowledge: introduces=[PA-FORM-THREE-SUPPORTED-01]; assesses=[PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-THREE-TWO-LINE-SUPPORTED-01] -->

Complete one language line, one residence line, and one work line. Copy from the visible banks. This is supported entry, not independent writing evidence.

## Guided Practice — one pass at a time
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SUPPORTED-01] -->

Check selection on all three lines. Then check spelling. Then check spacing and placement. Do not mix the passes.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-SUPPORTED-01] -->
<!-- hl-activity: {"id":"PA-W06-three-field-supported-check","kind":"text","assesses":["PA-FORM-THREE-SUPPORTED-01"],"prompt":"Does this visible-bank practice count as independent writing evidence?","answer":"no","accepted":[],"feedback":{"correct":"Visible answers support assembly but do not prove independent recall.","incorrect":"The models remain visible, so this is preparation only."},"response_seconds":10} -->

Close the banks before beginning the separate repair lessons.
`,y=e({default:()=>b}),b=`---
schema_version: 2
id: PA-W06-two-field-supported
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1410
delivery: script
chapter: 22
type: writing
headword: "ਭਾਸ਼ਾ: ਪੰਜਾਬੀ · ਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ"
romanization: "two supported form lines"
gloss: "join the known language and residence fields with both banks visible"
prerequisites: [PA-W06-three-field-cues]
sounds: []
roots: []
duration:
  max_seconds: 170
requires:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01]
introduces:
  knowledge: [PA-FORM-THREE-TWO-LINE-SUPPORTED-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01, PA-FORM-THREE-TWO-LINE-SUPPORTED-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-language-supported, PA-W04-residence-supported, PA-W06-three-field-cues]
---

# Join two supported lines first

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-CUE-SELECTION-01, PA-FORM-LANGUAGE-PUNJABI-01, PA-FORM-RESIDENCE-CITY-01] -->

Keep the language and residence banks visible. Read the cue pair **A · B** and point to **ਪੰਜਾਬੀ**, then **ਸ਼ਹਿਰ**.

## Writing — two models remain visible
<!-- hl-knowledge: introduces=[PA-FORM-THREE-TWO-LINE-SUPPORTED-01]; assesses=[PA-FORM-THREE-LABEL-ORDER-01, PA-FORM-THREE-CUE-SELECTION-01] -->

Complete only these two lines:

> A — **ਭਾਸ਼ਾ: __________**
>
> B — **ਰਿਹਾਇਸ਼: __________**

Copy the selected values while their banks remain visible. This is supported entry, not independent writing evidence.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-TWO-LINE-SUPPORTED-01] -->

Check the first line, then the second. Stop before adding a work line.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-THREE-TWO-LINE-SUPPORTED-01] -->
<!-- hl-activity: {"id":"PA-W06-two-field-supported-check","kind":"text","assesses":["PA-FORM-THREE-TWO-LINE-SUPPORTED-01"],"prompt":"How many lines belong in this bridge?","answer":"two","accepted":["2"],"feedback":{"correct":"Two supported lines make the smallest bridge into the combined form.","incorrect":"Stop after language and residence; work joins next."},"response_seconds":10} -->

The third line joins only after two lines feel small.
`,x=e({default:()=>S}),S=`---
schema_version: 2
id: PA-W07-age-delayed
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1750
delivery: script
chapter: 29
type: writing
headword: "ਉਮਰ: __________"
romanization: "age field after a short delay"
gloss: "fill one age line after hiding the known bank"
prerequisites: [PA-W07-age-spacing]
sounds: []
roots: []
duration:
  max_seconds: 175
requires:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-SPACING-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-SPACING-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-select, PA-W07-age-supported, PA-W07-age-spacing]
---

# Hide, wait, then fill the age field

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-CUE-MAP-01] -->

Read A — **੧੫**, B — **੨੫** once. Hide the bank and count slowly to ten.

## Writing — short delayed entry
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-SPACING-01] -->
<!-- hl-writing-stage: delayed-copy -->

With the bank hidden, complete **ਉਮਰ: __________** for B. Finish before
revealing the bank.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-SPACING-01] -->

Reveal and compare selection, digit order, spacing, then placement. Do not
repair yet.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-SPACING-01] -->
<!-- hl-activity: {"id":"PA-W07-age-delayed-b","kind":"text","assesses":["PA-FORM-AGE-CUE-MAP-01","PA-FORM-AGE-SPACING-01"],"prompt":"After hiding the bank for ten seconds, complete the age field for B.","answer":"੨੫","accepted":["ਉਮਰ: ੨੫","ਉਮਰ:੨੫"],"feedback":{"correct":"The hidden-bank cue selected ੨੫.","incorrect":"Finish the attempt, then reveal the bank and compare."},"response_seconds":28} -->

One delayed line is enough.
`,C=e({default:()=>w}),w=`---
schema_version: 2
id: PA-W07-age-dictation
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1760
delivery: script
chapter: 29
type: writing
headword: "੧੫ / ੨੫"
romanization: "15 / 25"
gloss: "transcribe one heard fictional age into Gurmukhi digits"
prerequisites: [PA-W07-age-delayed]
sounds: []
roots: []
duration:
  max_seconds: 160
requires:
  knowledge: [PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01]
skills: [listening, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-fifteen, PA-W07-age-twenty-five, PA-W07-age-delayed]
---

# Hear a number, write its Gurmukhi digits

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01] -->

Hide every written value. This task checks digit transcription, not new Punjabi
number words.

## Writing — one support-language dictation cue
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01] -->
<!-- hl-writing-stage: dictation-transcription -->

Have a partner or screen reader say the support-language number **fifteen** once.
Write only its Gurmukhi digits. No model is visible during the attempt.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-FIFTEEN-01] -->

Reveal **੧੫** and compare the first digit, then the second digit. Repair only
after the comparison.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-FIFTEEN-01] -->
<!-- hl-activity: {"id":"PA-W07-age-dictation-fifteen","kind":"text","assesses":["PA-FORM-AGE-FIFTEEN-01"],"prompt":"After hearing the support-language number fifteen with no written model, write it in Gurmukhi digits.","answer":"੧੫","accepted":[],"feedback":{"correct":"The heard cue became ੧੫.","incorrect":"Compare first digit, then second digit: ੧੫."},"response_seconds":18} -->

Punjabi spoken forms beyond one to five remain future instruction.
`,T=e({default:()=>E}),E=`---
schema_version: 2
id: PA-W07-age-fifteen
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1700
delivery: script
chapter: 29
type: writing
headword: "੧੫"
romanization: "15"
gloss: "assemble the fictional age value 15"
prerequisites: [PA-W07-digit-five]
sounds: []
roots: []
duration:
  max_seconds: 165
requires:
  knowledge: [PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-FIVE-01]
introduces:
  knowledge: [PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-FIFTEEN-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-FIFTEEN-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-digit-one, PA-W07-digit-five]
---

# ੧੫ — one fictional age value

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Write **੧**, then **੫**, with a small gap.

## Script — keep the digit order
<!-- hl-knowledge: introduces=[PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-FIFTEEN-01]; assesses=[PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Close the gap: **੧ + ੫ = ੧੫**, the digit entry 15. Left-to-right order matters.
Copy this fictional age value once.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-FIFTEEN-01] -->

Cover the model, write 15 in Gurmukhi digits, then compare only digit order.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-FIFTEEN-01] -->
<!-- hl-activity: {"id":"PA-W07-age-fifteen-check","kind":"text","assesses":["PA-FORM-AGE-FIFTEEN-01"],"prompt":"Write the fictional age value 15 in Gurmukhi digits.","answer":"੧੫","accepted":[],"feedback":{"correct":"੧੫ is 15.","incorrect":"Keep ੧ before ੫: ੧੫."},"response_seconds":12} -->

This is invented practice data.
`,D=e({default:()=>O}),O=`---
schema_version: 2
id: PA-W07-age-label
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1660
delivery: script
chapter: 29
type: writing
headword: "ਉਮਰ"
romanization: "umar"
gloss: "the form label age"
prerequisites: [PA-W07-independent-u]
sounds: []
roots: []
duration:
  max_seconds: 170
requires:
  knowledge: [PA-SCRIPT-INDEPENDENT-U-01, PA-SCRIPT-MA-01, PA-SCRIPT-RA-01]
introduces:
  knowledge: [PA-FORM-LABEL-AGE-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-INDEPENDENT-U-01, PA-SCRIPT-MA-01, PA-SCRIPT-RA-01, PA-FORM-LABEL-AGE-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-independent-u, PA-W01-ma, PA-W04-ra]
---

# ਉਮਰ — assemble the age label

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-INDEPENDENT-U-01, PA-SCRIPT-MA-01, PA-SCRIPT-RA-01] -->

Touch the pieces in order: **ਉ**, **ਮ**, **ਰ**.

## Script — one label
<!-- hl-knowledge: introduces=[PA-FORM-LABEL-AGE-01]; assesses=[PA-SCRIPT-INDEPENDENT-U-01, PA-SCRIPT-MA-01, PA-SCRIPT-RA-01] -->

Join **ਉ + ਮ + ਰ = ਉਮਰ**. On this fictional beginner form, **ਉਮਰ** labels the
age line. Copy the label once and leave the value blank.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01] -->

Point to **ਉਮਰ** and say what information belongs there. Do not add a number.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01] -->
<!-- hl-activity: {"id":"PA-W07-age-label-check","kind":"text","assesses":["PA-FORM-LABEL-AGE-01"],"prompt":"A fictional practice form shows ਉਮਰ. What belongs on that line?","answer":"age","accepted":[],"feedback":{"correct":"ਉਮਰ labels the age field.","incorrect":"ਉਮਰ labels age on this practice form."},"response_seconds":12} -->

No real personal information is requested.
`,k=e({default:()=>A}),A=`---
schema_version: 2
id: PA-W07-age-no-model
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1780
delivery: script
chapter: 29
type: writing
headword: "ਉਮਰ: __________"
romanization: "age field"
gloss: "choose and write one known fictional age value from a nonverbal cue with no model"
prerequisites: [PA-W07-age-repair]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-SPACING-01, PA-FORM-AGE-REPAIR-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-SPACING-01, PA-FORM-AGE-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-label, PA-W07-age-select, PA-W07-age-delayed, PA-W07-age-repair]
---

# One no-model age field

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01] -->

Close the previous lessons. There is no value bank, support-language label, or
copyable digit answer below.

## Writing — independent controlled choice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-SPACING-01] -->
<!-- hl-writing-stage: controlled-composition -->

Complete the requested fictional field:

> A — **ਉਮਰ: __________**

Choose the known value attached to A and write it from memory. Do not reopen an
earlier lesson until the attempt is complete.

## Guided Practice — separate repair passes
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-REPAIR-01] -->
<!-- hl-writing-stage: controlled-composition -->

After writing, compare selection, digit order, spacing, and placement. Repair
only the first differing dimension.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-FIFTEEN-01] -->
<!-- hl-activity: {"id":"PA-W07-age-no-model-a","kind":"text","assesses":["PA-FORM-AGE-FIFTEEN-01"],"prompt":"With no bank or copyable answer, complete the fictional age field for A.","answer":"੧੫","accepted":["ਉਮਰ: ੧੫","ਉਮਰ:੧੫"],"feedback":{"correct":"The nonverbal cue independently selected ੧੫.","incorrect":"Finish the attempt first; then reopen the bank and repair one dimension."},"response_seconds":30} -->

This closes only the untimed age-field ladder. Phone, date, integration, and A1
readiness remain separate dependency-linked work.
`,j=e({default:()=>M}),M=`---
schema_version: 2
id: PA-W07-age-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1770
delivery: script
chapter: 29
type: writing
headword: "ਉਮਰ: ੨੫"
romanization: "age-field repair"
gloss: "repair selection, digit order, spacing, or placement one dimension at a time"
prerequisites: [PA-W07-age-dictation]
sounds: []
roots: []
duration:
  max_seconds: 175
requires:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-SPACING-01]
introduces:
  knowledge: [PA-FORM-AGE-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-SPACING-01, PA-FORM-AGE-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus, meaning-output]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-select, PA-W07-age-spacing, PA-W07-age-delayed, PA-W07-age-dictation]
---

# Repair one dimension, then stop

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-SPACING-01] -->

For B, inspect **ਉਮਰ: ੫੨**. The label and spacing are correct; digit order is not.

## Writing — repair one named dimension
<!-- hl-knowledge: introduces=[PA-FORM-AGE-REPAIR-01]; assesses=[PA-FORM-AGE-TWO-DIGIT-ORDER-01] -->

Repair only digit order: **ਉਮਰ: ੨੫**. Do not rewrite a correct label or boundary.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-REPAIR-01] -->

Name the four checks in order: selection, digit order, spacing, placement. Stop
at the first mismatch.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W07-age-repair-order","kind":"text","assesses":["PA-FORM-AGE-REPAIR-01"],"prompt":"For B, repair only digit order in ਉਮਰ: ੫੨.","answer":"ਉਮਰ: ੨੫","accepted":[],"feedback":{"correct":"Only digit order changed.","incorrect":"Keep the label and spacing; reorder ੫੨ to ੨੫."},"response_seconds":18} -->

One bounded repair is enough.
`,N=e({default:()=>P}),P=`---
schema_version: 2
id: PA-W07-age-select
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1720
delivery: script
chapter: 29
type: writing
headword: "A / B"
romanization: "two fictional age cues"
gloss: "select between two separately practised age values"
prerequisites: [PA-W07-age-twenty-five]
sounds: []
roots: []
duration:
  max_seconds: 165
requires:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01]
introduces:
  knowledge: [PA-FORM-AGE-CUE-MAP-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01, PA-FORM-AGE-CUE-MAP-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-label, PA-W07-age-fifteen, PA-W07-age-twenty-five]
---

# Select one known age value

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01] -->

Read the fictional bank once:

> A — **੧੫**
>
> B — **੨੫**

## Script — attach one cue to each value
<!-- hl-knowledge: introduces=[PA-FORM-AGE-CUE-MAP-01]; assesses=[PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-TWENTY-FIVE-01] -->

Point A to **੧੫** and B to **੨੫**. Cover the bank and select the value requested
by B. Selection is separate from digit writing.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-CUE-MAP-01] -->

Reveal, point, cover, and select A once more. Do not fill the form yet.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-CUE-MAP-01] -->
<!-- hl-activity: {"id":"PA-W07-age-select-b","kind":"text","assesses":["PA-FORM-AGE-CUE-MAP-01"],"prompt":"In this fictional bank, which Gurmukhi-digit value is paired with B?","answer":"੨੫","accepted":[],"feedback":{"correct":"B requests ੨੫.","incorrect":"Reveal the bank, point to B and ੨੫, then cover it again."},"response_seconds":10} -->

These cues belong only to this invented bank.
`,F=e({default:()=>I}),I=`---
schema_version: 2
id: PA-W07-age-spacing
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1740
delivery: script
chapter: 29
type: writing
headword: "ਉਮਰ: ੧੫"
romanization: "age label and value boundary"
gloss: "keep the age label and value visibly separate"
prerequisites: [PA-W07-age-supported]
sounds: []
roots: []
duration:
  max_seconds: 145
requires:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01]
introduces:
  knowledge: [PA-FORM-AGE-SPACING-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01, PA-FORM-AGE-SPACING-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus, meaning-output]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-supported]
---

# One clear boundary

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-FIFTEEN-01] -->

Read **ਉਮਰ:੧੫**. The label and value are both correct, but crowded.

## Writing — repair only spacing
<!-- hl-knowledge: introduces=[PA-FORM-AGE-SPACING-01]; assesses=[PA-FORM-LABEL-AGE-01] -->

Rewrite it as **ਉਮਰ: ੧੫**. Keep one visible space after the colon. Change no
digit and no letter.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-SPACING-01] -->

Point once to the label, the boundary, and the value.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-SPACING-01] -->
<!-- hl-activity: {"id":"PA-W07-age-spacing-check","kind":"text","assesses":["PA-FORM-AGE-SPACING-01"],"prompt":"Repair only the spacing in ਉਮਰ:੧੫.","answer":"ਉਮਰ: ੧੫","accepted":[],"feedback":{"correct":"The label and value now have a clear boundary.","incorrect":"Keep all shapes; add one space after the colon."},"response_seconds":12} -->

One spacing repair is enough.
`,L=e({default:()=>R}),R=`---
schema_version: 2
id: PA-W07-age-supported
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1730
delivery: script
chapter: 29
type: writing
headword: "ਉਮਰ: __________"
romanization: "age field"
gloss: "fill one age line with a visible bank"
prerequisites: [PA-W07-age-select, PA-S02-mamma-rara-lava]
sounds: []
roots: []
duration:
  max_seconds: 175
requires:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-SCRIPT-RECOG-MA-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-SCRIPT-RECOG-MA-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-age-label, PA-W07-age-select, PA-S02-mamma-rara-lava]
---

# Fill one supported age field

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01] -->

Keep the fictional bank visible: A — **੧੫**, B — **੨੫**.

## Writing — model visible
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01] -->
<!-- hl-writing-stage: guided-copy -->

Complete **ਉਮਰ: __________** for A. Read the label, select A, and copy **੧੫**.
This is supported entry, not independent writing evidence.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01] -->

Compare selection, digit order, spacing, and placement in separate passes.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-AGE-01, PA-FORM-AGE-CUE-MAP-01, PA-SCRIPT-RECOG-MA-01] -->
<!-- hl-activity: {"id":"PA-W07-age-supported-a","kind":"text","assesses":["PA-FORM-LABEL-AGE-01","PA-FORM-AGE-CUE-MAP-01"],"prompt":"With the fictional bank visible, complete the age field for A.","answer":"੧੫","accepted":["ਉਮਰ: ੧੫","ਉਮਰ:੧੫"],"feedback":{"correct":"A selects ੧੫.","incorrect":"Use the visible bank and repair only the first differing piece."},"response_seconds":22} -->

Stop after one supported line.
`,z=e({default:()=>B}),B=`---
schema_version: 2
id: PA-W07-age-twenty-five
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1710
delivery: script
chapter: 29
type: writing
headword: "੨੫"
romanization: "25"
gloss: "assemble the fictional age value 25"
prerequisites: [PA-W07-age-fifteen]
sounds: []
roots: []
duration:
  max_seconds: 160
requires:
  knowledge: [PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01]
introduces:
  knowledge: [PA-FORM-AGE-TWENTY-FIVE-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-AGE-TWO-DIGIT-ORDER-01, PA-FORM-AGE-TWENTY-FIVE-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-digit-two, PA-W07-digit-five, PA-W07-age-fifteen]
---

# ੨੫ — a second fictional age value

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Write **੨**, then **੫**, separately.

## Script — reuse the same order rule
<!-- hl-knowledge: introduces=[PA-FORM-AGE-TWENTY-FIVE-01]; assesses=[PA-FORM-AGE-TWO-DIGIT-ORDER-01] -->

Join them: **੨ + ੫ = ੨੫**, the digit entry 25. Copy this fictional value once.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-TWENTY-FIVE-01] -->

Cover the model, write 25 in Gurmukhi digits, and check digit order.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-AGE-TWENTY-FIVE-01] -->
<!-- hl-activity: {"id":"PA-W07-age-twenty-five-check","kind":"text","assesses":["PA-FORM-AGE-TWENTY-FIVE-01"],"prompt":"Write the fictional age value 25 in Gurmukhi digits.","answer":"੨੫","accepted":[],"feedback":{"correct":"੨੫ is 25.","incorrect":"Keep ੨ before ੫: ੨੫."},"response_seconds":12} -->

The closed two-value bank is now fully taught.
`,V=e({default:()=>H}),H=`---
schema_version: 2
id: PA-W07-digit-five
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1690
delivery: script
chapter: 29
type: writing
headword: "੫"
romanization: "five"
gloss: "trace the Gurmukhi digit five"
prerequisites: [PA-W07-digit-two]
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: [PA-LEX-NUMBERS-ONE-TO-FIVE, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01]
introduces:
  knowledge: [PA-SCRIPT-DIGIT-FIVE-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-LEX-NUMBERS-ONE-TO-FIVE, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-digit-one, PA-W07-digit-two, PA-C06-numbers-1-5]
---

# ੫ — add the digit five

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-LEX-NUMBERS-ONE-TO-FIVE] -->

Write **੧, ੨**. Say the already-known Punjabi word for five: **ਪੰਜ** (*panj*).

## Script — one digit
<!-- hl-knowledge: introduces=[PA-SCRIPT-DIGIT-FIVE-01]; assesses=[PA-LEX-NUMBERS-ONE-TO-FIVE] -->

Trace **੫** twice. It is the Gurmukhi digit for 5.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-FIVE-01] -->

Cover the model and write **੫** once.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-FIVE-01] -->
<!-- hl-activity: {"id":"PA-W07-digit-five-check","kind":"text","assesses":["PA-SCRIPT-DIGIT-FIVE-01"],"prompt":"Write the Gurmukhi digit for 5.","answer":"੫","accepted":[],"feedback":{"correct":"੫ is 5.","incorrect":"Trace ੫ once more."},"response_seconds":10} -->

The three needed digit shapes are now separate and known.
`,U=e({default:()=>W}),W=`---
schema_version: 2
id: PA-W07-digit-one
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1670
delivery: script
chapter: 29
type: writing
headword: "੧"
romanization: "one"
gloss: "trace the Gurmukhi digit one"
prerequisites: [PA-W07-age-label]
sounds: []
roots: []
duration:
  max_seconds: 145
requires:
  knowledge: [PA-LEX-NUMBERS-ONE-TO-FIVE]
introduces:
  knowledge: [PA-SCRIPT-DIGIT-ONE-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-LEX-NUMBERS-ONE-TO-FIVE, PA-SCRIPT-DIGIT-ONE-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-C06-numbers-1-5]
---

# ੧ — the first Gurmukhi digit

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-LEX-NUMBERS-ONE-TO-FIVE] -->

Say the already-known Punjabi word for one: **ਇੱਕ** (*ikk*).

## Script — one digit
<!-- hl-knowledge: introduces=[PA-SCRIPT-DIGIT-ONE-01]; assesses=[PA-LEX-NUMBERS-ONE-TO-FIVE] -->

Trace **੧** twice. This is the Gurmukhi digit for 1. The spoken word was known;
only the digit shape is new.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ONE-01] -->

Cover the model and write **੧** once.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ONE-01] -->
<!-- hl-activity: {"id":"PA-W07-digit-one-check","kind":"text","assesses":["PA-SCRIPT-DIGIT-ONE-01"],"prompt":"Write the Gurmukhi digit for 1.","answer":"੧","accepted":[],"feedback":{"correct":"੧ is 1.","incorrect":"Trace ੧ once more."},"response_seconds":10} -->

One digit is enough.
`,G=e({default:()=>K}),K=`---
schema_version: 2
id: PA-W07-digit-two
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1680
delivery: script
chapter: 29
type: writing
headword: "੨"
romanization: "two"
gloss: "trace the Gurmukhi digit two"
prerequisites: [PA-W07-digit-one, PA-S07-rara-ghagga-dadda]
sounds: []
roots: []
duration:
  max_seconds: 145
requires:
  knowledge: [PA-LEX-NUMBERS-ONE-TO-FIVE, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-RECOG-DA-01]
introduces:
  knowledge: [PA-SCRIPT-DIGIT-TWO-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-LEX-NUMBERS-ONE-TO-FIVE, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-RECOG-DA-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-digit-one, PA-C06-numbers-1-5, PA-S07-rara-ghagga-dadda]
---

# ੨ — add the digit two

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ONE-01, PA-LEX-NUMBERS-ONE-TO-FIVE] -->

Write **੧** and say the already-known Punjabi word for two: **ਦੋ** (*do*).

## Script — one digit
<!-- hl-knowledge: introduces=[PA-SCRIPT-DIGIT-TWO-01]; assesses=[PA-LEX-NUMBERS-ONE-TO-FIVE] -->

Trace **੨** twice. It is the Gurmukhi digit for 2.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-TWO-01] -->

Cover the model and write **੨** once.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-RECOG-DA-01] -->
<!-- hl-activity: {"id":"PA-W07-digit-two-check","kind":"text","assesses":["PA-SCRIPT-DIGIT-TWO-01"],"prompt":"Write the Gurmukhi digit for 2.","answer":"੨","accepted":[],"feedback":{"correct":"੨ is 2.","incorrect":"Trace ੨ once more."},"response_seconds":10} -->

Stop after this second digit.
`,q=e({default:()=>J}),J=`---
schema_version: 2
id: PA-W07-independent-u
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1650
delivery: script
chapter: 29
type: writing
headword: "ਉ"
romanization: "u"
gloss: "trace the independent Gurmukhi vowel u"
prerequisites: [PA-R28-head-na-r4, PA-S09-dulainkar-oora]
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: [PA-SCRIPT-MA-01, PA-SCRIPT-RA-01, PA-SCRIPT-RECOG-OORA-01]
introduces:
  knowledge: [PA-SCRIPT-INDEPENDENT-U-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-MA-01, PA-SCRIPT-RA-01, PA-SCRIPT-INDEPENDENT-U-01, PA-SCRIPT-RECOG-OORA-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W01-ma, PA-W04-ra, PA-R28-head-na-r4, PA-S09-dulainkar-oora]
---

# ਉ — one new shape for the age label

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-MA-01, PA-SCRIPT-RA-01, PA-SCRIPT-RECOG-OORA-01] -->

Write the familiar **ਮ** and **ਰ** once each. The age label needs only one new
letter before those known pieces can return.

## Script — one tiny step
<!-- hl-knowledge: introduces=[PA-SCRIPT-INDEPENDENT-U-01]; assesses=[] -->

Trace **ਉ** slowly twice. It is the independent vowel *u*. Do not assemble a
word yet.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-INDEPENDENT-U-01] -->

Cover the model, write **ਉ** once, and repair only the first shape difference.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-INDEPENDENT-U-01] -->
<!-- hl-activity: {"id":"PA-W07-independent-u-check","kind":"text","assesses":["PA-SCRIPT-INDEPENDENT-U-01"],"prompt":"Which new independent Gurmukhi vowel did you trace?","answer":"ਉ","accepted":[],"feedback":{"correct":"ਉ is the one new vowel shape.","incorrect":"Trace ਉ once more, then stop."},"response_seconds":12} -->

Stop after one new letter.
`,Y=e({default:()=>X}),X=`---
schema_version: 2
id: PA-W08-digit-recognition
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1860
delivery: script
chapter: 30
type: writing
headword: "੦ · ੧ · ੨ · ੫"
romanization: "zero, one, two, five"
gloss: "recognise and produce each introduced phone digit separately"
prerequisites: [PA-W08-phone-b]
sounds: []
roots: []
duration:
  max_seconds: 175
requires:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-digit-zero, PA-W08-phone-a, PA-W08-phone-b]
---

# Read one place at a time

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Shuffle four cards marked **੦, ੧, ੨, ੫**. Name each card before writing.

## You'll want to know — locate, do not guess
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->

In A, circle the second digit. In B, circle the fifth digit. Read the complete
values only after locating those positions.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-writing-stage: delayed-copy -->

Hear: **zero, one, two, two, five, one**. Write one digit after each word, then
compare with A.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-activity: {"id":"PA-W08-digit-recognition-check","kind":"text","assesses":["PA-SCRIPT-DIGIT-ZERO-01","PA-SCRIPT-DIGIT-ONE-01","PA-SCRIPT-DIGIT-TWO-01","PA-SCRIPT-DIGIT-FIVE-01","PA-FORM-PHONE-DIGIT-ORDER-01"],"prompt":"Write these heard digits in order: zero, one, two, two, five, one.","answer":"੦੧੨੨੫੧","accepted":[],"feedback":{"correct":"Each heard digit maps to one introduced Gurmukhi shape.","incorrect":"Replay one digit at a time and repair only the first mismatch."},"response_seconds":28} -->

No unintroduced numeral is scored.
`,Z=e({default:()=>Q}),Q=`---
schema_version: 2
id: PA-W08-digit-zero
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1830
delivery: script
chapter: 30
type: writing
headword: "੦"
romanization: "zero"
gloss: "trace the Gurmukhi digit zero"
prerequisites: [PA-W08-phone-label]
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: [PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01]
introduces:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W07-digit-one, PA-W07-digit-two, PA-W07-digit-five]
---

# ੦ — add the digit zero

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Write the three known Gurmukhi digits: **੧, ੨, ੫**.

## Script — one digit
<!-- hl-knowledge: introduces=[PA-SCRIPT-DIGIT-ZERO-01]; assesses=[PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Trace **੦** twice. It is the Gurmukhi digit zero. Phone values are identifiers,
so a leading zero stays visible.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01] -->
<!-- hl-writing-stage: guided-copy -->

Cover the model and write **੦** once.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01] -->
<!-- hl-activity: {"id":"PA-W08-digit-zero-check","kind":"text","assesses":["PA-SCRIPT-DIGIT-ZERO-01"],"prompt":"Write the Gurmukhi digit for zero.","answer":"੦","accepted":[],"feedback":{"correct":"੦ is zero.","incorrect":"Trace ੦ once more."},"response_seconds":10} -->

All four digit shapes needed by this chapter are now known.
`,$=e({default:()=>ee}),ee=`---
schema_version: 2
id: PA-W08-hora
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1810
delivery: script
chapter: 30
type: writing
headword: "ੋ"
romanization: "hora"
gloss: "trace and place the Gurmukhi long-o vowel mark"
prerequisites: [PA-W08-pairin-bindi, PA-S05-babba-lalla-hora]
sounds: []
roots: []
duration:
  max_seconds: 155
requires:
  knowledge: [PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-RECOG-HORA-01]
introduces:
  knowledge: [PA-SCRIPT-HORA-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-HORA-01, PA-SCRIPT-RECOG-HORA-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-pairin-bindi, PA-S05-babba-lalla-hora]
---

# ੋ — add the long-o mark

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-RECOG-HORA-01] -->

Write **ਫ਼** once.

## Script — one vowel mark
<!-- hl-knowledge: introduces=[PA-SCRIPT-HORA-01]; assesses=[PA-SCRIPT-PAIRIN-BINDI-01] -->

The vowel mark **ੋ**, called *hora*, sits above and to the right of its base.
Add it to the known base: **ਫ਼ + ੋ = ਫ਼ੋ**.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-HORA-01] -->
<!-- hl-writing-stage: guided-copy -->

Copy **ਫ਼ੋ** once. Check that the dot stays below and hora stays above-right.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-HORA-01] -->
<!-- hl-activity: {"id":"PA-W08-hora-check","kind":"text","assesses":["PA-SCRIPT-HORA-01"],"prompt":"Add hora to ਫ਼ and write the result.","answer":"ਫ਼ੋ","accepted":[],"feedback":{"correct":"ਫ਼ੋ places hora above-right.","incorrect":"Keep the dot below and place ੋ above-right: ਫ਼ੋ."},"response_seconds":14} -->

Stop after this one vowel mark.
`,te=e({default:()=>ne}),ne=`---
schema_version: 2
id: PA-W08-pairin-bindi
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1800
delivery: script
chapter: 30
type: writing
headword: "ਫ਼"
romanization: "fa"
gloss: "add the below-letter dot that changes pha to fa"
prerequisites: [PA-W08-pha]
sounds: []
roots: []
duration:
  max_seconds: 160
requires:
  knowledge: [PA-SCRIPT-PHA-01]
introduces:
  knowledge: [PA-SCRIPT-PAIRIN-BINDI-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-PHA-01, PA-SCRIPT-PAIRIN-BINDI-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-pha]
---

# ਫ਼ — place one dot below

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PHA-01] -->

Write **ਫ** once and leave room directly below it.

## Script — one mark
<!-- hl-knowledge: introduces=[PA-SCRIPT-PAIRIN-BINDI-01]; assesses=[PA-SCRIPT-PHA-01] -->

Add the small dot below: **ਫ + ਼ = ਫ਼**. This below-letter dot is *pairin bindi*;
here it changes *pha* to the borrowed sound *fa*. It is not the above-letter
bindi already used in **ਹਾਂ**.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PAIRIN-BINDI-01] -->
<!-- hl-writing-stage: guided-copy -->

Copy **ਫ਼** once. Point to the dot below before you stop.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PAIRIN-BINDI-01] -->
<!-- hl-activity: {"id":"PA-W08-pairin-bindi-check","kind":"text","assesses":["PA-SCRIPT-PAIRIN-BINDI-01"],"prompt":"Add the below-letter dot to ਫ and write the result.","answer":"ਫ਼","accepted":[],"feedback":{"correct":"ਫ਼ carries the borrowed f sound.","incorrect":"Keep the dot below the base: ਫ਼."},"response_seconds":14} -->

Only the base and its below-letter dot are scored here.
`,re=e({default:()=>ie}),ie=`---
schema_version: 2
id: PA-W08-pha
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1790
delivery: script
chapter: 30
type: writing
headword: "ਫ"
romanization: "pha"
gloss: "trace the Gurmukhi letter pha"
prerequisites: [PA-W07-age-no-model, PA-S04-phappha-gagga]
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: [PA-SCRIPT-PA-01, PA-SCRIPT-RECOG-PHA-01]
introduces:
  knowledge: [PA-SCRIPT-PHA-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-PA-01, PA-SCRIPT-PHA-01, PA-SCRIPT-RECOG-PHA-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W03-pa, PA-S04-phappha-gagga]
---

# ਫ — one new base shape

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PA-01, PA-SCRIPT-RECOG-PHA-01] -->

Write the familiar **ਪ** once. The new base begins similarly but is not the same
letter.

## Script — one letter
<!-- hl-knowledge: introduces=[PA-SCRIPT-PHA-01]; assesses=[PA-SCRIPT-PA-01] -->

Trace **ਫ** slowly twice. This is *pha*. Stop before adding any dot or vowel
mark.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PHA-01] -->
<!-- hl-writing-stage: guided-copy -->

Cover the model, write **ਫ** once, then compare only its shape.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PHA-01] -->
<!-- hl-activity: {"id":"PA-W08-pha-check","kind":"text","assesses":["PA-SCRIPT-PHA-01"],"prompt":"Write the new Gurmukhi base letter pha.","answer":"ਫ","accepted":[],"feedback":{"correct":"ਫ is the new base shape.","incorrect":"Trace ਫ once more, without adding a dot."},"response_seconds":12} -->

One new base is enough.
`,ae=e({default:()=>oe}),oe=`---
schema_version: 2
id: PA-W08-phone-a
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1840
delivery: script
chapter: 30
type: writing
headword: "੦੧੨੨੫੧"
romanization: "zero-one-two-two-five-one"
gloss: "fictional practice phone value A"
prerequisites: [PA-W08-digit-zero]
sounds: []
roots: []
duration:
  max_seconds: 170
requires:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01]
introduces:
  knowledge: [PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-digit-zero, PA-W07-digit-one, PA-W07-digit-two, PA-W07-digit-five]
---

# ੦੧੨੨੫੧ — build fictional value A

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Point to the digits as they are named: zero, one, two, five.

## Script — six places
<!-- hl-knowledge: introduces=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Fictional practice value A is **੦੧੨੨੫੧**. Touch and say each place from left to
right: **੦ · ੧ · ੨ · ੨ · ੫ · ੧**. This short value is only curriculum data; it
is not a real or callable number.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-writing-stage: guided-copy -->

Copy A once, pausing after the third digit. Do not add grouping yet.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-a-check","kind":"text","assesses":["PA-FORM-PHONE-A-01","PA-FORM-PHONE-DIGIT-ORDER-01"],"prompt":"Copy fictional practice phone value A digit by digit.","answer":"੦੧੨੨੫੧","accepted":[],"feedback":{"correct":"A keeps all six digits in order.","incorrect":"Touch the six places left to right and copy only the first mismatch again."},"response_seconds":24} -->

Only one six-place value is active.
`,se=e({default:()=>ce}),ce=`---
schema_version: 2
id: PA-W08-phone-b
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1850
delivery: script
chapter: 30
type: writing
headword: "੦੨੫੧੨੫"
romanization: "zero-two-five-one-two-five"
gloss: "fictional practice phone value B"
prerequisites: [PA-W08-phone-a]
sounds: []
roots: []
duration:
  max_seconds: 170
requires:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-DIGIT-ORDER-01]
introduces:
  knowledge: [PA-FORM-PHONE-B-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-a]
---

# ੦੨੫੧੨੫ — build fictional value B

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->

Touch value A, **੦੧੨੨੫੧**, from left to right once.

## Script — a second six-place value
<!-- hl-knowledge: introduces=[PA-FORM-PHONE-B-01]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->

Fictional practice value B is **੦੨੫੧੨੫**. Touch each place: **੦ · ੨ · ੫ · ੧ ·
੨ · ੫**. It uses only the four introduced digit shapes.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-writing-stage: guided-copy -->

Copy B once, pausing after the third digit. Do not add grouping yet.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-b-check","kind":"text","assesses":["PA-FORM-PHONE-B-01","PA-FORM-PHONE-DIGIT-ORDER-01"],"prompt":"Copy fictional practice phone value B digit by digit.","answer":"੦੨੫੧੨੫","accepted":[],"feedback":{"correct":"B keeps all six digits in order.","incorrect":"Touch the six places left to right and copy only the first mismatch again."},"response_seconds":24} -->

The bounded bank now contains exactly A and B.
`,le=e({default:()=>ue}),ue=`---
schema_version: 2
id: PA-W08-phone-delayed
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1900
delivery: script
chapter: 30
type: writing
headword: "ਫ਼ੋਨ: ੦੨੫ ੧੨੫"
romanization: "delayed phone-field entry"
gloss: "hide the bank and write one requested fictional phone value after a delay"
prerequisites: [PA-W08-phone-grouping]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-select, PA-W08-phone-grouping]
---

# Hide the bank, then write B

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-GROUPING-01] -->

Study **ਫ਼ੋਨ: ੦੨੫ ੧੨੫** for five seconds. Say each digit while pointing.

## Writing — short delay
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-writing-stage: delayed-copy -->

Hide the bank. Wait five seconds. For **ਖ**, complete one blank **ਫ਼ੋਨ** field
from memory.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->

Uncover the model only after writing. Compare digit selection, order, and the
single group space separately.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-delayed-b","kind":"text","assesses":["PA-FORM-LABEL-PHONE-01","PA-FORM-PHONE-B-01","PA-FORM-PHONE-CUE-MAP-01","PA-FORM-PHONE-DIGIT-ORDER-01","PA-FORM-PHONE-GROUPING-01"],"prompt":"After hiding the bank for five seconds, complete the fictional phone field for ਖ.","answer":"ਫ਼ੋਨ: ੦੨੫ ੧੨੫","accepted":["੦੨੫ ੧੨੫","ਫ਼ੋਨ:੦੨੫ ੧੨੫"],"feedback":{"correct":"B was recovered after a short delay.","incorrect":"Finish the attempt, uncover B, and repair only the first mismatch."},"response_seconds":32} -->

The model is hidden during the attempt.
`,de=e({default:()=>fe}),fe=`---
schema_version: 2
id: PA-W08-phone-dictation
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1910
delivery: script
chapter: 30
type: writing
headword: "੦੧੨ ੨੫੧"
romanization: "heard digit string"
gloss: "transcribe one heard fictional phone value into grouped Gurmukhi digits"
prerequisites: [PA-W08-phone-delayed]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01]
skills: [listening, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-digit-recognition, PA-W08-phone-grouping]
---

# Hear six digits and write them

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01] -->

Without a number model, write the four introduced digits when you hear: zero,
one, two, five.

## Writing — digit by digit
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-writing-stage: dictation-transcription -->

Listen twice: **zero — one — two | two — five — one**. The pause marks the one
group space. Write Gurmukhi digits; do not write English numerals or words.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->

Read your six digits back left to right. Check the space only after checking all
six positions.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-DIGIT-ZERO-01, PA-SCRIPT-DIGIT-ONE-01, PA-SCRIPT-DIGIT-TWO-01, PA-SCRIPT-DIGIT-FIVE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-dictation-a","kind":"text","assesses":["PA-SCRIPT-DIGIT-ZERO-01","PA-SCRIPT-DIGIT-ONE-01","PA-SCRIPT-DIGIT-TWO-01","PA-SCRIPT-DIGIT-FIVE-01","PA-FORM-PHONE-A-01","PA-FORM-PHONE-DIGIT-ORDER-01","PA-FORM-PHONE-GROUPING-01"],"prompt":"Write in Gurmukhi digits: zero, one, two; pause; two, five, one.","answer":"੦੧੨ ੨੫੧","accepted":[],"feedback":{"correct":"The heard sequence became six Gurmukhi digits with one group space.","incorrect":"Replay one digit at a time and repair only the first mismatch."},"response_seconds":36} -->

Romanization and Latin digits do not earn the Gurmukhi writing score.
`,pe=e({default:()=>me}),me=`---
schema_version: 2
id: PA-W08-phone-grouping
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1890
delivery: script
chapter: 30
type: writing
headword: "੦੧੨ ੨੫੧"
romanization: "three digits, space, three digits"
gloss: "group a six-digit fictional phone value as three plus three"
prerequisites: [PA-W08-phone-supported]
sounds: []
roots: []
duration:
  max_seconds: 165
requires:
  knowledge: [PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01]
introduces:
  knowledge: [PA-FORM-PHONE-GROUPING-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-a, PA-W08-phone-b, PA-W08-phone-supported]
---

# Three digits, one space, three digits

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->

Touch A in six places: **੦ · ੧ · ੨ · ੨ · ੫ · ੧**.

## Script — one grouping boundary
<!-- hl-knowledge: introduces=[PA-FORM-PHONE-GROUPING-01]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->

Insert one clear space after the third digit: **੦੧੨ ੨੫੧**. The space helps the
eye track the practice value; it does not remove or reorder a digit. B groups as
**੦੨੫ ੧੨੫**.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-writing-stage: guided-copy -->

Copy grouped B once. Count three digits, make one space, then count three more.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-grouping-a","kind":"text","assesses":["PA-FORM-PHONE-A-01","PA-FORM-PHONE-DIGIT-ORDER-01","PA-FORM-PHONE-GROUPING-01"],"prompt":"Rewrite fictional value A with its three-plus-three grouping.","answer":"੦੧੨ ੨੫੧","accepted":[],"feedback":{"correct":"A keeps six digits with one middle space.","incorrect":"Keep the order and place one space after digit three."},"response_seconds":24} -->

This book keeps one grouping inside the bounded practice bank; it is
not a claim about every real-world phone format.
`,he=e({default:()=>ge}),ge=`---
schema_version: 2
id: PA-W08-phone-label
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1820
delivery: script
chapter: 30
type: writing
headword: "ਫ਼ੋਨ"
romanization: "fon"
gloss: "the form label phone"
prerequisites: [PA-W08-hora, PA-S03-nanna-bihari-dulava]
sounds: []
roots: []
duration:
  max_seconds: 170
requires:
  knowledge: [PA-SCRIPT-PHA-01, PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-HORA-01, PA-SCRIPT-NA-01, PA-SCRIPT-RECOG-NANNA-01]
introduces:
  knowledge: [PA-FORM-LABEL-PHONE-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-SCRIPT-PHA-01, PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-HORA-01, PA-SCRIPT-NA-01, PA-FORM-LABEL-PHONE-01, PA-SCRIPT-RECOG-NANNA-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-pha, PA-W08-pairin-bindi, PA-W08-hora, PA-W01-na, PA-S03-nanna-bihari-dulava]
---

# ਫ਼ੋਨ — assemble the phone label

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-SCRIPT-PHA-01, PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-HORA-01, PA-SCRIPT-NA-01] -->

Touch the pieces in order: **ਫ**, the dot below, **ੋ**, **ਨ**.

## Script — one label
<!-- hl-knowledge: introduces=[PA-FORM-LABEL-PHONE-01]; assesses=[PA-SCRIPT-PHA-01, PA-SCRIPT-PAIRIN-BINDI-01, PA-SCRIPT-HORA-01, PA-SCRIPT-NA-01] -->

Join the pieces as **ਫ਼ੋਨ**. On this fictional beginner form, **ਫ਼ੋਨ** labels the
phone-number line. Copy the label once and leave its value blank.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01] -->

Point to **ਫ਼ੋਨ** and say what information belongs there. Do not add digits.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-SCRIPT-RECOG-NANNA-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-label-check","kind":"text","assesses":["PA-FORM-LABEL-PHONE-01"],"prompt":"A fictional practice form shows ਫ਼ੋਨ. What belongs on that line?","answer":"phone number","accepted":["telephone number","phone"],"feedback":{"correct":"ਫ਼ੋਨ labels the phone-number field.","incorrect":"ਫ਼ੋਨ labels a phone number on this practice form."},"response_seconds":12} -->

No real contact information is requested.
`,_e=e({default:()=>ve}),ve=`---
schema_version: 2
id: PA-W08-phone-no-model
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1930
delivery: script
chapter: 30
type: writing
headword: "ਫ਼ੋਨ: __________"
romanization: "phone field"
gloss: "choose and write one known fictional phone value from a nonverbal cue with no model"
prerequisites: [PA-W08-phone-repair]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01, PA-FORM-PHONE-REPAIR-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01, PA-FORM-PHONE-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-label, PA-W08-phone-select, PA-W08-phone-delayed, PA-W08-phone-repair]
---

# One no-model phone field

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-CUE-MAP-01] -->

Close the previous lessons. There is no value bank, support-language label,
Latin-digit version, or copyable Gurmukhi answer below.

## Writing — independent controlled choice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-writing-stage: controlled-composition -->

Complete the requested fictional field:

> ਖ — **ਫ਼ੋਨ: __________**

Choose the known value attached to the circle and write it from memory in
Gurmukhi digits. Do not reopen an earlier lesson until the attempt is complete.

## Guided Practice — separate repair passes
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-REPAIR-01] -->
<!-- hl-writing-stage: controlled-composition -->

After writing, compare cue selection, six digit positions, grouping, and field
placement. Repair only the first differing dimension.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-no-model-b","kind":"text","assesses":["PA-FORM-PHONE-B-01","PA-FORM-PHONE-DIGIT-ORDER-01","PA-FORM-PHONE-GROUPING-01"],"prompt":"With no bank or copyable answer, complete the fictional phone field for ਖ.","answer":"੦੨੫ ੧੨੫","accepted":["ਫ਼ੋਨ: ੦੨੫ ੧੨੫","ਫ਼ੋਨ:੦੨੫ ੧੨੫"],"feedback":{"correct":"The selector independently selected the grouped Gurmukhi value B.","incorrect":"Finish the attempt first; then reopen the bank and repair one dimension."},"response_seconds":40} -->

This closes only the untimed phone-field ladder. Date, integration, and A1
readiness remain separate dependency-linked work.
`,ye=e({default:()=>be}),be=`---
schema_version: 2
id: PA-W08-phone-repair
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1920
delivery: script
chapter: 30
type: writing
headword: "select · digits · group · place"
romanization: "four repair passes"
gloss: "repair one phone-field dimension at a time"
prerequisites: [PA-W08-phone-dictation]
sounds: []
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01]
introduces:
  knowledge: [PA-FORM-PHONE-REPAIR-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01, PA-FORM-PHONE-REPAIR-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-select, PA-W08-phone-grouping, PA-W08-phone-dictation]
---

# Repair only one dimension

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->

Name four separate checks: cue selection, six digit positions, middle group
space, and placement on the **ਫ਼ੋਨ** line.

## Guided Practice — bounded repair
<!-- hl-knowledge: introduces=[PA-FORM-PHONE-REPAIR-01]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01, PA-FORM-PHONE-GROUPING-01] -->

For **ਕ**, inspect **ਫ਼ੋਨ: ੦੧੨੨ ੫੧**. The selected value and digit order are
right; only the group boundary is wrong. Repair it as **੦੧੨ ੨੫੧** without
rewriting correct digits.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-REPAIR-01] -->
<!-- hl-writing-stage: controlled-composition -->

Run the checks in order. Stop after the first differing dimension is repaired.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-GROUPING-01, PA-FORM-PHONE-REPAIR-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-repair-group","kind":"text","assesses":["PA-FORM-PHONE-A-01","PA-FORM-PHONE-GROUPING-01","PA-FORM-PHONE-REPAIR-01"],"prompt":"For ਕ, repair only the grouping in ੦੧੨੨ ੫੧.","answer":"੦੧੨ ੨੫੧","accepted":[],"feedback":{"correct":"Only the group boundary changed.","incorrect":"Keep all six digits and move the space after digit three."},"response_seconds":24} -->

Do not replace the whole answer when one dimension differs.
`,xe=e({default:()=>Se}),Se=`---
schema_version: 2
id: PA-W08-phone-select
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1870
delivery: script
chapter: 30
type: writing
headword: "ਕ / ਖ"
romanization: "two target-script selectors"
gloss: "select one known fictional phone value before writing digits"
prerequisites: [PA-W08-digit-recognition]
sounds: []
roots: []
duration:
  max_seconds: 160
requires:
  knowledge: [PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-DIGIT-ORDER-01]
introduces:
  knowledge: [PA-FORM-PHONE-CUE-MAP-01]
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-a, PA-W08-phone-b]
---

# Choose A or B before writing

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01] -->

Read the closed bank once: A **੦੧੨੨੫੧**; B **੦੨੫੧੨੫**.

## You'll want to know — two fixed selectors
<!-- hl-knowledge: introduces=[PA-FORM-PHONE-CUE-MAP-01]; assesses=[PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01] -->

For this fictional practice only, **ਕ** means A and **ਖ** means B. Point to the
matching value before your pen moves. The selector is not part of the phone
field.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-CUE-MAP-01] -->

For **ਖ**, select B. For **ਕ**, select A. Selection and digit writing are
separate checks.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-B-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-select-kha","kind":"text","assesses":["PA-FORM-PHONE-CUE-MAP-01","PA-FORM-PHONE-B-01"],"prompt":"The fictional selector is ਖ. Select A or B, then give its known value.","answer":"B — ੦੨੫੧੨੫","accepted":["B","੦੨੫੧੨੫"],"feedback":{"correct":"ਖ selects B in this bounded bank.","incorrect":"Return to the fixed selector map: ਖ means B."},"response_seconds":18} -->

The selectors carry no special meaning outside this exercise.
`,Ce=e({default:()=>we}),we=`---
schema_version: 2
id: PA-W08-phone-supported
spine_node: SPINE-EXCHANGE-NAMES
sequence: 1880
delivery: script
chapter: 30
type: writing
headword: "ਫ਼ੋਨ: __________"
romanization: "phone field"
gloss: "fill one fictional phone field with the bounded bank visible"
prerequisites: [PA-W08-phone-select]
sounds: []
roots: []
duration:
  max_seconds: 175
requires:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-B-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01]
introduces:
  knowledge: []
introduces_idioms: []
introduces_senses: []
introduces_culture_claims: []
practises:
  knowledge: [PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01]
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-punjabi
reviews_of: [PA-W08-phone-label, PA-W08-phone-select]
---

# One supported phone field

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-CUE-MAP-01] -->

Point to **ਫ਼ੋਨ**. Read the visible bank: A **੦੧੨੨੫੧**; B **੦੨੫੧੨੫**.

## Writing — bank visible
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-writing-stage: guided-copy -->

The selector is **ਕ**. Complete **ਫ਼ੋਨ: __________** with A while the bank remains
visible. Write one digit per place.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-PHONE-DIGIT-ORDER-01] -->

Point under each copied digit and compare all six places left to right.

## Wrap-up recall
<!-- hl-knowledge: introduces=[]; assesses=[PA-FORM-LABEL-PHONE-01, PA-FORM-PHONE-A-01, PA-FORM-PHONE-CUE-MAP-01, PA-FORM-PHONE-DIGIT-ORDER-01] -->
<!-- hl-activity: {"id":"PA-W08-phone-supported-a","kind":"text","assesses":["PA-FORM-LABEL-PHONE-01","PA-FORM-PHONE-A-01","PA-FORM-PHONE-CUE-MAP-01","PA-FORM-PHONE-DIGIT-ORDER-01"],"prompt":"With the bank visible, complete the fictional phone field for ਕ.","answer":"ਫ਼ੋਨ: ੦੧੨੨੫੧","accepted":["੦੧੨੨੫੧","ਫ਼ੋਨ:੦੧੨੨੫੧"],"feedback":{"correct":"The supported row uses A in six-place order.","incorrect":"Select A first, then compare one digit at a time."},"response_seconds":30} -->

This supported copy does not earn independent-writing credit.
`;export{y as A,N as C,T as D,D as E,l as F,s as I,a as L,h as M,p as N,C as O,d as P,r as R,F as S,k as T,G as _,he as a,z as b,le as c,re as d,te as f,q as g,Y as h,_e as i,_ as j,x as k,se as l,Z as m,xe as n,pe as o,$ as p,ye as r,de as s,Ce as t,ae as u,U as v,j as w,L as x,V as y,t as z};