# LEAN00 — Lean 4 on the generic pipeline: a Lean-like machine

**Status:** Draft umbrella programme spec, for owner approval. Spec only; no
code exists. Each layer gets its own child spec (LEAN01 onward) before its
code, except L1, whose spec is [VER11a](VER11a-lean4-proof-kernel.md).

**Owner direction (2026-11-23):** "We should be able to run any Lean proof
through a lexer, parser and other packages… We have our own Constraint VM
and others. I would love to build this as a general purpose Lean like machine
that can execute commands." The kernel must be Lean 4 compatible, and the
same kernel checks the repository's own `proved` labels.

**Builds on:** [LANG00](LANG00-generic-language-pipeline.md) (generic
language pipeline), [LANG01](LANG01-interpreter-ir.md) to
[LANG09](LANG09-notebook-kernel.md) (InterpreterIR, vm-core, JIT, AOT,
tooling), [LANG24](LANG24-constraint-vm.md) (Constraint VM),
[VER09](VER09-cdcl-sat.md), [VER11](VER11-drat-proofs.md) and
[VER11a](VER11a-lean4-proof-kernel.md) (SAT, DRAT, the Lean 4 kernel),
[RS01](RS01-rust-grammars.md) (grammar-engine extensions X1 to X6),
[PR05](PR05-prolog-operator-directive-model.md) and
[PR07](PR07-prolog-op3-execution.md) (the precedent for a grammar extended
while a file is parsed).

---

## 0. The programme in one page

Lean is treated as **one more language on the repository's generic
pipeline**, with one difference: it carries a checking kernel, and everything
else is checked by it.

| Layer | What it is | Trusted? | Oracle it is tested against | Size (our code) | Difficulty |
|---|---|---|---|---|---|
| **L1** Kernel and export checker | Checks Lean 4 declarations from lean4export files | **The only layer that decides**; its own correctness is closed by independent agreement, later verification (VER11a §11.2) | Lean's kernel; nanoda_lib; lean4lean; Lean core and Mathlib as valid corpora | ≤ 8,500 lines of Rust, plus the trusted `bignum-core` surface and a 200-line FFI crate | High, well understood |
| **L2** Grammar | `code/grammars/lean/lean4.{tokens,grammar}` on the shared grammar engine, plus runtime-extensible syntax categories | No | Lean's parser, comparing syntax trees on Lean core and Mathlib source | 2,000–4,000 lines of grammar; 4,000–8,000 lines of grammar-engine extension (parser-driven lexing, categories, runtime rules) | High: Lean's syntax is not fixed |
| **L3** Core elaborator | `def`, `theorem`, `inductive`, `structure`, implicit and instance arguments, unification, pattern matching, structural recursion, basic tactics | No | Lean's elaborator: same source, declarations with the same types | 20,000–40,000 lines of Rust | Very high |
| **L4** Command execution | `#check`, `#print axioms`, `#reduce`, `#eval` by compiling to the LANG VM; decision procedures on the Constraint VM with SAT/LRAT proof terms | No (except a `reduceBool` evaluator, which is an oracle axiom) | Lean's `#eval`; Lean's test suite | 25,000–40,000 lines (compiler, runtime, pretty-printer) | High |
| **L5** Full source compatibility | Lean's own elaborator, compiled to Lean IR, run on our VM; its output re-checked by L1 | No | Lean itself, end to end | 40,000–80,000 lines (IR and `.olean` loading, initialisers, the rest of the runtime, native replacements for C snippets, performance) | Very high; long horizon |

**The rule that makes this safe:** only L1 decides. L2 to L5 can be as
large and as clever as they need to be, because whatever they produce is a
declaration that L1 re-checks, LCF style. A bug in the elaborator yields a
rejected declaration, never a false theorem.

