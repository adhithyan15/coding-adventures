// SIMPLE-level void-operator drop in statement position (closure-pass-dce 0.30.0).
//
// `void <expr>` (ECMAScript 13.5.2) evaluates its operand for side effects and
// yields `undefined`. An expression statement already discards its value, so at
// statement position the `void` wrapper is redundant and is dropped, keeping the
// (impure) operand:
//   void f();        -> f();
//   void a.b();      -> a.b();
//   void new C();    -> new C();    (and the emitter drops the empty arg list)
//   void a(),void b(); -> a();b();  (comma-split first, then each void drops)
//
// A `void` in a NON-statement position keeps it (the `undefined` is observed):
//   h(void g());     -> h(void g());
//
// A `void <pure>` (e.g. `void 0;`) is intentionally NOT removed here (see the
// handler's scoping note): constant-fold folds it to `undefined` before this
// pass runs, and the reference keeps a from-source `undefined;`. Covered by unit
// tests / a follow-up, not shown here so the fixture stays byte-identical.
void f();
void a.b();
void new C();
void a(), void b();
h(void g());
