- **Unflagged tree-construction cases are checked in both scripting modes
  (BR02 P1.2).** html5lib's rule is that a case without `#script-on` or
  `#script-off` must build the same tree either way. They ran only with
  scripting on; `unflagged_cases_match_with_scripting_off` now runs every one
  of them again with scripting off. All pass. A future difference would be
  listed as `<source>#script-off` in the expected-failure file.

