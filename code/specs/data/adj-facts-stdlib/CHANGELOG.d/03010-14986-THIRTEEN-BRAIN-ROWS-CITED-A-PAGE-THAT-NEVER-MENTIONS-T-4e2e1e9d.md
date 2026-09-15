- **#14986: thirteen brain rows cited a page that never mentions them.**
  `anatomy/brain-parts.adj` carried the CEREBRUM sentence as its `source` — the field that carries the
  tier — for **all fifteen rows**, on a SEER page which, measured, contains **no brain-stem function
  at all** (its only brain stem sentence is anatomical), **no thalamus sentence** (the string occurs
  there only inside "hypothalamus"), and the word **"hippocampus" zero times**.

  The evidence for those thirteen rows was already in the file, one line below, as five untiered
  `cites` on two StatPearls chapters. Each row now carries its own span **and its own locator** —
  three pages — and the envelope carries a framing sentence naming three parts and no function, so it
  warrants none of the fifteen.

  ### The test asserted the wrong page onto thirteen rows, and passed

  `out.contains("training.seer.cancer.gov")` sat over a query set including the brainstem and the
  hippocampus. It passed because the envelope's locator covered every row — which is exactly the
  defect. It now pins each answer's own page, and asserts that these four answers cite SEER **not at
  all**.

  ### Pins

  **21 of 21 mutants killed, one control.** All fifteen rows broken one at a time — **ten** of them
  share the brainstem sentence — plus warranting a brainstem row with the cerebrum sentence on the SEER
  page, repointing the hippocampus row at SEER, rebinding a function, dropping a row's locator, and
  fabricating the envelope.

  **One mutant survived first, and it was an equivalent mutant — for thirteen rows, not one.** Dropping
  a row's own locator changes nothing for any row whose locator already equals the envelope's, and
  **13 of the 15 do**: cerebellum, all ten brainstem rows, hypothalamus and thalamus all sit on
  *Physiology, Brain*, which is also the envelope's page. The output is byte-identical, so this is not
  a coverage gap. It bites only on `cerebrum` (SEER) and `hippocampus` (the hippocampus chapter);
  re-aimed there, both die. A draft of this paragraph attributed the survival to the hypothalamus row
  alone and called the split "one of three versus the other two" — measured, it is 13 versus 2.

  ### The envelope's LOCATOR was unpinned, and only its source was

  Review measured it: the envelope-pinning needle stopped at `
    locator`, pinning that a locator
  follows but never its value. Repointing the envelope locator at either other page **survived** — and
  no other assertion could catch it, because every row overrides the envelope, so its locator reaches
  no answer. Both of those now die. The sibling entries that "closed the envelope gap" (#15176 onward)
  closed half of it.

  ### The count was wrong in six places, and the file already said so

  A draft wrote **nine** brainstem rows. There are **ten** — `brain-parts.adj:44`, fifty-two lines
  above the paragraph that got it wrong, already said "always listed TEN autonomic functions". The
  error reached the header, a row comment, two Rust doc comments, a test function's name, and this
  entry. Review counted the rows.

  ### Two guards fired on me while writing this

  - The converter hardcoded a span-to-locator map and paired the cerebellum and brainstem sentences
    with the **Hippocampus** chapter; the file pairs them with *Physiology, Brain*. Rewritten to read
    the pairings out of the file. Same retyping failure as the drifted test span in #15176, caught
    before it ran.
  - Pairing spans to rows by "the span that names the part" was **ambiguous**: two spans name the
    cerebrum, because the thalamus sentence ends *"…relays this information to the cerebrum."* Pairing
    by earliest mention fixed that — and then over-fired, rejecting the thalamus span because its
    grammatical subject is *"Sensory neurons"*. The span states the relation, which is what a warrant
    must do; the rule was wrong, not the sentence. It is recorded in the header.