**The order:** L1 first (it also unblocks VER11a's K-M0), then L2 and L4's
command shell, then L3's core, then L5. The layer ranges add up to roughly
90,000 to 175,000 lines over many months. These estimates have wide error bars and are recorded so the owner
can weigh the work, not as commitments.

---

## 1. What Lean is, layer by layer

Lean 4, like Coq and Isabelle, is not just a language that can be lexed and
parsed. It has five parts, and only one of them is about trust:

```
   source text
      │  parser: table-driven, extensible while a file is read
      ▼
   Syntax trees
      │  macro expansion: Syntax → Syntax, user-definable
      ▼
   elaborator: type inference, unification, type classes, coercions,
      │        pattern matching, recursion compilation, tactics.
      │        Written in Lean; much of it is user-extensible (Mathlib's
      │        tactics are Lean programs).
      ▼
   kernel declarations (Expr) ──────► kernel: type-checks each declaration
      │                                (small, trusted)
      ▼
   compiler: Expr → Lean IR → C or interpreter ──► runtime (`#eval`, tactics)
```

- **The kernel** is the part that decides (VER11a). In Lean it is trusted; here, under the owner's rule, its correctness must itself be established (VER11a §11.2).
- **The elaborator** is very large and is what a user experiences as "Lean".
  It is written in Lean, so running it needs the compiler and runtime.
- **The parser** is not a fixed grammar: `syntax`, `notation`, `infix` and
  `macro` commands add rules while a file is being parsed.
- **Mathlib** is a library *and* a body of Lean programs (tactics, linters,
  parsers) that run during elaboration.

What this programme reproduces, and what it reuses:

| Part | Approach |
|---|---|
| Kernel | Written fresh (L1) |
| Parser | Grammar-driven, written fresh (L2) |
| Elaborator | A small core written fresh (L3); the full elaborator is Lean's own, *run* on our VM (L5) |
| Compiler and runtime | Lean definitions compiled to LANG01 InterpreterIR and run on vm-core (L4) |
| Libraries | Lean core and Mathlib, as data, checked by L1 |

---

## 2. Principles

1. **One checking layer.** Every other layer's output is a declaration L1
   re-checks, and no layer may add declarations to the environment except
   through L1. Under the owner's rule (VER31 §3.4: only the compiler is
   trusted), L1 is not *trusted* either: its correctness is an open id,
   `kernel.checker.correct`, closed by independent agreement and, later, by
   verification (VER11a §11.2, Q6). What the design guarantees is that L2 to
   L5 never need to be.
2. **Lean is the oracle, never a dependency.** At every layer the matching
   part of the official Lean toolchain, at one pinned version, is a
   differential oracle (the same terms as Z3 in VER00 §7.5). Our verdict is
   the gate; an oracle that is unavailable never turns red into green.
3. **No third-party code is linked.** Lean's source, its compiled IR, Lean
   core and Mathlib are **input data**: libraries checked by our kernel and
   programs executed by our VM, like any user program. Whether running Lean's
   own elaborator on our VM is acceptable under that rule is owner question
   QA (section 10).
4. **Grammar-driven, no hand-written lexers or parsers** (owner rule). L2
   uses `.tokens` and `.grammar` files and the shared engine. Where Lean's
   syntax needs more than a fixed grammar, the engine is extended, not
   bypassed (section 4.3).
5. **Reuse the substrate** (owner rule). Execution uses LANG00's pipeline;
   decision procedures use the Constraint VM, `cdcl-sat` and LRAT; numbers use
   `bignum-core`. Lean gets no private VM.
6. **One pin.** The Lean version, lean4export version and export format are
   pinned together; moving the pin is its own PR with every corpus rerun.
7. **Literate, as everywhere.** A reader should be able to learn how a proof
   assistant works by reading these packages, layer by layer: "peeking behind
   abstractions".

---

## 3. L1: the kernel and export checker

Specified in [VER11a](VER11a-lean4-proof-kernel.md) sections 3, 4 and 12.
Summary:

- Lean 4's type theory: dependent types, universe polymorphism, mutual and
  nested inductive types with kernel-generated recursors, quotients,
  definitional η for functions and structures, proof irrelevance, unit-like
  types, `Nat` literal operations and `String` literals.
- Input: lean4export files at the pin. Output: the checked environment, and
  for each theorem its axiom footprint (`#print axioms`).
- Rust, no dependencies, `forbid(unsafe_code)`, ≤ 8,500 lines of code (VER11a §4.2),
  literate; also built as a C-ABI library and as WASM, so any language can
  call it.
- Studied, not copied: nanoda_lib and lean4lean.

**Milestones.**

| Id | Milestone | Exit |
|---|---|---|
| L1-a | Lean core library (`Init`, then `Std`) | Every declaration accepted; same verdict as Lean's kernel; Lean's kernel-rejection tests rejected |
| L1-b | Mathlib at the pin | Same; checking time measured and published |
| L1-c | Adversarial corpora | The tricky-declaration corpora independent checkers use, and our own mutation corpus, all rejected |

**Size and difficulty.** About 8,500 lines. The theory is fully documented and
three independent implementations exist to compare against, so this is the
best-understood layer. The hard parts are performance (lazy δ-reduction,
caching, sharing) on Mathlib's scale and nested inductive types.

---

## 4. L2: the Lean 4 grammar

### 4.1 Files

```
   code/grammars/lean/lean4.tokens     lexical grammar
   code/grammars/lean/lean4.grammar    bootstrap syntax: commands, terms,
                                       tactics, levels, attributes, do-notation
   code/grammars/lean/README.md
   code/packages/rust/lean-lexer, lean-parser   thin crates embedding them
```

### 4.2 Lexical features the engine must handle

| Feature | Example | Engine need |
|---|---|---|
| Nested block comments | `/- a /- b -/ c -/`, doc comments `/-- -/`, module docs `/-! -/` | RS01's X1 nested scanner |
| Unicode identifiers and symbols | `α`, `∀`, `→`, `⟨`, `⟩`, `ℕ` | Unicode classes (RS01 brought per-class XID support) |
| Escaped names | `«my name»` | Delimited token |
| Literals | `0x1F`, `0b101`, `1.5e3`, `'c'`, `"…\n…"`, interpolated `s!"x = {x}"` | Interpolation needs a mode switch back into terms |
| Name tokens with dots | `Nat.succ`, `.succ`, `x.1` | Context-sensitive splitting |
| A token table that grows | `notation "⟪" x "⟫" => …` makes `⟪` a token | Runtime token-table extension (4.3) |

**Lean has no separate lexing pass.** Its parser reads tokens on demand from a
token trie that changes as declarations arrive, and some tokens depend on what
the parser expects next. A fixed `.tokens` file is therefore a poor fit on its
own: `lean4.tokens` holds the fixed lexical classes (identifiers, literals,
comments, whitespace), and the engine gains **parser-driven lexing**, where the
parser asks for the next token against the current token table. This is the
largest single engine change L2 needs.

### 4.3 Extensible syntax: why a fixed grammar is not enough, and what we do

Lean's parser is **table-driven and extended while a file is read**. It has
syntax *categories* (`command`, `term`, `tactic`, `level`, `doElem`, `prio`,
`prec`, `attr`, and any category a user declares). Each category has leading
and trailing parser tables indexed by the first token, with Pratt-style
precedence. The commands `syntax`, `notation`, `infix`, `infixl`, `infixr`,
`prefix`, `postfix`, `macro` and `declare_syntax_cat` add entries to those
tables. Much of Lean's familiar notation (`+`, `×`, `∘`, `fun x ↦ …`,
`if … then … else`) is declared that way in Lean's own `Init`, not hard-wired.

