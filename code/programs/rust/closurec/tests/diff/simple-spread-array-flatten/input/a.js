// SIMPLE-level array-literal spread flattening (constant-fold 0.111.0).
//
// A `...[…]` whose argument is an array LITERAL is inlined into the enclosing
// array literal -- a spread over an array literal yields exactly that array's
// elements in order, so inlining is behaviour-preserving:
//   [...[1, 2], 3]      -> [1,2,3]
//   [0, ...[1, 2], 3]   -> [0,1,2,3]
//   [...[1, ...[2, 3]]] -> [1,2,3]   (fixed-point flattens the nested spread)
//
// A spread whose argument is NOT an array literal (an identifier, string, call)
// is left intact -- `[...y, 4]` stays a spread.
//
// A spread of an array literal with a HOLE (`[...[1, , 3]]`) is intentionally
// NOT inlined (spread iterates and yields `undefined` for the hole, observably
// different from a hole); covered by a unit test, not shown here.
var a = [...[1, 2], 3];
var b = [0, ...[1, 2], 3];
var c = [...[1, ...[2, 3]]];
var d = [...y, 4];
