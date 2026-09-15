- `physics/simple-machine-alt-example.adj` (new) — a sibling to the already-shipped
  `simple-machines.adj` (`simple_machine_example(machine, example)`, ONE everyday example per
  simple machine): a new `simple_machine_alt_example(machine, alt_example)` table names the SECOND
  everyday example the SAME already-cited NASA educator-notes page lists for a machine, decoded
  from spans already sitting unused inside `simple-machines.adj`'s own header and provenance
  block — no new WebFetch. Five rows: lever → scissors, inclined_plane → stairs, wedge → knife,
  wheel_and_axle → clock, pulley → water_well. Honest abstention on `screw` (its first-listed
  example is literally the word "screw", not a distinct object, and its second, "bottle caps", is
  already `simple-machines.adj`'s primary row — no unused second example remains). New e2e test
  file `facts_simplemachinealtexample_e2e.rs` (3 tests: forward recall of all five with citation,
  backward recall from a bound alternate example, honest abstention on screw). No manifest
  objective, matching `simple-machines.adj`'s own precedent of not having one. Second slice from
  the physics/ sweep tranche.