So the design has three tiers:

| Tier | Covers | How |
|---|---|---|
| **Bootstrap** | Lean's built-in parsers (the syntax Lean implements in code with `builtin_*_parser`), the category framework and precedence | `lean4.grammar`, fixed |
| **Declared** | Every rule added by `syntax`-family commands | In Lean, such a command is *elaborated and evaluated* to a `ParserDescr`, a small data type (`node`, `trailingNode`, `symbol`, `nonReservedSymbol`, `cat` with precedence, `sepBy`, `sepBy1`, and `unary`/`binary`/`const` applications of named combinators such as `optional` and `many`). L2 does not elaborate: it **reimplements that translation** directly from the parsed `syntax` command to a rule in the same rule language as `lean4.grammar`, with a fixed alias table for the named combinators, and inserts the rule into the category's table before the next command. `notation`, `infix`, `prefix` and `macro` are first expanded, as Lean does, to `syntax` plus `macro_rules` |
| **Code-defined** | Parsers written as Lean code (`@[term_parser] def p : Parser := …`), and aliases or `ParserDescr.parser` references to such code | Not expressible as a grammar. Files that use them are parsed by **Lean's own parser running at L5**, not by L2. Before L5 such a file is refused with a typed error naming the parser. (A hook from our engine into Lean parser functions was considered and rejected: those functions take and return Lean's whole parser state and call back into category parsers, so the marshalling would be a second parser) |

