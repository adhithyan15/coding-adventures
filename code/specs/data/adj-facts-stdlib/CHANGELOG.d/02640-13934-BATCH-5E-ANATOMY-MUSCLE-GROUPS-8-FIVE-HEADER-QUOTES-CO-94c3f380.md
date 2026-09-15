- **#13934 batch 5e: `anatomy/muscle-groups` +8, five header quotes corrected, and a PROPAGATED
  copy of one of them repaired in a different library.**

  *** THE PROPAGATION REPAIR IS THE POINT, NOT AN EXTRA. *** `facts_musclebodyaspect_e2e.rs` records
  that `muscle-body-aspect` was built from "clauses already sitting unused inside
  `muscle-groups.adj`'s own already-quoted Wikipedia source sentences". So muscle-groups' **damaged**
  rectus_abdominis header quote was copied into `muscle-body-aspect`'s `source` — where a comment
  defect became **a shipped citation the engine returns**.

  Fixing only the header would leave the derived data broken. That is exactly what happened with
  `rectangle`: its header was corrected in batch 5a (#14066, merged) and
  `geometry/quadrilateral-secondary-property:62` is *still* shipping the dropped-subject form on
  `main` today. Repeating that knowingly would be worse than having done it once by accident, so
  `muscle-body-aspect:84` is repaired here with the same verified span. (The
  `quadrilateral-secondary-property` copy belongs to a #14070 installment — different library,
  different page.)

  The convergence made it cheap: `muscle-body-aspect`'s three rows are `rectus_abdominis → ventral`,
  `sartorius → anterior`, `gastrocnemius → posterior` — precisely the three muscles whose
  muscle-groups quotes were being corrected, and all three corrected spans name their subject *and*
  contain their row's value.

  THE FIVE DEFECTIVE HEADER QUOTES, one per subtype:

  | row | defect |
  |---|---|
  | `deltoid` | dropped a footnote marker — "is the muscle forming" for "is the muscle**[1]** forming" |
  | `rectus_abdominis` | elided the alias list with "…" (third `\"` escape case in the stdlib) |
  | `quadriceps` | **deleted a parenthetical with no ellipsis at all** |
  | `sartorius` | verbatim, but began with a bare "It is", naming no subject |
  | `gastrocnemius` | same |

  `quadriceps` is the worst subtype found: the IPA-and-alias block simply vanished, so the quote
  **reads as faithful and is not**. No text-only screen can see it — which is precisely how
  `rectangle`'s dropped subject propagated unnoticed into another library's shipped `source`.

  The last two were not wrong, only weaker than what the page offers; both pages carry a contiguous
  block naming the muscle *and* its region. Declining to settle for the first acceptable span
  produced better evidence for the third time, after `tape` and `spider`.
  `triceps_brachii`, `pectoralis_major` and `gluteus_maximus` verified CLEAN and are unchanged.

  **The propagated repair was unprotected until this change.** `muscle-body-aspect`'s existing
  assertions are `contains("en.wikipedia.org")` and `contains("\"trust\":\"consensus\"")` — two bare
  scans that pass equally well against the damaged form, which is why its three tests passed both
  before *and* after the repair. A pin now binds the answer to the actual sentence.

  A CLARIFICATION THE MUTATION CHECK FORCED: **per-row pins are NESTED PREFIXES, not independent.**
  A contiguous pin running to corroboration index 5 necessarily *contains* indices 0–4, so damaging
  an earlier row reddens every later pin. My first expectation assumed row-isolation and was wrong.
  Directionality is proved the other way instead — damaging a *later* row leaves earlier pins green
  (M2, M4) — and that is what shows a pin is bound to its own row rather than to the head of the
  list.

  Five directional mutations pass, including two that restore the old defective forms outright
  (deltoid's dropped marker, quadriceps' unmarked deletion) and one that restores the *inherited*
  elided source in `muscle-body-aspect`.

  Both `.query.adj` companions parse, run, and abstain correctly. 533 test binaries / 1612 tests
  green, clippy `-D warnings` clean.

