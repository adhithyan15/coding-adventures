---
category: Compiler / VM / language pipeline
---

# SIR functions and variables are separate namespaces, so a backend whose target has one namespace must keep their spellings apart

**What happened.** In SIR a function is named by `DirectCall`/`MakeClosure`
and a variable by `VarRef`/`Assign`/bindings, and the validator accepts a
module holding both an `f` function and an `f` local.  Ruby relies on that
(`def f; 40; end; f = 2; f() + f` is 42).  The Python, JavaScript,
TypeScript, Go and Rust backends emitted both under the same identifier, so
the local shadowed the function: a runtime `TypeError` in JS/TS/Python and a
compile error in Go/Rust (#18485).  C and Ruby were already safe only
because #18466 had moved every user function to `sir_fn_<name>`.

**Fix.** Each backend has a `var_ident(name)` used at every *variable* site
(locals, parameters, captures, loop variables, `rescue` bindings, and
Python keyword-argument names; module globals keep their spelling).  It returns
`<marker>l<sanitize_ident(name)>` when a module function is named `name`,
and `sanitize_ident(name)` otherwise.  The function side cannot move in JS
and Python, because callers append code that calls emitted functions by
name.  The `l` tag beside `sanitize_ident`'s `v`/`e` tags keeps the map
one-to-one.  A first version renamed JS globals too, and broke a test that
reaches a function's handle through a `Global` reference: check what else
already relies on the shared namespace before splitting it.

**Follow-up (security review of #18788).**  The fix listed two of SIR's
four namespaces.  Globals and constants are namespaces too: JavaScript and
TypeScript declared a global spelled like a function twice (`SyntaxError`)
and let a local hide a global from a `Global` read; Python and TypeScript
bound a constant spelled like a function over it.  And the Python
keyword-argument rename was applied to *runtime* callees, whose parameters
no user function renames.  Each backend now also has `global_ident` and
`const_ident` (JavaScript/TypeScript) or `const_ident` (Python), with tags
`g` and `c` beside `l`, and only a `DirectCall` renames its keywords.

**Do differently.** When a backend maps an IR with several namespaces onto a
target with fewer, list every IR namespace and decide, per pair, which side
gets escaped.  Then route *every* emission site of that kind through one
helper.  A grep for the old sanitizer at each site is the checklist.  An
end-to-end test needs a source language that actually uses both
namespaces at once (here Ruby), or hand-built IR.