This is the same pattern as Prolog's `op/3` (PR05, PR07): parsing proceeds
command by command, and a declaration changes the table used for what
follows. Nothing is hand-written: the bootstrap is a grammar file, and
declared syntax becomes grammar rules.

Further Lean behaviour L2 must reproduce:

- **Scope.** `scoped` and `local` syntax is active only inside a namespace
  that is `open`, or until the end of a `section`; the tables are scoped
  accordingly.
- **Imports.** Syntax declared in imported modules is active in the importing
  file. L2 parses a file's import closure in import order, or reads the
  declared syntax from the extension snapshot (VER11a §5.3) when the closure's
  source is not at hand.
- **Ambiguity.** When several parsers produce longest parses, Lean keeps all
  of them in a `choice` node and lets the elaborator decide. L2 does the same.
- **Identifiers versus keywords.** A category's `leadingIdentBehavior`
  decides whether an identifier that spells a token is read as either; this is
  per category data.
- **The `prec` and `prio` categories** are themselves parsed, so precedences
  written as expressions (`max`, `arg`, `lead`, `n+1`) are evaluated by L2.

**What the grammar engine (`grammar-tools`) must gain**, in the order L2
needs them. Some are already planned by RS01; the rest are new and land as an
engine batch under `ci-full-build`, because thousands of packages use the
engine.

