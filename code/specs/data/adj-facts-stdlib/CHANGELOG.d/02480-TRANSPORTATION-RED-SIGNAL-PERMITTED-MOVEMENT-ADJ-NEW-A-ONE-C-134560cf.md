- `transportation/red-signal-permitted-movement.adj` (new) — a one-column `table` recording which
  movements a steady CIRCULAR RED signal permits: `red_signal_permitted_movement(movement)`, two rows
  from one sentence of MUTCD Section 4D.04. `turn_right_after_stopping` and
  `turn_left_from_one_way_street_into_one_way_street_after_stopping`. Fifth transportation library,
  and the exact structural sibling of `green-signal-permitted-movement.adj` — same relation shape,
  same source, same locator.

  READ IT NEXT TO THE GREEN SIBLING; THAT CONTRAST IS THE POINT. Green permits `turn_right`
  OUTRIGHT. Red permits it only AFTER STOPPING. Because the qualifier is part of the value, the two
  relations share NO movement atom at all, and a bare `turn_right` is absent from the red set
  entirely. A driver who reads the two tables as saying the same thing about right turns has made
  exactly the mistake this pair exists to prevent.

  BE PRECISE ABOUT WHAT THE CONTRAST IS: the difference is the STOPPING requirement and nothing else.
  An earlier draft also claimed red applies "only where no device prohibits it", as though
  device-prohibition were a red-specific narrowing. It is not — the manual's green sentence carries a
  parallel exception in the same paragraph, continuing "except as such movement is modified by
  lane-use signs, turn prohibition signs, lane markings, roadway design, separate turn signal
  indications, or other traffic control devices." Both permissions are device-modifiable. Noted
  separately: that clause is absent from the shipped `green-signal-permitted-movement.adj` citation,
  which is a verbatim PREFIX stopping just before its own exception — a truncation of exactly the kind
  this entry condemns two paragraphs down. FILED AS ISSUE #13916 rather than fixed here, since
  editing a sibling library's provenance belongs in its own change.

  THE QUALIFIER SPLIT NEEDS BOTH HALVES OF THE SETTLED PLACEMENT RULE AT ONCE. "After stopping"
  modifies EACH MOVEMENT individually, so it rides INSIDE the atom, the same rule `veto-override.adj`
  and `karst-process-zone.adj` apply — right-on-red is never permitted WITHOUT stopping, so an
  unqualified `turn_right` is a claim this source does not make. "Except when a traffic control device
  is in place prohibiting a turn on red or a steady RED ARROW signal indication is displayed" modifies
  the WHOLE PERMISSION rather than any one movement, so it stays in the CITATION, which is quoted in
  full including that clause — the same treatment `sign-shape.adj` gave the warning-sign default, and
  for the same reason: a citation has to carry the qualifier it is being used to justify. Quoting from
  "vehicular traffic facing a steady CIRCULAR RED" would have been shorter and would have silently
  dropped the exception from the evidence.

  Honest abstention on `straight_through` and `u_turn`, which are not permitted on red at all, and on
  an unqualified `turn_left` — the permission is narrower than "turning left", being only from a
  one-way street INTO a one-way street, and flattening the atom would assert a permission a driver at
  an ordinary two-way intersection does not have. What a driver MUST DO at a red signal (stop at the
  stop line, or before the crosswalk, or before the intersection) is a different relation stated in a
  different paragraph and belongs in its own table. The RED ARROW indication has its own provisions
  and gets no rows in a CIRCULAR RED table.

  A LANGUAGE-SURFACE FACT WORTH RECORDING, since it shapes how one-column tables must be queried and
  tested: A FULLY-GROUND QUERY IS NOT A RECALL QUERY. `? red_signal_permitted_movement(turn_right)`
  with no variable is routed to the hypothesis/ranking path and produces NO `recall` entry at all, so
  it can neither answer nor abstain. On a one-column table, membership can only be asked by binding a
  variable and reading the returned set — which is what the shipped green sibling's companion already
  does, and what this library's companion and tests do throughout.

  New `red-signal-permitted-movement.query.adj` (which imports BOTH tables so the contrast is visible
  in one run) and `facts_redsignalpermittedmovement_e2e.rs` (4 tests: both rows pinned with the
  citation's exception clause, the four unqualified movements all absent, the two signals sharing no
  movement atom with green's bare `turn_right` as the positive control, and the two relations staying
  separate when both are imported). EVERY ROW is mutation-tested and not merely every assertion —
  deleting the right-turn row fails the suite, baring either atom fires the corresponding negative,
  and adding `straight_through` fires the leak check.

  A CARDINALITY ASSERTION WAS ADDED AFTER REVIEW, and it closes the mirror image of the row-coverage
  lesson. Pinning every row proves nothing is MISSING; it cannot prove nothing was INVENTED. Review
  showed that adding a third row — `turn_right_on_red_arrow`, `reverse`, or even a duplicate of an
  existing row — survived the ENTIRE suite, because no assertion counted the answers. That is
  precisely the failure mode a fabricating generator produces, which makes it the one a
  provenance-first stdlib can least afford to leave unguarded. The test now asserts the answer count
  is exactly two. New manifest objective `adj.socialstudies.k2.red_signal_permitted_movement`.

