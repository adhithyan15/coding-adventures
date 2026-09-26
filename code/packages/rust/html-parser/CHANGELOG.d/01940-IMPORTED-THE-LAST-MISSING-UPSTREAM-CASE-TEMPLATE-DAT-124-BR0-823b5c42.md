- **Imported the last missing upstream case, `template.dat:124` (BR02 P1.3).**
  The coverage audit now reports 0 of 1,952 upstream WPT tree-construction
  cases missing (2,655 local cases). The case **fails**, so it is a declared
  expected failure: in a template-context fragment, `</form>` should close the
  open `<div>` (upstream puts `"FG"` at the top level), while the parser keeps
  the div open (`"EF"` inside it). The fix belongs with the spec-structured
  tree builder (BR02 P2). The six focused WHATWG audits whose generators index
  the case were regenerated, and every audit test now consults the shared
  expected-failure list.

