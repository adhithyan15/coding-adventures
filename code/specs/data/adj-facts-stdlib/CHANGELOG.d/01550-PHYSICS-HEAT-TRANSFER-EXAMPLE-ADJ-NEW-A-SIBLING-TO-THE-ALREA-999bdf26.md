- `physics/heat-transfer-example.adj` (new) — a sibling to the already-shipped `heat-transfer.adj`
  (`heat_transfer_mode(mode, mechanism)`, HOW / through-what each mode moves heat — conduction →
  direct_contact, convection → motion_of_gasses_and_liquids, radiation → light_waves). That table's
  own header already quotes, verbatim, the NASA sentence that states each mechanism, and the SAME
  sentence closes with a parenthetical `(e.g., ...)` everyday example: "Conduction - ... (e.g.,
  holding a cup of hot chocolate, walking bare foot across a cold floor)", "Convection - ... (e.g.,
  when warm air rises in your home)", "Radiation - ... (e.g., sunlight on your skin, heating food in
  a microwave)". New `heat_transfer_example(mode, example)` table: conduction → hot_chocolate_cup,
  convection → warm_air_rising, radiation → sunlight_on_skin. Covers the full three-mode domain with
  NO abstention among the three (conduction and radiation each have a second example — cold floor,
  microwave food — sitting unused for a possible future alt-example sibling; convection lists only
  the one). New e2e test file `facts_heattransferexample_e2e.rs` (3 tests: forward recall with
  citation, backward recall of the convection mode, honest abstention on evaporation). No manifest
  objective, matching `heat-transfer.adj`'s own precedent. Second slice from the fresh physics/
  sweep, queued after `simple-machine-function.adj`.
