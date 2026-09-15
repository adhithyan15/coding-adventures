- `physics/simple-machine-function.adj` (new) — a sibling to the already-shipped
  `simple-machines.adj` (`simple_machine_example(machine, example)`, one everyday example per
  simple machine) and its own already-shipped sibling `simple-machine-alt-example.adj` (the second
  everyday example for five of the six). Both decode only the trailing `(examples: ...)`
  parenthetical of each NASA definition sentence; the SAME six sentences open with a leading
  clause stating what the machine actually DOES: "Lever: uses a surface situated on a fulcrum
  (pivoting point) to move an object...", "Screw: helps to fasten two objects together...". New
  `simple_machine_function(machine, function)` table: lever → moves_object_over_fulcrum, inclined
  plane → moves_objects_up_angled_surface, wedge → splits_or_separates_objects, screw →
  fastens_objects_together, wheel_and_axle → turns_or_moves_load, pulley →
  changes_direction_of_object. Covers the full six-machine domain with NO abstention among the six
  (even screw, which `simple-machine-alt-example.adj` must skip since it has no distinct second
  example) — abstains only outside that set. New e2e test file
  `facts_simplemachinefunction_e2e.rs` (3 tests: forward recall with citation, backward recall of
  the fastening machine, honest abstention on a non-simple-machine word). No manifest objective,
  matching `simple-machines.adj`'s own precedent. First slice from a fresh physics/ sweep, launched
  after biology/'s sweep-discoverable candidates were exhausted across 3 rounds.