| Extension | Source |
|---|---|
| Nested scanner for comments | RS01 X1 |
| `parse_rule(name)` and prefix parsing | RS01 X4 |
| Pratt categories: leading and trailing tables, precedence, `cat` references by name | new |
| Runtime rule insertion into a category, and runtime token-table extension | new |
| Position predicates: `colGt`, `colGe`, `lineEq`, `checkWsBefore`, `checkNoWsBefore` (Lean is whitespace-sensitive in `by` blocks, `where`, and tactic sequences) | new |
| Quotations with antiquotations: `` `(term| $x + 1) `` parses a named category inside another, with `$x` holes | new |
| Parser-driven lexing against a changing token trie (4.2) | new |
| `choice` nodes for ambiguous longest parses | new |

### 4.4 Macros

`macro_rules` and `macro` expand `Syntax` to `Syntax`. Two kinds:

- **Pattern macros** (a quotation pattern and a quotation template) are data
  and are expanded by L2/L3 directly, without running Lean code.
- **Macros with arbitrary bodies** (`do` blocks, helper functions) are Lean
  programs; they run on the VM from L4 on.

### 4.5 Oracle and milestones

Oracle: Lean's own parser at the pin, emitting syntax trees for the same
files (a dev-time export); trees are compared after erasing source positions.

| Id | Milestone |
|---|---|
| L2-a | Lean's `Init` source parses, bootstrap plus declared tiers, and matches the oracle's trees |
| L2-b | All of Lean core and `Std` |
| L2-c | Mathlib, except files that need code-defined parsers (listed, with the parser that blocks each) |

**Size and difficulty.** The bootstrap grammar is large: Lean has several
hundred built-in parsers. Estimate 2,000–4,000 lines of grammar and
4,000–8,000 lines of engine extension. High difficulty, mostly in whitespace
sensitivity and in getting precedence and the token table exactly right.

---

## 5. L3: a core elaborator

### 5.1 Scope

The elaborator turns `Syntax` into kernel declarations. Lean's own is very
large: the elaborator and its meta-programming framework are many tens of
thousands of lines of Lean, and they keep moving. L3 builds a **deliberately
small core** with three purposes:

1. elaborate the repository's own Lean (`warrant-lib`, the generated
   per-claim proofs and the escape-hatch proofs of VER11a §9.2 and §10.2), so
   the official toolchain no longer *elaborates* anything in VER11a's path;
2. teach: a readable elaborator, layer by layer;
3. serve as a second, independent elaborator against which L5's results can
   be compared.

The core subset:

| Feature | Note |
|---|---|
| `def`, `theorem`, `abbrev`, `example`, `axiom`, `namespace`, `section`, `variable`, `open` | |
| `inductive`, `structure` (with projections and simple inheritance), `class`, `instance` | Structures and classes are most of real Lean |
| Implicit `{}`, strict implicit `⦃⦄` and instance-implicit `[]` arguments | |
| Unification: first-order plus higher-order patterns, delayed assignments | The core of any elaborator |
| Type-class resolution (tabled) | Needed for every `+`, `=`, `<` |
| Coercions (`↑`), numeric literals via `OfNat` | |
| `match` and pattern matching, compiled to `casesOn` and auxiliary matchers | |
| Structural recursion, compiled to `brecOn` | Well-founded recursion is outside the core |
| `do` notation (core forms) | Needed for `#eval` programs |
| Tactics: `intro`, `exact`, `apply`, `refine`, `rfl`, `rw`, `cases`, `induction`, `constructor`, `assumption`, `decide`, `simp only` with given lemmas | Full `simp`, `omega`, `aesop` and Mathlib's tactics are L5 |

Outside the core: well-founded recursion, `partial`/`unsafe`, `mutual` beyond
simple cases, auto-bound implicits in full generality, `deriving` handlers,
the full `simp`, and every user-defined elaborator (`elab`, `elab_rules`),
which needs L5.

**What L3 needs besides source: an environment-extension snapshot.** An
export carries kernel declarations only. Instances, `@[simp]` sets,
reducibility hints, structure and projection information, coercions,
`match` equations and notation live in Lean's environment extensions, stored
in `.olean` files. L3 cannot recover them from `Init`'s export, and cannot
elaborate `Init` from source (it uses well-founded recursion, code macros and
custom elaborators). So L3 reads a **snapshot of those extensions** for the
modules it imports, dumped by the official toolchain at the pin as data. The
toolchain therefore stays in the build path as a producer of data until L5
can rebuild the snapshot itself (owner question QA). `warrant-lib` and every
hand-written proof stay inside L3's tactic subset from day one, so they never
need rewriting (VER11a §10.2).

### 5.2 Oracle and milestones

Oracle: Lean's elaborator on the same source. The comparison is on
**declaration types** (values may legitimately differ, for example in how a
match is compiled), and every value is checked by L1.

| Id | Milestone |
|---|---|
| L3-a | `warrant-lib` and the generated per-claim proofs elaborate on L3, against the pinned extension snapshot; declarations match the oracle's types; the official toolchain no longer elaborates anything in VER11a's path (it still supplies the snapshot) |
| L3-b | A curated subset of Lean core (for example `Init.Data.Nat.Basic`) elaborates |

**Size and difficulty.** 20,000–40,000 lines of Rust. Very high difficulty:
unification and type classes are where elaborators spend their complexity,
and error messages good enough for agents are their own work.

---

## 6. L4: command execution

### 6.1 Commands

