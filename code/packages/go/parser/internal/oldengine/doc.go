// Package oldengine is a FROZEN copy of the grammar-driven parser as it was
// before the compiled/slice-memo rewrite (map-based packrat memo, no depth
// limit). It exists only as the reference oracle for the differential tests in
// the parent package: the new engine must produce the same trees (including
// positions and nil-ness of Children) and the same error text on every input
// the old engine handled.
//
// Do not use it, and do not fix it: it still overflows the stack on deep
// nesting and loops on zero-width repetitions, which is exactly why the
// differential tests only drive it with inputs known to terminate. Never edit
// the copied files; they are the specification of "old behaviour".
package oldengine
