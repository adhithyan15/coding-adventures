package trace

// Every test in this package that calls Parse or ParseOptions is a differential:
// the hook makes the call run the old scanner AND the grammar path and panic on
// any disagreement the spec's section 4.9 does not allow (crosscheck.go).  No
// existing test had to change for this.
func init() { crossCheck = parseCrossChecked }