| Command | Implementation |
|---|---|
| `#check e` | Elaborate, infer with L1, pretty-print through a delaborator (sized below; untrusted) |
| `#print axioms c` | L1's footprint walk (VER11a §11) |
| `#reduce e` | A full normaliser built **outside** the kernel on top of L1's `whnf`; untrusted, not in L1's budget |
| `#eval e` | Compile to InterpreterIR, run on vm-core (6.2) |
| `#print c`, `#find`-like queries | Environment queries |

A REPL (LANG08) and a notebook kernel (LANG09) come with the generic pipeline.
This is the "general purpose Lean-like machine that can execute commands".

### 6.2 Compiling Lean to the LANG VM

```
   kernel-level definitions (as elaborated, before recursion is compiled
   to recursors) ──► erase types and proofs ──► compile matchers to switches
   ──► LANG01 InterpreterIR ──► vm-core (LANG02), later jit-core (LANG03)
```

- **Compile from the elaborator's pre-definitions**, not from kernel terms:
  `brecOn`-compiled recursion is correct but very slow to run. So `#eval`
  needs L3 (or L5) to keep the pre-definitions; definitions that arrive only
  as exports can be run, slowly, through recursors.
- **The core library's code.** Any real program calls Lean core functions
  (`List.map`, `String.append`, `IO.println`), whose runnable forms are not in
  the export. They come from Lean's compiled IR for the core library, which is
  L5's loader (7.3). So `#eval` beyond toy programs depends on that loader,
  not only on L3.
- **A Lean runtime on vm-core**: `Nat` and `Int` as big integers
  (`bignum-core`), `Array`, `ByteArray`, `String`, `Thunk`, `Task`, `IO`,
  `ST.Ref`, and the `@[extern]` primitives Lean's core library relies on
  (hundreds of them). Lean's runtime updates values in place when their
  reference count is one; vm-core's memory model decides whether that can be
  matched, which matters for performance, not for correctness.

### 6.3 Decision procedures on the Constraint VM

Tactics that search are **untrusted search plus a proof term** (VER11a §9.5):

| Tactic (ours) | Search | Proof term |
|---|---|---|
| `decide` | none | Kernel reduction |
| `bv_decide`-like | Constraint VM bit-vectors, `cdcl-sat` | LRAT as proof terms, or a verified LRAT checker run by kernel reduction. As a tactic it needs L3's tactic framework |
| `omega`-like (linear integer arithmetic) | Constraint VM LIA | A certificate the tactic turns into a term (Farkas-style for the rational relaxation, plus case splits); later |

Lean's own `bv_decide` closes its goal with `Lean.ofReduceBool`, which trusts
compiled code. Our kernel can only honour that axiom if L4 supplies the
evaluator, and then it is an oracle axiom with its own bucket-(c) id
(`tcb.lean.reduce-bool`, VER11a §4.5). Our own procedures avoid it.

### 6.4 Oracle and milestones

Oracle: Lean's `#eval` on the same programs, and Lean's test suite.

| Id | Milestone |
|---|---|
| L4-a | `#check`, `#print axioms`, `#reduce` on checked exports, from a REPL |
| L4-b | `#eval` of `Nat`, `List`, `String` and `IO` programs from L3 output |
| L4-c | The `bv_decide`-like tactic proves VER11a's bit-level obligations through the Constraint VM, with proof terms L1 accepts |

**Size and difficulty.** 25,000–40,000 lines (compiler, runtime, tactic
glue, pretty-printer and delaborator). High difficulty, mostly the runtime's
breadth.

---

## 7. L5: full source compatibility

### 7.1 Why L3 can never be enough

Mathlib's tactics, linters, `simp` sets, `norm_num` extensions and custom
parsers are **Lean programs that run during elaboration**. Elaborating Mathlib
therefore means running Lean code that manipulates Lean's own elaborator
state. No fixed reimplementation of the elaborator can do that; only Lean's
own elaborator, running, can.

