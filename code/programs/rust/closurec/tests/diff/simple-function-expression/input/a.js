// A function *value* in every common position — closurec must optimise
// INSIDE each body, not fall back to WHITESPACE_ONLY (gap-153).
var factory = function make(n) { return 1 + 2; };   // named fn-expr (RHS)
var obj = { run: function () { return 2 * 3; } };    // function-valued property
(function () { var y = 3 + 4; })();                  // IIFE
arr.map(function (x) { return x + 0; });             // callback argument
