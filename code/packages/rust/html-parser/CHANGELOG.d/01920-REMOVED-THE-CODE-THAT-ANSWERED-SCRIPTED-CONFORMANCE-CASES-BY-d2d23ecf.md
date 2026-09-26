- **Removed the code that answered scripted conformance cases by string
  matching (BR02 P1.1).** `apply_scripted_tree_construction_side_effects` ran
  whenever scripting was enabled -- including on real pages -- and recognised
  the exact script text of the six `scripted/*` / `scripted_foster01` WPT cases
  (`document.write("2")`, `getElementById("A").id = "B"`, …), rewriting the tree
  to the expected answer. The tree builder also compared a script's text to one
  test's script to suspend parsing. Both are gone, with their helpers and the
  unit test that pinned the fake. There is no script engine, so those six cases
  are now **declared expected failures** in
  `tests/fixtures/tree-construction-expected-failures.txt`, each with its
  reason. The corpus test and the five WHATWG audits that include them require
  each listed case to still fail, so the list can only shrink. The scripted
  cases no longer count as "post-parse repair evidence".
