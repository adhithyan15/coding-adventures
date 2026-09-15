- `chemistry/measuring-tools.adj` (new) -- a genuinely new "observation and measurement" axis
  (ADJ-STDLIB-COVERAGE.md 5.1's named Major Gap for K-8 science), distinct from the sibling
  `lab-equipment.adj`'s tool->purpose-verb table. A new `measuring_tool(tool, quantity)` table
  names which ONE quantity each of four common lab tools measures (ruler->length,
  graduated_cylinder->volume, balance->mass, thermometer->temperature), quoted verbatim from a
  Chemistry LibreTexts introductory lab manual, "Introducing Measurements in the Laboratory",
  whose four-part lab exercise each opens with a sentence naming the tool and the quantity/unit
  it measures. WebFetch-verified TWICE for consistency before writing. Deliberately NOT a 5th
  ordinal-bridge instance -- the science lane's four prior slices (season/planet/moon-phase/
  mitosis) already saturate that pattern; this slice diversifies into a different axis
  (observation/measurement) entirely, after a survey of chemistry reaction-types.adj/gas-laws.adj
  and earth-science rock-types.adj found no clean, uninvented causal pairing available without
  fabricating an unstated link. Grounds the NGSS science-practice observation/measurement gap.
  New manifest objective `adj.science.3to5.measuring_tools` (band 3-5, `recall` competency -- a
  pure lookup, not a `rule`-derived fact). New e2e test `facts_measuringtools_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an unshipped tool).
