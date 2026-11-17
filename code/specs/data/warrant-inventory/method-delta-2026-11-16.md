# Method delta, 2026-11-14 -> 2026-11-16 (VER30-INV2)

Analysis only; no repository edits. Scratch tooling: `extract.py`, `classify.py`, `analyze.py`, `analyze2.py`, `make_summary.py` copies in this directory. Source: `origin/main` a522491e, 1,499 crates (11-14: 1,459).

## 1. New blocker: `recursive-types`

**Graph.** Per crate, a type-reference graph. Node = a named struct, enum or type alias. Edge A -> B if any field of A (every enum variant field, `pub` struct fields, or the alias body) mentions identifier B anywhere in the type expression, so `Box<B>`, `Vec<B>`, `Option<Rc<RefCell<B>>>`, `HashMap<K, B>`, tuples and `Self` all count. B resolves to the crate's own types first, then `pub` types of its transitive path dependencies. Generic parameter names are skipped. `fn(..)`/`Fn*(..)` types are stripped before scanning: a callback that mentions its own type is not data recursion (without this `virtual-machine::ContextOpcodeHandler` was a false hit).
**Cycles.** Tarjan SCC over all 13,716 nodes; a node is cyclic if it has a self loop or sits in an SCC of size > 1 (mutual recursion). A node "reaches recursion" if it or any successor is cyclic, computed in reverse topological order.
**Tag.** A public entry (same entry set the old classifier votes on) gets `recursive-types` (tier 3) if any identifier in its parameter types, result type, or `self` type resolves to a node that reaches recursion. It then flows through the unchanged required-set vote (cheapest 60% plus forced object protocols), so `req`/`sec` lists and `alone`/cumulative tables treat it like any other feature.
**Modes.** `vis` (default, used for the outputs): only recursion visible in the public structure. `all`: also private fields (a `Tree` whose private `Node` is recursive). Sensitivity numbers use `all`. `extract.py` also now records module-level `type X = ..;` aliases (kept out of the old type table so existing tags do not move).

## 2. Validation

- Known examples (vis): `canonical-cbor` (`CborValue`, 8 of 8 entries), `bounded-json` (`JsonValue`, 4 of 7), `json-value`, `json-serializer`, `json-parser`, `css-parser`, `toml-parser`, `sql-parser` (via `parser::ASTNodeOrToken`), `javascript-ast` (29 of 29), `lisp-parser` (`SExpr`), `binary-tree`, `avl-tree`, `red-black-tree`, `binary-search-tree`, `html-tree-builder` all flagged. Correctly not flagged in vis: arena/map containers (`directed-graph`, `graph`, `skip-list`, `tree`, `heap`, `segment-tree`, `fenwick-tree`, `lsm-tree`); `trie`, `b-tree`, `radix-tree`, `suffix-tree`, `immutable-list` (private recursive `Node`) flag only in `all` mode, as designed. `document-ast` is recursive but has no public fns, so it has no entries to tag.
- Independent cross-check: a plain-text test "type name appears in its own body" over all structs/enums finds 2 pub types that the graph misses (`lattice-ast-to-css::ScopeChain`, whose recursive field is `pub(crate)`, and `mosaic-app-runtime::Hover`, a variant named like the type). Both are correct non-hits in vis. So direct self-reference recall is about 100% by construction; mutual recursion and alias recursion rest on the graph.
- Precision: 14 random cyclic types and 12 random flagged crates read by hand: all genuine data recursion (JSON-like `Value`, ASTs, IR trees, `PaintGroup` -> `PaintInstruction`, `Predicate`). One fn-pointer false positive found and fixed (section 1).
- Known weak spots: (a) same-named types in different modules of one crate merge into one node; (b) `use ... as` renames and `pub use` re-exports from non-path (external) crates do not resolve, giving false negatives; (c) a recursive type reached only through a generic argument of a user type in a signature is caught (identifier scan) but one reached only through a trait object or associated type is not; (d) tagging by reachability means a crate that merely passes `Vec<Program>` where `Program` holds a recursive `Expr` is tagged, which matches "needs recursive types in its contract" but is the main reason 344 crates carry the tag.
- Not done: no new random-sample re-scoring of the T0-T3 tiers; the 11-14 error rates (8.3% overall, T0 about 20%) are carried over.

## 3. Other changes to the base classifier (affect comparability)

1. **Dependency-aware type resolution.** 11-14 resolved a foreign type name via `GLOBAL[name]`, the first `pub` type of that name anywhere, alphabetically. On current main a new crate defining a `Token` with `f64` made ~130 crates (lexers, parsers, compilers) read as needing floats (cumulative floats step 107 -> 228), a pure artifact. Names now resolve own crate -> transitive path dependencies (pub) -> the old `GLOBAL` fallback. Re-run on the 11-14 snapshot this gives T0 119 / T1 21 / T2 35 / T3 1055 / T4 229 (11-14: 119/21/35/1056/228); baseline (specced + text + stateful) 558 vs 576; crates behind the four 671 vs 654. So the fix moves about 18 crates either way on the old snapshot.
2. Repo growth: 1,459 -> 1,499 crates.
3. Unchanged: tier rule, the 10 hand overrides, port counting (same-name dirs; ports recounted from current main: 3,646), security weights (substring regexes, with the false positives noted in the summary, e.g. `sha` in `csharp`).

## 4. Definitions used for the unlock numbers

Baseline lifted set = Bytes<N>, fixed-array, int128, WX-G, WX-F, WX-D2, text, stateful-objects (roadmap steps 1-8). "X alone" = crates adoptable with baseline + X but not with baseline. "Strict alone" (11-14 table) = required set equals exactly {X}. T4 crates are excluded throughout; `async` stays unlifted (1 crate). Security-relevant = security weight >= 2.

Note on the brief: the quoted "26 (opaque), 4 (collections), 35 (generics), 36 (floats)" are the 11-14 strict-alone counts but the labels are permuted; the 11-14 table has floats 26, collections 4, opaque 35, generics 36.
