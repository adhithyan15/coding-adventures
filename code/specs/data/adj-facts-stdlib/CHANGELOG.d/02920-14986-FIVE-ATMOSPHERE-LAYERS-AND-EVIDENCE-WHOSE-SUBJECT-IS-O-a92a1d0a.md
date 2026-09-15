- **#14986: five atmosphere layers, and evidence whose subject is only "here".**
  `earth-science/atmosphere-layers.adj` had one envelope, the TROPOSPHERE row's weather sentence, so
  a recall of the exosphere was warranted by a sentence about clouds. Each row now states its own.

  ### Widened, not paired

  The NASA article gives each layer its own paragraph, names the layer in the opening sentence, then
  refers back to it. The sentence carrying the feature names its subject only as a pronoun for three
  of the five rows:

  | row | the sentence that carries the feature |
  | --- | --- |
  | `troposphere → weather` | *"Most of Earth's weather happens **here**…"* |
  | `mesosphere → meteors` | *"Most meteors burn up in **this atmospheric layer**."* |
  | `thermosphere → auroras` | *"The aurora borealis and aurora australis are sometimes seen **here**."* |

  Quoted alone, none grounds its row — "here" is not a layer.

  **This repo already had the answer, and my first draft did not use it.** A drafted shape gave those
  rows a `source` naming the layer plus a `cites` carrying the feature. Security review found that
  this contradicts a standard already shipped in [`README.md`](README.md) — *"A citation must name
  its own subject"*, whose remedy is explicit: **widen the quote until it is self-contained.**
  (That standard landed under #13992, as the root-cause fix for #13934;
  `earth-science/speleothem-substrate.adj` widens for the same reason — its second sentence opens
  "They typically grow…", a bare pronoun.) Worse, the
  paired shape put the *tier* on the naming sentence while the sentence that actually supports the
  row became an untiered corroboration — the row's warrant would have been a sentence about altitude
  that never mentions the feature.

  So the spans are **widened minimally**: each of the three begins at the last sentence naming its
  layer before the feature sentence — not at the paragraph opener. (`stratosphere` and `exosphere`
  are widened by zero; their own sentence already names layer and feature.) 346 / 227 / 269 / 358 /
  194 characters, all inside the 708-character precedent this stdlib already set in
  `biology/blood-groups.adj` — recorded as 702 in the entry below, which undercounted by six. No row needs a `cites`; the file adds
  none. Nothing here is a new shape, and the `read`/`reasoned` vocabulary is unchanged.

  ### A second false alarm from the bounding-language audit

  `exosphere → highest` was flagged as a superlative with no bounding language in its provenance. The
  page says *"the exosphere is the highest layer of Earth's atmosphere"*. That is the **second** such
  false alarm, after `rainforest-layer` — in both the claim was fine and the evidence was merely
  outside the machine-readable envelope, which is what RS-5e moves. Two of that audit's flags have
  now resolved this way, which is worth weighing before treating its remaining flags as defects.

  ### Measured

  All six spans (five row, one framing) were re-checked against the fetched page after the last
  edit, read out of the shipped `.adj` rather than from a copy: each occurs **exactly once** in the
  rendered text, and each row span contains its own layer's name. Controls: a nonsense span scores
  zero; a near-miss of each span — final word altered, asserted to differ from the span itself —
  scores zero. (An earlier control in this cascade "passed" by replacing a word its subject did not
  contain, so the assertion that control ≠ subject is now part of the harness.)

  ### Pins

  **6 of 6 mutants killed. Two controls, counted separately.** Un-widening the troposphere span back
  to the anaphoric weather sentence, rebinding `mesosphere` to another feature, dropping the
  superlative clause from the exosphere span, deleting the stratosphere row's source so it falls back
  to the envelope, repointing the shared locator, and adding a spurious corroboration all redden.
  Controls: the unmutated file is green, and —

  **Disclosed gaps.** Two things these tests do *not* catch:

  - **The envelope's wording is unreachable.** Once every row overrides `source`, no output test can
    see it: replacing it with a fabricated sentence leaves the suite green, and that mutant is run to
    demonstrate it rather than left unmentioned. Only its *non-leakage* into an answer is pinned.
    Same unreachability as `si-base-units` (#15073) and `planets` (#15127).
  - **A `trust` pin cannot prove inheritance.** The rows themselves do not default: a row block goes
    through `row_provenance` (`lower.rs:2496`), which clones the envelope and overrides only the
    fields the row writes. The slack is one level up — `lower.rs:2622`, in `annotations_to_provenance`,
    defaults the **envelope's** tier to `Authoritative` whenever a `source` is present, so deleting
    the envelope's `trust authoritative` line leaves every row still reading `authoritative` and the
    suite green. (Checked both directions: setting the envelope to `trust consensus` does propagate
    to the rows, so inheritance is real — it just isn't what the pin proves.)

  **Three of my own errors, all repeats:**

  - The **two-span draft** above reinvented a rule the repo had already written down. Check
    `README.md` for an existing standard before shipping a shape that competes with one.
  - A mutant reported as surviving was a **harness bug**: a bare phrase replace hit a header comment
    quoting the same sentence and left the row untouched. Identical to the `rainforest-layer` harness
    bug in the entry below. Anchors are now row lines.
  - Two test assertions were wrong because **provenance is emitted twice per answer** (`citations`
    and `steps`) and because a **corroboration entry also carries a `source` key** — so a bare
    `"source":"<text>` needle matched the corroboration and inverted a negative assertion. An earlier
    draft of this entry credited both facts to the si-base-units entry below; that entry says nothing
    about either. These two lines are where they are written down.

  ### Also in this file

  The header used to re-quote all five spans a second time under the words "quoted here word-for-word".
  **Two of the five occur nowhere on the page**, measured: the stratosphere restatement re-opened a
  mid-sentence clause as a standalone sentence (*"The stratosphere is perhaps best known…"*, where the
  page reads *"…above Earth's surface, the stratosphere is perhaps best known…"*), and the exosphere
  restatement ended at a full stop the page does not have. Neither is a punctuation difference —
  swapping straight apostrophes for the page's curly ones still scores zero for both. The restatements
  are deleted rather than repaired: a second copy of a quote is a second thing to keep true, and the
  rows are the copy that ships. The header also reasoned from *"An ADJ `table` carries ONE provenance
  envelope"* to "so only the cleanest span can ship, and the rest live in a comment". The premise is
  still in `ADJ-TABLES.md` §4 and still true; RS-5e falsified the inference, and that sentence is the
  form the exemplar-span defect takes when written down.

  The `thermosphere → auroras` row now records that the page places auroras in the exosphere's lowest
  part as well; this table carries one feature per layer and does not claim the aurora is unique to
  the thermosphere.