### 7.2 Options

| Option | Idea | Verdict |
|---|---|---|
| A. Reimplement Lean's elaborator | Port the whole elaborator to Rust | Rejected: very large, and a moving target. Every Lean release would break it |
| B. Self-host from source | Compile Lean's own elaborator source with our L3 and L4 | Rejected for now: Lean's own source uses every feature, so our L3 would have to be complete first |
| **C. Run Lean's compiled IR** | Lean compiles every definition, including its own elaborator, to a small first-order IR (`Lean.IR`). Take that IR at the pin, translate it to InterpreterIR, complete the runtime, and run Lean's frontend on our VM. Its output goes to L1 | **Recommended.** The elaborator stays untrusted: L1 re-checks every declaration, LCF style |

### 7.3 Option C in more detail

```
   Lean IR at the pin (from Lean's build artefacts, a dev-time export)
      │  lean-ir-loader: Lean IR → LANG01 InterpreterIR
      ▼
   vm-core + the complete Lean runtime (every @[extern] the frontend uses:
      │      I/O, files, processes, tasks, hashing, …)
      ▼
   Lean's own frontend: parse, elaborate, run tactics ──► declarations
      │
      ▼
   L1 kernel re-checks each declaration ──► environment
```

- **Trust** is unchanged: only L1 decides. A bug in our IR translation or
  runtime produces a declaration L1 rejects, or a crash; never a false theorem.
- **The kernel calls are replaced by L1.** Lean's frontend calls its C++
  kernel through externs (`addDecl`, `Kernel.whnf`, `Kernel.isDefEq`). On our
  VM those externs are bound to L1, so every declaration Lean's elaborator
  produces is checked by our kernel as it is added.
- **Files with code-defined parsers** are parsed here, by Lean's own parser
  running on the VM (L2 does not call into it; LEAN00 §4.3).
- **More than IR is needed.** Five things the IR alone does not give:
  1. **`.olean` loading.** Lean stores each compiled module's environment as
     a binary image laid out for its C++ runtime. Either L5 reads that format
     (version-specific) or it re-elaborates from `Init.Prelude` on every run,
     which is far too slow; a reader is in L5's scope.
  2. **Module initialisers.** The code that initialises a module's constants
     is synthesised by Lean's C emitter, not present as IR declarations; L5
     reproduces it.
  3. **Inline C.** `@[extern c inline "…"]` bodies are C snippets; each is
     reimplemented natively in the runtime.
  4. **`evalConst` and the IR interpreter.** Lean runs non-built-in tactics and
     macros through its own IR interpreter; on our VM, that role is played by
     vm-core itself.
  5. **Where the IR comes from.** Extracting IR at the pin keeps the official
     toolchain in the build path permanently, as a producer of data (owner
     question QA).
- **Performance is the risk.** Lean's frontend is heavy; building Mathlib
  natively takes hours on many cores. Interpreting it on vm-core may be one to
  two orders of magnitude slower, which puts even L5-b (Lean core) at days of
  compute until JIT compilation (LANG03) and AOT (LANG04) are in place. Elaborating all of Mathlib on our VM is a long-horizon goal;
  *checking* Mathlib (L1-b) does not need it.
- **Third-party code as data.** This runs Lean's own code on our VM. It is
  linked into nothing and is replaceable by any other Lean build, but it is
  still running someone else's program to produce our declarations. Owner
  question QA.

### 7.4 Milestones

| Id | Milestone |
|---|---|
| L5-a | Lean's frontend, from IR, elaborates one small file on our VM; L1 accepts the output |
| L5-b | Lean core elaborates on our VM; L1 accepts everything; matches L1-a's environment |
| L5-c | A chosen slice of Mathlib elaborates on our VM |

**Size and difficulty.** 40,000–80,000 lines: the IR and `.olean` loaders,
initialisers, the rest of the runtime with native replacements for inline C,
and performance work. Very high, long horizon. L5 needs L4's runtime and a JIT,
not L3.

