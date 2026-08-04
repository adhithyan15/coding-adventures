// SIMPLE-level empty-loop-body normalization (fold-control-flow 0.37.0).
//
// A loop whose body folds to an empty block ({}, {;;}, {{}}) has that body
// normalized to an empty statement (;), dropping the braces -- an empty block
// declares no bindings, so the ; form is behaviour-identical:
//   for (var i = 0; i < n; i++) {}  -> for(var i=0;i<n;i++);
//   while (cond) {}                 -> for(;cond;);   (while lowers to for first)
// A non-empty body is unaffected (a single statement just unwraps its braces):
//   for (; run();) { step(); }      -> for(;run();)step();
for (var i = 0; i < n; i++) {}
while (cond) {}
for (; run();) { step(); }