---

## 8. How the programme serves software verification

The same kernel checks the repository's own claims (VER11a):

- `warrant-lib` (the WIR semantics and the generic lockstep theorem) is Lean
  source, elaborated by the official toolchain until L3-a, then by L3.
- Per-claim proofs are written straight to export files by the Rust VC
  builder, and checked by L1 on top of Lean core and `warrant-lib`.
- Bit-level obligations go through L4-c's tactic, which uses the Constraint VM.
- The bucket rule is unchanged: the platform baseline is accepted, and our own
  tool gaps must close before `proved` (VER11a §1.1, §13).

So VER11a's K-M0 is L1-a, its escape hatch becomes fully in-house at L3-a, and
its SAT obligations use L4-c.

---

## 9. Package map

```
   code/grammars/lean/lean4.tokens, lean4.grammar       L2
   code/packages/rust/lean-kernel                       L1 (decides; correctness id in (c))
   code/packages/rust/lean-export                       L1 reader (part of the kernel, in its budget) and writer (untrusted)
   code/packages/rust/lean-lexer, lean-parser           L2
   code/packages/rust/lean-elab                         L3
   code/packages/rust/lean-compiler, lean-runtime       L4
   code/packages/rust/lean-tactics                      L4 decision procedures
   code/packages/rust/lean-ir-loader                    L5
   code/packages/rust/lean-cli                          #check, #eval, REPL
   code/packages/lean/warrant-lib                       VER11a library (Lean source)
```

Rust first, as for the other proof tools. Other languages reach the kernel
through its C-ABI and WASM builds; ports of the untrusted layers to other
languages are not planned.

---

## 10. Owner questions

| # | Question | Recommendation | Alternatives |
|---|---|---|---|
| QA | May the official Lean toolchain stay in the build path as a **producer of data** (exports, the extension snapshot, compiled IR), and may Lean's own code (its elaborator as IR, Lean core and Mathlib as libraries) run on our VM and feed our kernel? | Yes: nothing is linked, every declaration is re-checked by L1, and L3 stays as an independent in-house elaborator | No: L5 is dropped, full compatibility is not a goal, and L3 must elaborate everything it needs from source |
| QB | Pin policy | Pin one Lean release and move it deliberately, a PR at a time | Track every release |
| QC | Implementation language | Rust, with C-ABI and WASM for the kernel | Ports of the kernel to other lanes |
| QD | L3's ambition | The small core of section 5.1 | A larger elaborator before L5 |

**Recorded decisions.** 2026-11-23, QA: yes. The official Lean toolchain may
stay in the build path as a producer of data, and Lean's own code may run on
our VM and feed our kernel (owner, in the VER11a session). QB, QC and QD are
open, with the recommendations as working defaults.

## 11. Relationship to other specifications

| Spec | Relationship |
|---|---|
| VER11a | L1's spec, and the software-verification route through the same kernel |
| LANG00 to LANG09 | L4 and L5 run on this pipeline: InterpreterIR, vm-core, JIT, AOT, REPL, notebook |
| LANG24 | Search back end for L4's decision procedures |
| VER09, VER11 | SAT and DRAT; LRAT is added for proof terms |
| RS01 | X1 and X4 are shared grammar-engine extensions; L2 adds Pratt categories, runtime rules and position predicates |
| PR05, PR07 | Precedent: Prolog's operator table changes while a file is parsed |
| VER-BACKLOG | `KRN-*` rows for L1 and the verification route; `LEAN-*` rows for L2 to L5 |

## 12. What this programme does not promise

- Elaborating Mathlib on our own stack soon. Checking Mathlib (L1-b) is near;
  elaborating it (L5-c) is far.
- Bit-for-bit agreement with Lean's elaborator. Agreement is on declaration
  types, and every value is checked by L1.
- That the kernel is small. It is budgeted, literate and checked against Lean
  (VER11a §12), not a few hundred lines.
- Following Lean releases automatically. The pin moves deliberately.
