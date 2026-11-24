# VER15a — Weakness Registry v2, Per-Lane Matrices and Language Threat Lists

**Parent:** [VER15](VER15-weakness-registry.md) (the registry and the matrix rules), [VER01](VER01-assurance-report.md) (the report), [VER04i section 8.3](VER04i-polyglot-warrant-evidence.md) (lane isolation), [VER31 section 6.2](VER31-tiered-release-standard.md) (lane packages), [LG02 sections 7 and 9](LG02-basic-gates-cross-lane-contract.md) (error identities and the type-confusion matrix). Backlog item **LGE-S3** of [logic-gates-everywhere](VER-BACKLOG.d/logic-gates-everywhere.md).

**Status:** specification only. Nothing in `ca-assurance-report` reads registry v2 yet, and no validator exists, so no lane evidence can be `live` yet (section 5.2); the integration points are named in section 10 and the work is LGE-W1.

This is an amendment, not a replacement: VER15 stays the authority for the v1 registry, the three closure statuses and the Rust matrix. VER15a adds a second registry file, a matrix per language lane and a per-language threat list. Where it overrides a sentence of VER15, VER04i or VER31, section 3 says so.

---

## 0. The idea in one page

A CVE is rarely a new kind of mistake. It is almost always one of a few hundred structural mistakes that MITRE catalogues as CWEs: a write past the end of a buffer, a value of one type used as another, input that was never checked. VER15 turned that observation into a **weakness registry**: a fixed list of classes, and a rule that every package's report must say, for every class, why it cannot happen (`excluded`), how we know it does not happen (`discharged`), or what is missing (`exemption`).

That worked for Rust, where one analyzer and one proof stack produce the evidence. The logic gates now exist in eighteen languages, and two things became clear:

1. **The registry has no row for the mistakes a gate is most likely to make.** A gate is a function from two bits to a bit. Its characteristic bugs are accepting a value that is not a bit (CWE-20), treating a value of the wrong type as a bit (CWE-843: the string `"1"`, Python's `True`, a Perl dualvar), and converting a value on the way in so that it changes meaning (CWE-704: `256` truncated to `0`, `1.0` read as `1`). Registry v2 adds those three.
2. **A Rust result says nothing about a Python, C or Haskell implementation.** VER04i section 8.3 already says "Rust evidence never feeds a lane". The weakness matrix must obey the same rule, so every lane gets a matrix of its own, computed from that lane's own evidence.

Each language also has pitfalls the shared registry does not name: C has signed-overflow undefined behaviour and `printf` format strings; Perl and Lua coerce strings to numbers; JavaScript has `-0` and prototype pollution; Python's `bool` is an `int`; Go panics; Haskell has partial functions. Each lane therefore carries a **threat list**: the registry classes plus that language's own classes, each with the reason it is plausible, the check that closes it, and a status.

The rule that holds everything together is the owner's: **a release says truthfully what is verified; an open row is labelled, never hidden, and does not block a `tested`-tier release.**

---

## 1. Registry v2: the class list

The file is [`data/weakness-registry-v2.json`](data/weakness-registry-v2.json). It has `"version": 2` and 29 classes: the 26 classes of v1 **copied verbatim, except for the additive deltas of section 1.3** (the validator compares them after JSON parsing and after removing exactly those deltas) plus three new ones. The schema is exactly the v1 schema of VER15 section 3; no field was added, because every v2 need fits an existing field:

* the three new classes use new `family` strings (`input_validation`, `type_safety`), and the v1 loader treats `family` as a free string (`registry.rs:127-131`);
* an `applies_to` of `["all"]` already exists in v1 (CWE-78, CWE-22, CWE-94);
* a class with no real language-level exclusion says so in its `exclusion_rationale` and names a narrow `safe_subsets` entry.

### 1.1 The 29 classes

| CWE | Name | Family | Floor | Excludable via (v1 `safe_subsets`) | Discharged by (`required_evidence`) |
|---|---|---|---|---|---|
| 416 | Use After Free | memory_safety | `proved` | `safe_rust`, `go_race_free`, `gc_runtimes` | `attack-corpus`, `fuzz-checked-allocator`, `memory-safety-proof` |
| 787 | Out-of-bounds Write | memory_safety | `proved` | `safe_rust`, `go_race_free`, `gc_runtimes` | `attack-corpus`, `fuzz-address-sanitizer`, `memory-bounds-proof` |
| 125 | Out-of-bounds Read | memory_safety | `proved` | `safe_rust`, `go_race_free`, `gc_runtimes` | `attack-corpus`, `fuzz-address-sanitizer`, `memory-bounds-proof` |
| 415 | Double Free | memory_safety | `proved` | `safe_rust`, `go_race_free`, `gc_runtimes` | `attack-corpus`, `fuzz-checked-allocator`, `resource-lifecycle-proof` |
| 476 | NULL Pointer Dereference | memory_safety | `proved` | `safe_rust`, `go_race_free`, `gc_runtimes` | `attack-corpus`, `type-exhaustiveness-proof` |
| 190 | Integer Overflow to Buffer Boundary | memory_safety | `proved` | `safe_rust`, `go_race_free`, `gc_runtimes` | `fuzz-integer-boundaries`, `arithmetic-overflow-proof` |
| 78 | OS Command Injection | injection | `proved` | `pure_computation`, `capability_confined_no_proc` | `capability-audit-proc-empty`, `injection-attack-corpus` |
| 89 | SQL Injection | injection | `proved` | `pure_computation`, `capability_confined_no_db` | `capability-audit-no-db`, `parameterized-query-proof` |
| 22 | Path Traversal | injection | `proved` | `pure_computation`, `capability_confined_no_fs` | `capability-audit-fs-empty`, `path-canonicalization-proof` |
| 79 | Cross-Site Scripting (XSS) | injection | `proved` | `pure_computation`, `capability_confined_no_ui` | `capability-audit-ui-empty`, `context-aware-escaping-proof` |
| 94 | Code Injection / Eval | injection | `proved` | `compiled_languages_no_eval`, `no_dynamic_eval_reachable` (v2) | `capability-audit-banned-constructs`, `ast-eval-absence-proof` |
| 116 | Improper Output Encoding | injection | `proved` | `pure_computation_no_serialization` | `protocol-encoder-fuzz`, `escaping-bijection-proof` |
| 117 | Log Injection | injection | `proved` | `pure_computation`, `capability_confined_no_stdout` | `capability-audit-stdout-empty`, `log-sanitizer-proof` |
| 400 | Uncontrolled Resource Consumption | parsing_dos | `bounded(k)` | `pure_zero_allocation` | `microbenchmark-zero-alloc`, `bounded-step-proof`, `rejects-input-bounds` |
| 409 | Decompression Bomb | parsing_dos | `bounded(k)` | `non_decompressing_packages` | `max-expansion-ratio-enforcement`, `streaming-bounded-decoder` |
| 674 | Uncontrolled Recursion | parsing_dos | `proved` | `iterative_only_algorithms` | `ast-call-graph-acyclic`, `recursion-depth-guard-proof` |
| 1333 | Inefficient Regular Expression (ReDoS) | parsing_dos | `proved` | `no_regular_expressions`, `linear_automata_engines` | `regex-linear-guarantee-proof`, `fuzz-redos-attack-corpus` |
| 407 | Inefficient Algorithmic Complexity | parsing_dos | `bounded(k)` | `constant_time_linear_algorithms` | `siphash-secret-seed-proof`, `deterministic-step-bound-proof` |
| 208 | Observable Timing Discrepancy | cryptography | `proved-to-gate-level` | `no_secret_inputs` | `attack-corpus`, `microbenchmark-instructions`, `trace-self-composition-proof` |
| 330 | Use of Insufficiently Random Values | cryptography | `proved` | `pure_computation_deterministic` | `csprng-entropy-audit`, `drbg-statistical-test-suite` |
| 327 | Use of Broken or Risky Cryptographic Algorithm | cryptography | `proved` | `pure_computation_non_cryptographic` | `cryptographic-algorithm-allowlist-audit` |
| 326 | Inadequate Encryption Strength | cryptography | `proved` | `pure_computation_non_cryptographic` | `type-refinement-key-size-proof` |
| 362 | Concurrent Execution Race Condition | concurrency | `proved` | `pure_computation`, `capability_confined_no_thread` | `capability-audit-thread-empty`, `thread-sanitizer-pass`, `race-freedom-proof` |
| 367 | Time-of-check Time-of-use (TOCTOU) Race Condition | concurrency | `proved` | `pure_computation`, `capability_confined_no_fs` | `capability-audit-fs-empty`, `atomic-operation-proof` |
| 502 | Deserialization of Untrusted Data | deserialization | `proved` | `pure_computation_no_deserialization`, `typed_safe_decoders` | `fuzz-deserializer-malformed`, `type-safe-decoder-proof` |
| 1321 | Prototype Pollution | deserialization | `proved` | `safe_rust`, `go`, `c`, `cpp`, `jvm`, `dotnet`, `ocaml`, `no_computed_key_writes_to_shared_objects` (v2) | `language-type-system-guarantee` |
| 20 | Improper Input Validation (new in v2) | input_validation | `proved` | `no_caller_supplied_inputs` | `rejection-corpus-every-operation-every-position`, `short-circuit-rows`, `input-domain-boundary-enumeration` |
| 843 | Access of Resource Using Incompatible Type (Type Confusion) (new in v2) | type_safety | `proved` | `statically_typed_no_escape_hatches` | `type-confusion-matrix-all-operations-all-positions`, `language-type-system-guarantee`, `analyzer-no-escape-hatch` |
| 704 | Incorrect Type Conversion or Cast (new in v2) | type_safety | `proved` | `no_conversions_on_inputs` | `wide-integer-and-coercion-rows`, `conversion-warning-clean-build`, `implicit-conversion-audit` |

### 1.2 The three new classes

**CWE-20, Improper Input Validation** (`family: input_validation`, floor `proved`). The product receives input and does not check that it has the properties the code relies on. For a gate the property is: the value is the bit 0 or the bit 1, in *every* argument position, checked *before* any result is produced. The attack patterns are the failures the LG02 matrix exists to catch: a non-bit accepted, one argument validated but not the other, an early return that skips the second argument, validation delegated to a callee, a wide integer masked before the range check. Nothing in a language excludes this class for a function that takes a value; only a package with no parameters (`no_caller_supplied_inputs`) can exclude it. Everyone else **discharges** it, with a rejection corpus for every operation in every argument position.

**CWE-843, Access of Resource Using Incompatible Type** (`family: type_safety`, floor `proved`). Type confusion: a value of one type is used as another. In C it is a union or a cast; in Python it is `True` accepted as the integer 1; in Perl it is a dualvar whose number says 1 and whose string says `"abc"`; in JavaScript it is an object whose `valueOf` returns 1. A statically typed language closes most of it at compile time, but only for a package that uses none of the escape hatches (`unsafeCoerce`, `Obj.magic`, `reinterpret_cast`, void pointers, reflection), so the exclusion needs `statically_typed_no_escape_hatches`, and an analyzer to check the "no escape hatch" half.

**CWE-704, Incorrect Type Conversion or Cast** (`family: type_safety`, floor `proved`). The conversion changes the meaning of the value: `1.0` read as `1`, `-0.0` as `0`, `256` truncated to `0`, `"1abc"` read as `1`, a `char` widened and compared as an `int`. A package that never converts, narrows, widens, parses or coerces a caller-supplied value can exclude it (`no_conversions_on_inputs`); otherwise it discharges with the wide-integer and coercion rows of LG02 section 9.

LG02 section 9 ("Weakness mapping") says its matrix rows map to these three classes and that the assignment was provisional until this spec. It is now fixed as follows.

| LG02 section 9 row group | Registry class |
|---|---|
| wide and negative integers, short-circuit rows, absence (`nil`/`undef`/`null`) | CWE-20 |
| strings that look numeric, booleans, containers, lane-specific rows (dualvar, `valueOf`, `bool` as `int`, `Rational`) | CWE-843 |
| floats (`1.0`, `-0.0`, `NaN`), the truncation half of the wide-integer rows (`256`, `257`, `2^32 + 1`), `"1abc"`, `"1\n"` | CWE-704 |

A row may map to more than one class (`256` is both an input that must be rejected, CWE-20, and a value that must not be truncated, CWE-704). The lane's threat list records which classes each corpus suite closes; the mapping above is the default.

### 1.3 Additive deltas to the 26 copied classes

v2 is new and is adopted only by the logic gates, so it may add to a copied class where the v1 class gave a whole family of lanes no exclusion route at all. Every delta is **additive** (a new `safe_subsets` entry, a wider `applies_to`, extra sentences in `exclusion_rationale`); no v1 value is removed or reworded, and v1 reports and `weakness-registry-v1.json` are untouched. The deltas are exactly these, and the validator's "first 26 equal v1" check (section 9 rule 1) allows exactly these and nothing else.

| CWE | Delta | Why |
|---|---|---|
| 94 | `safe_subsets` += `no_dynamic_eval_reachable` | The only v1 subset is `compiled_languages_no_eval`, so python, ruby, perl, lua, typescript and elixir could never exclude CWE-94 however plainly a gate never evaluates anything. Those lanes' rows would stay open for ever, which is a statement about the registry, not the package |
| 1321 | `safe_subsets` += `no_computed_key_writes_to_shared_objects`; `applies_to` += `python`, `ruby`, `perl`, `lua` | v1 lists only `javascript` and `typescript`, and gives them no route either. The real class is caller data choosing **which attribute or key is written on a shared object**: JavaScript `Object.prototype[k]`; Python `setattr(OwnClass, k, v)`, `globals()[k] = v`; Ruby `instance_variable_set`/`define_method` on a class or singleton; Perl `bless` into a caller-named package, symbol-table globs; Lua `_G[k]`, `setmetatable` on a shared table. Each is applicable in its lane and needs a route |

**`no_dynamic_eval_reachable`.** An exclusion that uses it holds the class at the registry's `proved` floor **only under fail-closed semantics**, which are normative:

* **(a) Over-approximation.** The analyzer rule is an over-approximation of the package's call graph. Any dynamic dispatch or string-to-callable that it cannot resolve to a constant target is **itself a finding** (`getattr`, `send`, `public_send`, `const_get`, `method_missing`, `AUTOLOAD`, symbolic references, metatable `__index`, `_ENV`, `Reflect`, `apply` with a variable module, a callback, decorator or metaclass of unknown origin). A pass therefore means every call target was resolved and none is an evaluation primitive: sound by construction, never "we did not see one".
* **(b) Negative control.** `exclude_requires` must contain, besides the `live` `analyzer-rule`, a `live` `lane-corpus` item that is a **seeded corpus**: a planted instance of every primitive in the lane's list below, each planted directly **and** via indirection (an alias, a callback, `getattr`/`send`/`__import__`), and the rule must flag every one. A rule that has not been shown to catch the seeded set does not count. A new primitive added to the list requires a new seeded case in the same change.
* **(c) Closed, versioned primitive list.** Each lane's primitives live in the analyzer rule's data as a closed, versioned list; the spec fixes the mandatory minimum below, and a lane may only add. python: `eval`, `exec`, `compile`, `__import__`, `importlib`, `runpy`, `types.FunctionType`, `types.CodeType`, `pickle`/`marshal`/`shelve` loads, `ctypes`, and `getattr`/`globals()`/`locals()` string-to-callable. ruby: `eval`, `instance_eval`, `class_eval`, `module_eval`, `instance_exec`, `binding.eval`, `Kernel#load`, `require` of a variable, `send`, `public_send`, `const_get`, `ObjectSpace`, `Marshal.load`, `method_missing`. perl: string `eval`, `s///ee`, `do EXPR`, `require` of a variable, symbolic references (`no strict 'refs'`), `AUTOLOAD`, `*{"name"} = sub`. lua: `load`, `loadstring`, `loadfile`, `dofile`, `require` of a variable, `debug.*`, `string.dump` with `load`, writes to `setmetatable`, `_G` or `_ENV`. typescript: `eval`, `new Function`, `import()` of a variable, `require(variable)`, `vm.*`, `child_process`, `setTimeout`/`setInterval` with a string, `Reflect.construct`. elixir: `Code.eval_*`, `Code.eval_quoted`, `Code.compile_*`, `Code.require_file`, macros that evaluate caller data, `:erlang.binary_to_term`, `String.to_atom` and `Module.concat` on caller data, `:erlang.apply`, `apply/3` with a computed module or function.
* **(d) Coverage and scope.** The call graph includes transitive dependencies and runtime-loaded modules (a primitive reachable through a dependency is reachable). Build, install and package-manager hooks (`setup.py`, gem extensions, `postinstall`, `mix` tasks) are **out of scope of this package rule**; they belong to the supply-chain row, and the lane list says so.
* **(e) Loads.** `load`, `require`, `import` or `do` of a non-literal path counts as reaching.

Until such a rule and corpus are `live` the row is `open` with the `reached` its own evidence supports; no lane has the analyzer today, so every interpreted lane's CWE-94 row is `open`, but it is now closable where before it could never be.

**`no_computed_key_writes_to_shared_objects`.** An exclusion that uses it requires a `live` `analyzer-rule` (with a seeded negative-control `lane-corpus` as for the eval subset) proving that the package (1) never writes to any class, module, global, prototype or metatable object (`Object.prototype`, a Python class or `globals()`, Lua `_G` or a shared metatable, a Ruby class or singleton, a Perl package or stash); (2) never writes to **any** object under an attribute, key or property name that is not a compile-time constant (so caller data can never choose the name); and (3) never recursively merges caller data into an object. Rule (2) is what the class is about: the earlier phrasing "no write to an object reachable from a caller value" missed `setattr(OwnClass, k, v)`, `globals()[k]`, `_G[k]`, `Object.prototype[k]`, `instance_variable_set` on a singleton and `bless` into a caller-named package, because the shared object is not reachable from the caller's value, only the key is. For a gate this is mechanical (the public surface takes bits and returns bits and writes no computed key). **Decision:** the subset is added, shared by typescript, python, ruby, perl and lua, rather than leaving every such row open for ever. A lane wanting a more specific threat (python `class-pollution`, lua `metatable-mutation`) adds an `origin: lane` threat; it may share the same `analyzer-rule` item.

The compiled lanes keep `compiled_languages_no_eval` for CWE-94. Java, kotlin, csharp, fsharp, swift, dart, go and haskell can still evaluate code through `ScriptEngine`, `Class.forName`, `System.CodeDom`, Reflection.Emit, `GHC.Hint` and similar, so a lane in them that wants to exclude CWE-94 needs an analyzer rule banning those constructs; whether that counts as `compiled_languages_no_eval` is the analyzer rule's definition, not the lane's say-so.

### 1.4 Floors

Registry floors are VER15's: `proved`, `bounded(k)` or `proved-to-gate-level`. The three new classes use `proved`. This is deliberately strict: it matches the rest of the registry, it is what a Rust package should one day reach, and it means a lane that has "only" an exhaustive or tested suite shows the row `open` with the label it did reach (section 7). That is the truthful state; it is not a failure of the lane and it does not block a `tested` release.

**A lane may not discharge a class at a lower floor than the registry.** Three rules make that mechanical:

1. **Labels are ordered** `none < tested < bounded < exhaustive < checked < proved < proved-to-gate-level` (VER31 section 4 order, with the v1 label `proved-to-gate-level` on top).
2. **Mapping of registry floors to lane labels.** `proved` maps to `proved`; `proved-to-gate-level` maps to `proved-to-gate-level`; `bounded(k)` maps to the lane label `bounded`, and the row then records the `k` actually reached in `bound_k` (the schema requires `bound_k` whenever `reached` is `bounded`). A lane row never reports `bounded` without its `k`, and a `k` below what the registry class's evidence needs (for CWE-400, the declared step bound, VER15 section 5) is not accepted.
3. **A registry threat must say its floor.** Every threat with `origin: registry-v2` carries `registry_floor` (the registry's text) and `floor_label` (the mapped label, never lower, and never `none`); the schema requires both. The validator (section 9) checks `floor_label` is at least the mapping of the registry's `minimum_proof_label`, and that a `discharged` row's `reached` is at least `floor_label`.

**A discharge must supply every item of the registry's `required_evidence`.** A threat with a `discharge_requires` list also carries `registry_evidence_map`: for each `required_evidence` id of the registry class, the closure ids (all inside `discharge_requires`) that supply it. If the lane cannot supply one of them (for CWE-400 the registry asks for a microbenchmark, a step-bound proof and an input-bound check), the threat has no discharge route and the row can only be `excluded`, `excluded-by-language` or `open`.

---

## 2. Which registry a report uses

* A report adopts v2 by carrying a top-level `"weakness_registry": 2`. A report without the field is a v1 report, exactly as today. No existing report changes (section 8).
* A report on v2 has 29 rows in `weaknesses` (the Rust or parent matrix) and, if it names lanes, one lane matrix per lane (section 4).
* Adoption is per package. The first package on v2 is `rust/logic-gates` and its lanes.

---

## 3. Where this amends or conflicts with existing specs

These are the places a reader of VER15, VER04i or VER31 would otherwise be surprised. Each is resolved here, and the three other specs should get a one-line pointer (not done in this change; they are listed so a reviewer can decide).

| # | Existing text | Conflict | Resolution |
|---|---|---|---|
| 1 | VER04i 8.3 (lines 1373-1395): lane rungs "never read or write ... the weakness matrix" of the report that carries them, and the `lanes` entry has "exactly these fields" | A lane matrix inside a `lanes[]` entry would break the "exactly these fields" table and blur the isolation sentence | The lane matrix is a **sibling** top-level field, `lane_weaknesses` (section 4). The `lanes` table is untouched. The isolation rule is kept and extended: neither matrix reads the other, and neither affects the other's verdict |
| 2 | VER15 section 4.1: "Exactly 26 entries must be present" | Version-specific number | The count is `|registry|` of the registry the report names: 26 for v1, 29 for v2 |
| 3 | VER15 section 1 invariant 3 and section 4.5: "Any row left open blocks release authorization"; a missing row aborts with exit 2 | A lane row may be `open` | In VER15, "open" meant *unclassified* (a missing row). A lane row `open` is a **classified** status with a mandatory reason and the evidence reached. Completeness is still enforced: a missing lane row is still exit 2. A classified open lane row does not block `tested` and never changes the Rust verdict |
| 4 | VER31 H2: every weakness row "`excluded` or `discharged` or an approved exemption" | Lane rows can be open | H2 is about the parent report's own `weaknesses` and is unchanged. Lane rows never feed H2 (VER31 section 6.2 already caps a lane package at `tested`). Open lane rows must be printed in the release notes (section 7) |
| 5 | VER31 section 6.2: a lane package "still meets every floor check itself" | Does an open lane row fail a floor check? | No. The floor checks F1-F10 are unchanged; the new requirement is *disclosure*: F10's generated `ASSURANCE.md` must contain the lane matrix block of section 7. A lane whose threat list is missing or incomplete fails (section 9), because then the disclosure is not true |
| 6 | VER04i 8.3: the lane loader is "the only source" of lane rungs | A lane matrix also needs the loader's checks as evidence | The lane matrix **consumes** the lane rung and loader results as one kind of evidence (`lane-corpus`); it does not feed them back. The rung table is not changed |
| 7 | VER15 section 4.2: `excluded` only if the registry `safe_subsets` cover the language **and the analyzer verifies** the package stays inside the subset | A language guarantee ("C has no prototype chains") is not an analyzer verification | A lane may not call such a row `excluded`. It uses a distinct status, **`excluded-by-language`**, that is never counted as `excluded`, is allowed only for a closed whitelist of (lane, CWE) pairs in section 5.5, and renders "excluded by language semantics (not machine-checked)" |
| 8 | VER15 section 4.2: the registry `safe_subsets` of a class decide whether a language can exclude it. For the memory classes they list `safe_rust`, `go_race_free`, `gc_runtimes` and no C subset | C memory-safety rows have no registry exclusion route; the reporter itself says manual memory management "must be discharged rather than excluded" (`matrix.rs:476-479`) | The registry is not edited. A **lane extension** table (section 5.5) names the narrow subset `c_no_pointers_arrays_allocation` for C and `cpp_address_surface_allowlist` for C++ (section 5.5) and the classes they apply to; an exclusion that uses it needs the analyzer rules named in the table to be **live**, including the one that rules out undefined behaviour. Every other registry exclusion route must be one of the class's own `safe_subsets` (validator rule, section 9) |
| 9 | VER15 section 4.3: discharge needs "concrete evidence artifacts" for each `required_evidence` item | An artifact in a file is not the same as a tool checking it | Evidence has four states (section 5.2). Only `live` (a named, merged tool establishes it at report time) can close a row. `claimed` (it exists, nothing checks it) cannot, and the row stays `open` with the reason "evidence claimed, not machine-checked" |

---

## 4. The per-lane matrix model

### 4.1 Where it lives

Three places are involved, each with one job:

| What | Where | Written by |
|---|---|---|
| The **threat list** of a language: which classes matter there, why, and what closes each | `code/specs/data/weakness-registry-v2/lanes/<lang>.json` | people (reviewed by an independent adversarial reviewer, LGE-S3b) |
| The **evidence** | the lane package itself: its tests, `BUILD`, build flags, sanitizer runs, and later the lane corpus and analyzer | the lane package |
| The **matrix** in a release | a top-level `lane_weaknesses` array in the parent report (`rust/<name>/assurance_report.json`) | the reporter (`ca-assurance-report`), mechanically |

The matrix is **derived**. The `rows` stored in a threat-list file record what a reviewer claimed and where the evidence is; the reporter recomputes each row's status from the evidence states and the rules of section 5, and the check fails if the stored status differs from the computed one (`LaneWeaknessStale`, regenerate). A status is never taken on trust.

### 4.2 Why a parent-report field and not a lane report

VER31 section 6.2: a lane package "has no report of its own", the gate finds the single report whose `lanes` array names the package, and the package "ships the parent report verbatim" (F10). A per-lane file that the gate does not read would be invisible to the release. So the matrix has to be in the parent report, which already ships.

### 4.3 Why this respects VER04i 8.3

The isolation rule has two halves: lane rungs never read the Rust matrix, and Rust evidence never feeds a lane. `lane_weaknesses` is held to both:

* It is derived only from the lane's threat list and the lane's own evidence. It never reads `weaknesses`, `exemptions`, `claims`, the rungs E0-E5, the effect certificate, the Rust proof manifest or the verdict.
* Those fields never read it. The verdict of a Rust package report is unchanged by any lane row.
* A closure item may name evidence only from the lane's own directory tree, the shared harness it executes (for example `code/packages/c/iso-harness`), the lane's toolchain, and the lane's loader result. A closure item that names a Rust file, report or proof is rejected (`LaneEvidenceCrossing`).

The one thing shared is the **registry** (the list of class names and floors), which is a specification, not evidence.

### 4.4 The `lane_weaknesses` shape

```json
"lane_weaknesses": [
  {
    "lane": "c",
    "package": "c/logic-gates",
    "registry_version": 2,
    "threat_list": "code/specs/data/weakness-registry-v2/lanes/c.json",
    "threat_list_sha256": "<hex of the file bytes>",
    "counts": { "excluded": 0, "excluded_by_language": 0, "discharged": 0, "open": 43 },
    "rows": [
      { "threat": "c.cwe-416", "cwe": 416, "name": "Use After Free",
        "status": "open", "reached": "tested",
        "reason": "<mirrors the threat-list row>", "evidence": ["sanitizer-run:asan+ubsan", "..."] }
    ]
  }
]
```

The headline counts keep `excluded` (machine-checked) and `excluded_by_language` apart; they are never summed into one figure. One entry per lane that has a threat list, sorted by `lane` like `lanes`. A lane named in `lanes` with rung `live` and **no** `lane_weaknesses` entry is refused once the report is on v2: the rung says the lane exists, so the matrix must say what is and is not verified about it. The Rust lane itself has no `lane_weaknesses` entry: its matrix is the parent's `weaknesses`.

---

## 5. Row closure: what can exclude and what can discharge a class

### 5.1 Four statuses

* **`excluded`**: the class cannot occur in this package in this language, **machine-checked**. Requires every item of the threat's `exclude_requires` to be `live` (section 5.2), and the exclusion route to correspond to one of the registry class's own `safe_subsets` or a section 5.5 lane extension (`safe_subset` field). Never inferred from a package manifest.
* **`excluded-by-language`**: a distinct status for a class the *language semantics* rule out and no tool can check (C has no prototype chains). Allowed only for a (lane, CWE) pair on the closed whitelist of section 5.5, and only when that entry is signed off. It is never counted as `excluded`, never counts toward "machine-checked", and renders "excluded by language semantics (not machine-checked)". A threat's `applicability: not-applicable` is only the reviewer's claim and closes nothing by itself.
* **`discharged`**: the class is applicable and is closed by evidence. Requires every item of `discharge_requires` to be `live`, the `registry_evidence_map` of section 1.4 to cover every registry `required_evidence` id, and the strongest label the evidence reaches to be at least the threat's `floor_label`.
* **`open`**: anything else. The row says what is missing and what was reached. A row whose closing items include any `claimed` item is `open` with a reason that begins "Evidence claimed, not machine-checked". It carries `becomes_when` and `becomes_status` if work is planned that closes it, or `no_planned_closure` if none is. **`becomes_when` holds identifiers only, never prose:** each entry is either a closure id of the threat (`kind:slug`, which must appear in its `closed_by`) or a backlog id (`LGE-W1`; matched as section 9 rule 13 says). The schema enforces the pattern; a sentence such as "reviewer sign-off on the exclusion" is written as the backlog id of the work that delivers it (`LGE-S3b`) and the explanation goes in `reason`. An `open` row is the lane analogue of VER15's computed, unapproved exemption, with no approval ceremony: a lane has none to approve and the release is capped at `tested` anyway.

`reached` is the label the row's evidence supports *as claimed*. It describes the evidence, not the status: an `open` row can say `reached: exhaustive` because the truth tables are exhaustive, and still be open because no tool checks that the suite runs.

### 5.2 Evidence kinds

Every closure item has a `kind`, an `id`, and a `state`:

| State | Meaning | Can close a row? |
|---|---|---|
| `live` | A **named, merged** repository tool establishes it at report time (the item's `tool` field names it; the schema requires it). Not a test file in a branch, not a backlog row | yes |
| `claimed` | The evidence exists (a test, a `BUILD` line, a sanitizer run) but no repository tool checks it when the report is generated. Today this is the state of every non-Rust lane's evidence, because no reporter reads registry v2 and no validator exists yet | **no** |
| `planned` | Named work item (`planned_item`, a backlog id such as `LGE-A1` or `LGE-R-python`, matched case-insensitively, see section 9 rule 13), not built | no |
| `unverified` | An attestation about the language that nothing checks (kind `language-guarantee` only) | no |

Until LGE-W1 (the reporter) and LGE-S3v (the validator) are merged, **no lane item can be `live`**, and the C example reflects that: its `live` items are all `claimed`, so no row is closed. When those two land, the lane's tests and `BUILD` lines become `live` by naming them as the tool that re-runs and parses them; that is a change of state in the lane file, not of its structure. The table below says what each kind's `live` will mean.

| Kind | What it is | How it can become `live` (the tool to name) | Can `exclude`? | Can `discharge`? |
|---|---|---|---|---|
| `type-system` | the compiler refuses the wrong-typed program | the lane's build compiles the library and its tests with the lane's strict flags | with an analyzer rule for escape hatches | for CWE-843/704/20 (the rows that cannot be written) |
| `analyzer-rule` | a rule of the lane's capability analyzer (VER08c, LGE-S4/A1/A2) over the library sources | the analyzer runs in the report step; today **no lane has one live** | yes, the main route for package-structure exclusions | rarely (it checks absence) |
| `compiler-flag` | a warning or hardening flag with `-Werror`, for example `-Wconversion` | the flag is on the lane's `BUILD` command line and the build is green | no | contributes, never alone |
| `sanitizer-run` | ASan, UBSan, MSan, TSan over the whole suite | the lane's `BUILD` runs it; a run **skipped** because the compiler cannot build one is not live | no (it shows absence on the inputs run) | contributes to a discharge for runtime-checkable classes |
| `lane-corpus` | the lane-local exhaustive suite now, the `warrant-corpus-v2` replay later | the lane's tests pass; later the lane loader's ten checks (VER04i 8.2) | no | yes: the main route for CWE-20 and the finite-domain rows |
| `type-confusion-matrix` | LG02 section 9, on all eleven gates, bad value in each position | the lane's matrix suite passes with no failing row | no | yes for CWE-843 and CWE-704 |
| `language-guarantee` | a statement about the language (C has no prototype chains) | never `live`; always `unverified` | only for `not-applicable` threats | no |
| `proof` | a machine-checked proof | VER-level proof tool; none exists for any non-Rust lane | yes | yes, the only route to `proved` |
| `build-check` | a check run by the lane `BUILD` that is not one of the above (compilers matrix, drift guard) | the `BUILD` runs it | no | contributes |
| `source-ref` | a claim backed only by a `file:line` or documentation passage in the package itself (a Javadoc `@throws`, a README sentence, a comment that states the contract, a header declaration) | **never `live`**: reading a comment is not a tool establishing a fact. State is `claimed`, `planned` or `unverified` (the schema refuses `live`) and `ref` is required | **no, never** | **no, never** |

**What `source-ref` is for.** A reviewer reads a package and finds the contract stated in the package's own words ("`and(a, b)` throws `InvalidBitException` for any input that is not 0 or 1", `Gates.java:41`). The statement is worth recording so a reader can open it, and it must not be recorded as an `analyzer-rule` or a `lane-corpus` item, because no analyzer ran and no test passed. Rules:

1. It is never `live`, so it can never be the item that closes a row. It may appear in `closed_by` and in a row's `evidence`, and it must carry a `ref`.
2. Its `id` prefix (before the colon) must equal its `kind`, so `source-ref:x` is a `source-ref` and nothing else. It must not appear in `exclude_requires`, `discharge_requires`, or `registry_evidence_map`; route membership is checked on the **resolved item's `kind`** (the `closed_by` entry the id resolves to), not on the spelling of the id. A route that contains one can never be satisfied (the item can never be `live`), so the route would be a lie; the validator rejects it (section 9 rule 15). A route needs the real tool, and the `source-ref` is the pointer to the claim that tool will check.
3. It contributes **no label**. `reached` is computed from the kinds that can reach one (`lane-corpus`, `type-confusion-matrix`, `sanitizer-run`, `compiler-flag`, `proof`); a row whose only evidence is `source-ref` items has `reached: none`. A `source-ref` may therefore accompany evidence of another kind that earns a label (for example the Javadoc line that states what the suite beside it exercises); it never raises the label.
4. It never turns an `applicability: not-applicable` claim into a closure (that is the section 5.5 whitelist's job).

Two rules the table encodes:

* **Absence of a failure is not a proof of impossibility.** A sanitizer run, a fuzz run or a corpus can reach `tested`, `bounded` or `exhaustive` over the inputs run. Only an analyzer rule, the type system plus an analyzer, or a proof can *exclude*.
* **A language statement alone excludes only what the language cannot express.** "Rust is memory safe" is not a reason to exclude a class for a package that might use `unsafe`; "C has no prototype chains" is a reason to exclude prototype pollution in C.

### 5.3 Rules by language family

The lane file's `family` picks the starting expectations below. They are defaults for what the reviewer should look for, not an automatic closure.

**A row's `reached` label comes from that row's own evidence.** Each row's `reached` must be supported by the evidence items listed in **that row's** `evidence` array, and so by kinds that can reach the label (section 5.2). A label may not be carried over from a different row, from a lane-level suite, or from the threat list's other threats. Example: a lane's gate-level corpus (`lane-corpus:truth-tables`) exhaustively checks the eleven gates, so the CWE-20 row may say `reached: exhaustive` and cite that item. The CWE-416 (use after free) row of the same lane may **not** say `exhaustive` or `tested` because "the whole suite passes": the truth tables never allocate or free anything, so for CWE-416 the suite is not evidence of the class. The CWE-416 row cites a sanitizer run, an analyzer rule, or the runtime guarantee, and reaches whatever those reach (often `none`, with status `open`). The validator checks that every `reached` above `none` has at least one evidence item in that row whose kind can reach it and whose closure id is in the threat's `closed_by` (rule 12); it cannot check semantic relevance, so the adversarial reviewer of a lane list does.

**`native-unmanaged` (c, cpp).** The memory-safety family (CWE-416, 787, 125, 415, 476, 190) is *applicable in principle*: the language lets any package take an address, index an array or call an allocator. A package closes it by showing it does none of that **and that it cannot reach undefined behaviour** (the premise of section 5.5), and the evidence is three things together: (1) an analyzer rule that the library declares no pointer, array or struct type and calls no allocator (c/logic-gates: `include/logic_gates.h:46-58` takes `int` by value and returns `int`, per PR #19926), (2) a **sanitizer run** over the whole suite (not a proof of impossibility), ASan+UBSan with `-fno-sanitize-recover=all` over the whole suite (`tools/run.sh:41,56` in that PR), and (3) strict warnings: `-std=c17 -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion -Werror` (`tools/run.sh:37`). Unless all three are `live` (not merely `claimed`) the row stays `open`: sanitizers show the error did not occur on the inputs run. C++ adds: the same three, plus `constexpr` evaluation of the truth tables (a constant expression that triggers undefined behaviour does not compile), deleted overloads for non-int arguments, and a no-exceptions-escape check if `noexcept` is claimed.

**`managed-static` (java, kotlin, csharp, fsharp, swift, go, dart).** Memory safety is excluded by the runtime **only if** the package has no FFI, no `unsafe`/`Unsafe`/`sun.misc`/`unsafe` pointers and no native code (analyzer rule). The characteristic classes are implicit widening and narrowing of `char`/`byte`/`short`, null, and (Go) panics and integer conversion; they are discharged by the lane corpus and the wide-integer rows, with a compiler flag where one exists (`-Xlint`, nullable annotations, `-warnaserror`).

**`functional-static` (haskell, ocaml).** The type system does the most work. The characteristic classes are *partial functions* (`head`, `fromJust`, incomplete patterns; `Obj.magic`, `Array.get` exceptions in OCaml) and escape hatches (`unsafeCoerce`, `unsafePerformIO`). Closed by a compiler flag (`-Wincomplete-patterns -Werror`, OCaml warnings as errors), an analyzer rule banning the escape hatches, and the lane corpus.

**`dynamic` (python, ruby, lua, perl, elixir, typescript/JavaScript).** The compiler does nothing, so CWE-843 and CWE-704 are **applicable** and are discharged only by the **type-confusion matrix** (LG02 section 9) over all eleven operations in each argument position, plus the lane's own rows (numeric-string coercion, truthiness, dualvars, `bool` as `int`, `-0`, prototype pollution). Excluding them needs a proof; none exists. Memory-safety classes are excluded by the runtime on the same FFI condition as above. CWE-94 and CWE-1321 are excluded only through the v2 subsets `no_dynamic_eval_reachable` and `no_computed_key_writes_to_shared_objects` (section 1.3), each needing a live analyzer rule; the compiled-only subset `compiled_languages_no_eval` never applies to these lanes.

### 5.4 What a lane may never do

* Close a row with evidence from the Rust lane (section 4.3).
* Report `discharged` on a probe set when the floor is `exhaustive` or higher. A boundary-complete sample of the 2^64 pairs of C `int` is `tested`, however large.
* Hide a skipped sanitizer run. If the run is skipped, the closure item is not `live` and the row renders accordingly.
* Mark a row `excluded` or `discharged` on `claimed` evidence.
* Mark a row `excluded` because a package manifest says so. `required_capabilities.json` declares capabilities; it checks nothing (the C package's declares none, line 5, and no tool yet compares the sources to it).

### 5.5 The language-exclusion whitelist and the lane safe-subset extensions

**Whitelist of `excluded-by-language`.** A closed list of (lane, CWE) pairs. A lane file may propose `language_exclusion` only for a pair listed here, and the status counts only when `signed_off` is true (a named reviewer, recorded in the lane file). Adding a pair is a change to this table, reviewed like any spec change.

| Lane | CWE | One-line justification | Sign-off |
|---|---|---|---|
| c | 1321 Prototype Pollution | C has no objects with prototype chains, so no program can mutate one (the v1 registry already lists `c` among the `safe_subsets` of CWE-1321) | pending: LGE-S3b adversarial reviewer |
| cpp | 1321 | Classes are fixed at compile time and a C++ object has no runtime-mutable prototype or delegate chain (`cpp` is in the v1 `safe_subsets`) | `signed_off: false`, owner signs |
| java | 1321 | The JVM has no prototype or delegate chain that a program can extend with a property other objects inherit from. **This entry excludes the prototype-chain mechanism only.** Reflection, `Unsafe` and agent-driven writes to shared statics are outside CWE-1321's definition and belong to a lane threat (`java.reflective-shared-state-write`), not to this row (`jvm` is in the v1 `safe_subsets`) | `signed_off: false`, owner signs |
| kotlin | 1321 | Same JVM object model, same limit: prototype-chain mechanism only; reflective shared-static writes belong to a lane threat (`jvm`) | `signed_off: false`, owner signs |
| csharp | 1321 | .NET has no prototype chain; `ExpandoObject` and `dynamic` are per-instance property bags with no prototype to pollute. **Prototype-chain mechanism only**; Reflection and `Unsafe` writes to shared statics are a lane threat, outside CWE-1321 (`dotnet` is in the v1 `safe_subsets`) | `signed_off: false`, owner signs |
| fsharp | 1321 | Same CLR object model and the same limit as C# (`dotnet`) | `signed_off: false`, owner signs |
| go | 1321 | Structs have no prototype chain and no runtime addition of fields; `map[string]T` keys are ordinary data. **Prototype-chain mechanism only**; `unsafe` and `reflect` writes to shared package variables are a lane threat (`go` is in the v1 `safe_subsets`) | `signed_off: false`, owner signs |
| rust | 1321 | Types are fixed at compile time and there is no prototype or delegate chain (`safe_rust` is in the v1 `safe_subsets`; this row is about the object model, not memory safety, which stays a machine-checked `excluded`) | `signed_off: false`, owner signs |
| haskell | 1321 | Immutable data with no objects and no inheritance chain to mutate. `unsafeCoerce` and `unsafePerformIO` corrupt shared state but that is type confusion and effects (CWE-843), not inheritance. **No registry subset covers haskell: this whitelist entry is the sole authority** | `signed_off: false`, owner signs |
| ocaml | 1321 | Records and variants are fixed by their type; no prototype chain. `Obj.magic` corrupts shared state but is type confusion, not inheritance (`ocaml` is in the v1 `safe_subsets`) | `signed_off: false`, owner signs |
| swift | 1321 | Swift classes have no prototype chain. **Conditional**: the entry holds only while the package has no `NSObject` subclass, no `@objc dynamic` member and no Foundation key-value coding (each analyzer-checkable); with those, Objective-C runtime swizzling is reachable and the row stays open. **No registry subset covers swift: this whitelist entry is the sole authority** | `signed_off: false`, owner signs |
| dart | 1321 | Dart has no prototype chain and a `Map` is data. `dart:mirrors` exists in the JIT VM and is absent in AOT builds; it cannot add members to a class either way, so it does not create a prototype chain. **No registry subset covers dart: this whitelist entry is the sole authority** | `signed_off: false`, owner signs |
| elixir | 1321 | Immutable terms, no objects and no inheritance chain; a loaded module cannot be extended by data. The process dictionary and `:persistent_term` are shared mutable state, not inheritance, and belong to a lane threat. **No registry subset covers elixir: this whitelist entry is the sole authority** | `signed_off: false`, owner signs |

**Deliberately not on the list:** python, ruby, perl, lua and typescript. Each has a prototype, class or metatable analogue that caller data can reach (JavaScript `__proto__`; Python `setattr`, `__class__`, `__dict__`; Ruby `instance_variable_set`, `define_method`; Perl symbol-table globs and `bless`; Lua `setmetatable` and `_G`). Their CWE-1321 row is closed, if at all, by the machine-checked `no_computed_key_writes_to_shared_objects` route (section 1.3) or a lane corpus plus analyzer, never by a language statement.

Every pair added here is entered **unsigned**. A lane file records `language_exclusion.signed_off: false`, and the owner's later sign-off flips it per pair; until then the row is `open` with the reason "proposed excluded-by-language, sign-off pending". A pair that is not on this table is refused by the validator even if signed.

**Whitelist and `applies_to`.** The registry's CWE-1321 `applies_to` lists only javascript, typescript, python, ruby, perl and lua, so most whitelisted lanes are outside it. The rule is: **a lane that is on the whitelist uses `excluded-by-language`, never `not-applicable`.** `applicability: not-applicable` stays the reviewer's note and closes nothing, so the row remains `open` ("proposed excluded-by-language, sign-off pending") until the owner signs, and then renders as `excluded by language semantics (not machine-checked)`. A lane that is outside `applies_to` and not on the whitelist cannot close the row by language statement and must discharge or leave it open.

Other lanes and other CWEs are added by LGE-S3b with their own justification and reviewer. A pair like (rust, CWE-416) is **not** a candidate: Rust's memory safety depends on the package using no `unsafe`, which an analyzer checks, so it is a machine-checked `excluded`.

**Lane extensions to `safe_subsets`** (where the registry has no route for a language and the package-level restriction is narrow and checkable). There is one extension per language family and each is lane-specific: the C one does not carry over to C++, because the C++ addressing surface is different (references, `this`, containers, `string_view` and `span`, iterators, lambdas capturing by reference, `new`, and RAII), and a C++ package cannot claim "no pointer, array or allocation" when its rejection path throws a heap-allocated `std::exception`. The C++ extension therefore does not forbid the whole surface; it **allow-lists exactly the rejection path**, and the analyzer rule must fail on anything else.

| Lane | Extension name | Applies to | `exclude_requires` must contain (all `live`) | Extra per-CWE requirement |
|---|---|---|---|---|
| c | `c_no_pointers_arrays_allocation` | CWE-416, 787, 125, 415, 476, 190 | `analyzer-rule:c.no-pointer-array-heap` **and** `analyzer-rule:c.no-ub-reachable` (section 5.6), plus the strict-warning and sanitizer items | CWE-190 also needs `analyzer-rule:c.integer-value-range` |
| cpp | `cpp_address_surface_allowlist` | CWE-416, 787, 125, 415, 476, 190 | `analyzer-rule:cpp.no-address-surface` **and** `analyzer-rule:cpp.no-ub-reachable` (section 5.6.1), `compiler-flag:strict-cxx17` and `sanitizer-run:asan+ubsan`, and `analyzer-rule:cpp.no-global-or-static-state` | CWE-190 also needs `analyzer-rule:cpp.integer-value-range` (covers unsigned and `size_t` arithmetic too) and the statement that the package authors no arithmetic other than on the validated 0/1 range (standard-library length arithmetic is trusted, section 5.6.1); `requires_closed` lists `cpp.signed-overflow-ub`, `cpp.bitwise-shift` and `cpp.uninitialized-variable` and every other cpp threat the `cpp.no-ub-reachable` rule names. **The allow-list holds no pointer parameter and is pinned by content** (section 5.6.1): the rejection helpers (`detail::reject` and `detail::check`) take only `int` arguments, select the parameter name from literals inside the body, and build one message from std::string temporaries produced by one fixed expression plus one exception object |

An exclusion that uses an extension is written `safe_subset: "lane-extension:<name>"`. The extension is the only way a C or C++ memory-safety row can be `excluded`; otherwise it is `discharged` (which for these classes needs evidence the lane does not have) or `open`. The file and this table must agree, and the validator checks it (section 9 rule 10).

**The premise: undefined behaviour voids every guarantee.** In the C abstract machine, a program that reaches undefined behaviour (UB) has no defined meaning at all: the compiler may assume it never happens and delete the checks around it. "The library declares no pointer, array or allocation" therefore does **not**, alone, imply memory safety; a signed overflow, an unchecked shift or a read of an uninitialised object can corrupt the rest of the program in ways the source does not show. The exclusion of the memory-addressing classes holds **only for a package free of undefined behaviour**. Two things make that premise mechanical, and both are required:

1. the extension's `exclude_requires` includes `analyzer-rule:c.no-ub-reachable`, the rule that no UB-prone construct is reachable (section 5.6); and
2. the memory rows carry `requires_closed`: the threat ids `c.signed-overflow-ub`, `c.bitwise-shift` and `c.uninitialized-variable` (and any `c.division-by-zero` if the lane lists one) must themselves be `excluded` or `discharged` before the memory row can be. The validator checks it.

A `requires_closed` threat counts as closed only when its row is `excluded`, or `discharged` on `live` evidence; a row `discharged` on a corpus alone does not count (C and C++).

**The C++ premise.** The same holds in the C++ abstract machine, where the undefined behaviour is wider: signed overflow, shifts, uninitialised reads, ODR violations and mismatched inline definitions across translation units, falling off the end of a non-void function, a `[[noreturn]]` function that returns, lifetime and `constexpr` undefined behaviour, and division. The C++ extension therefore requires `requires_closed` to list `cpp.signed-overflow-ub`, `cpp.bitwise-shift` and `cpp.uninitialized-variable` **and every other cpp threat the `analyzer-rule:cpp.no-ub-reachable` rule names** (for the cpp logic-gates file: `cpp.odr-header-only-version-skew` and `cpp.constexpr-runtime-divergence`).

**A skippable sanitizer run is never `live` for an extension row** (C and C++). `live` needs an affirmative run record naming the sanitizers that ran; a run that the `BUILD` can skip without failing leaves the item `claimed` and the row `open`.

**Not yet machine-enforced.** Rule 10 and `requires_closed` have no validator today (LGE-S3v) and no reporter (LGE-W1) recomputes the status. A lane file therefore stores such rows as `open` and never stores a status the reporter would compute differently.

Until both hold, the release text renders such a row as **open**, with the words "conditional on the package being free of undefined behaviour", never as excluded. A C++ row that does close renders "excluded, assuming a correct standard library".

**Names.** The registry's CWE-190 is "Integer Overflow to Buffer Boundary", a narrow class (wraparound that leads to a wrong buffer size or offset). Release text must render that name and must **never** say "integer overflow excluded". Integer overflow in general is the separate lane threat `c.signed-overflow-ub`, and it stays a separate row.

### 5.6 The C analyzer rules (normative input for LGE-A1 and VER08c)

The C capability analyzer is not built. These definitions are what its rules must mean, so the evidence ids in the lane files are not vague.

**Where it runs.** On the **preprocessed translation unit** of every library file (`src/*.c`, and every header the library installs, including `static inline` functions and macros, which expand into the caller's code). The preprocessed text is what the compiler sees; analysing the source text before preprocessing would miss anything a macro hides. Tests and tools are evidence producers and are not in scope.

**`analyzer-rule:c.no-pointer-array-heap`** is defined on **expression and declaration types after semantic analysis**, not on a list of allocator names. The library may not contain any of:

* an object, parameter, return value, member or typedef of pointer or array type (so `char *`, `int a[4]`, `void (*)(int)`);
* an expression whose type is a pointer after conversion: array-to-pointer decay, **function-designator decay** (a function name used as a value), string literals (an array of `char`), `__func__`, compound literals of array type, address-of (`&`), and callbacks;
* a call to any function that is not defined in the same library or on the allowlist below, so that allocators (`malloc`, `calloc`, `realloc`, `free`, `alloca`, `strdup`, and every one not yet invented) are excluded **because they are not allowed**, not because they are on a list;
* a variadic function or `va_list`, inline assembly, `setjmp`/`longjmp`, and aliasing or section attributes (`__attribute__((alias))`, `section`, `may_alias`, `packed`) that change what storage means.

**Include allowlist:** `<limits.h>`, `<stdint.h>`, `<stdbool.h>` and the package's own header. `<stddef.h>` and `<stdio.h>` are not on it (they define `NULL`, `size_t`, `ptrdiff_t` and the I/O surface). An alternative form of the rule is "used constructs only": the rule lists the constructs the library uses and fails on anything else.

**`analyzer-rule:c.no-ub-reachable`** checks that no UB-prone construct is reachable from a public function: signed arithmetic that can overflow, shifts, division and remainder, reads of an uninitialised object, calls through a declaration that does not match the definition, and the attributes above. It uses the value-range rule below for arithmetic; it is a semantic rule, not a text search.

**`analyzer-rule:c.integer-value-range`** is a **value-range dataflow**, not a syntactic rule: each signed operation is shown, from the validated range of its operands (here: `a` is 0 or 1 after the check), never to overflow, and no division has a zero divisor or `INT_MIN / -1`. For the gates, `1 - a` with `a` in {0, 1} is in {0, 1}. A purely syntactic rule ("no `+` or `*`") would miss `1 - a` and would not be sound. The syntactic companion `analyzer-rule:c.no-shift-operators` (no `<<` or `>>` at all) is separate.

**What the rules do not cover (stated, not hidden):**

* Memory the **caller** owns: the library takes values, so a caller that corrupts its own memory is out of scope.
* Compiler and hardware bugs, and a library compiled with a non-conforming implementation.
* Stack exhaustion and unbounded recursion: that is CWE-674, its own row.
* CWE-121, 122 and 124 (stack and heap buffer overflow, buffer underwrite) are children of CWE-787 and are covered by that row.
* Scalar `static` or global state is not a pointer or array, so it is not excluded by this rule; it is the concern of CWE-362 and CWE-367 and of the "no global or static variable" check on those rows.

### 5.6.1 The C++ analyzer rules (normative input for LGE-A2, LGE-CPP-EXT and VER08c)

The cpp capability analyzer is not built. These definitions fix what `analyzer-rule:cpp.no-address-surface`, `analyzer-rule:cpp.no-ub-reachable` and `analyzer-rule:cpp.integer-value-range` mean.

**Where they run.** On the **preprocessed and template-instantiated translation unit**, for every entity the library declares or defines, including `detail::` and every inline or `constexpr` function. Header-only code is compiled into each caller's translation unit, so the rule runs on the instantiations the package's own tests and installed headers produce, not on the text alone. **Scope is every externally visible entity**: `logic_gates::detail::reject` is callable by any user, so it is public surface for these rules.

**`analyzer-rule:cpp.no-address-surface`** forbids, anywhere in scope: a raw pointer or array object, parameter, return value, member or typedef (a `const char*` parameter is a pointer; a string literal decays to one at the call site, so a function that takes it is not allowed); a reference that escapes its full-expression (a reference parameter or return, a reference member, a by-reference lambda capture, `this` escaping); the address-of operator `&` and pointer arithmetic; `reinterpret_cast`, `const_cast`, C-style casts to a pointer; `new`, `delete` and any allocator object; iterators and `std::string_view`/`std::span`; `std::string::data()`, `c_str()`, `operator[]` and `begin()`/`end()`; static or `thread_local` objects; and any container. A call to anything not defined in the library and not on the include allow-list (`<stdexcept>`, `<string>`) is a finding.

**The allow-list.** Exactly one construct is allowed to allocate: the rejection helpers. They are allowed **only if** (a) they have **no pointer or reference parameter** (they take `int` arguments, for example `reject(int which, int value)` and `check(int which, int value)`, and choose the parameter name from string literals inside the body, so no literal pointer is passed at a call site), and (b) their content matches the pinned shape: the allow-list is identified by the **AST shape and a content hash of the allowed function bodies, not by their names**, so renaming does not widen it and an edit to a body invalidates the pin until a reviewer re-pins it. The pinned shape is one message built from std::string temporaries produced by one fixed expression plus one exception object. The current cpp/logic-gates header does not meet (a): `reject` and `check` take `const char*` (`logic_gates.hpp:77, :83`), so the rule would fail today; the header change is `LGE-CPP-HARDEN` (in progress) and is a prerequisite of every row that uses the extension.

**`analyzer-rule:cpp.no-ub-reachable`** is the C rule of section 5.6 for the C++ abstract machine: no signed arithmetic that can overflow, no shift, no division or remainder with a possible zero or `INT_MIN / -1`, no read of an uninitialised object, no lifetime or `constexpr` undefined behaviour, no function that can fall off its end or a `[[noreturn]]` function that can return, no ODR-violating redefinition, and no call through a mismatched declaration. It is a semantic rule, not a text search, and it uses the value-range rule below.

**`analyzer-rule:cpp.integer-value-range`** is the value-range dataflow of section 5.6, extended to **unsigned and `size_t` arithmetic** (wraparound is defined but still a CWE-190 chain) and subject to the statement **"the package authors no arithmetic other than on the validated 0/1 range"**: every arithmetic, comparison or conversion the library performs has operands in {0, 1} after the validation, or is on the rejection helpers' fixed expression.

**What the C++ extension does not cover (stated, not hidden):**

* The **standard library** (libstdc++, libc++, MSVC STL) and the runtime exception unwinder are **trusted**: this is the repository's compiler and standard-library baseline assumption. Length arithmetic inside `std::string` and `std::to_string` is the standard library's. A row that closes by this extension renders **"excluded, assuming a correct standard library"**.
* **Allocation failure** is `std::bad_alloc` (or `std::terminate` when exceptions are disabled). That is CWE-248 and CWE-703 (the lane threats `cpp.uncaught-exception` and `cpp.exception-identity`), not memory unsafety.
* **Stack exhaustion** and unbounded recursion are CWE-674, its own row.
* Memory the **caller** owns, compiler and hardware bugs, and a non-conforming implementation, as for C.
* Scalar global or static state is the concern of CWE-362 and CWE-367 and of `analyzer-rule:cpp.no-global-or-static-state`, which the extension also requires (the address-surface rule forbids static objects of pointer or reference type; the state rule forbids every static or `thread_local` object).


---

## 6. The threat-list file

### 6.1 Location and naming

`code/specs/data/weakness-registry-v2/lanes/<lang>.json`, one per lane of the eighteen: rust, python, typescript, go, ruby, c, cpp, java, kotlin, csharp, fsharp, elixir, lua, perl, haskell, swift, dart, ocaml. The `lane` field equals the file name. The Rust file exists only for the language-specific threats list; the Rust matrix remains the parent's.

The schema is [`data/weakness-registry-v2/lane-threat-list.schema.json`](data/weakness-registry-v2/lane-threat-list.schema.json) (JSON Schema 2020-12). It checks shape only; section 9 lists what the validator must check beyond it.

### 6.2 Shape, in plain words

* **Header:** `schema_version` (1), `registry_version` (2), `lane`, `family`, `last_reviewed`, `sources` (where the lane-specific classes came from), `toolchain` (language standard and implementations the evidence runs under).
* **`threats`:** the language-level list. Each threat has:
  * `id`: `<lane>.<slug>`, for example `c.cwe-416` or `c.signed-overflow-ub`.
  * `origin`: `registry-v2` (one of the 29; `cwe` required) or `lane` (the language's own; `cwe` optional).
  * `rationale`: why this is plausible in this language for a gate-like package, in prose a beginner can follow.
  * `applicability`: `applicable`, `applicable-in-principle` (the language allows it but a package-level restriction closes it), or `not-applicable` (the language cannot express it).
  * `closed_by`: the checks that close it, each with `kind`, `id`, `state` (`live`, `claimed`, `planned`, `unverified`), an optional `detail`, a `tool` when `live`, and a `planned_item` backlog id (such as `LGE-A1` or `LGE-R-python`, matched case-insensitively) when `planned`. The kind `source-ref` (section 5.2) is never `live`, needs a `ref`, and appears only in `closed_by` and row `evidence`, never in a route list. A `language-guarantee` item is always `unverified`; a `live` `proof` item carries a `ref`.
  * A **route**: at least one non-empty `exclude_requires` (ids that must all be `live`), `discharge_requires` (likewise), a `language_exclusion` (section 5.5: `justification`, `reviewer`, `signed_off`), or `cannot_close_in_package: true`. An empty list is invalid; a threat with no route is vacuous and the schema refuses it. The ids must appear in the same threat's `closed_by` (a rule JSON Schema cannot express; section 9 rule 4).
  * For `origin: registry-v2`: `registry_floor` (the registry's text), `floor_label` (required, never `none`, at least the mapped registry floor), `safe_subset` when there is an exclusion route (section 5.5), and `registry_evidence_map` when there is a discharge route.
* **`considered_not_included`:** the classes of the named sources that were considered and left out, each with `why_not`. It exists so a reader can see the list is deliberate, not a sample.
* **`packages`:** per package of the lane, `package_id`, `reviewed_at_commit` (a hash, or `pr-<n>` while not on main), `scope` (which files the rows speak about), and `rows`: one per threat with `status` (`excluded`, `excluded-by-language`, `discharged`, `open`), `reached` (a label), `bound_k` when `reached` is `bounded`, `reason`, and `evidence` (each item has a `ref` a reader can open: `file:line`, a `BUILD` line, a PR number, a spec section; **never a backlog row**). A closed row (`excluded` or `discharged`) must have non-empty `evidence` and `reached` other than `none`, and may not carry `becomes_*`. An `open` row must carry either `becomes_when` plus `becomes_status`, or `no_planned_closure`.

Keeping threats (about the language) separate from rows (about a package) means the language analysis is written once per lane and each package of the lane (not only the gates) adds a `packages` entry when it adopts v2.

### 6.3 The worked example: C

[`data/weakness-registry-v2/lanes/c.json`](data/weakness-registry-v2/lanes/c.json) covers `c/logic-gates`. At the time of writing the package is on branch `claude/c-logic-gates` (PR #19926), not on main, so every ref is to that branch and `reviewed_at_commit` is `pr-19926`.

**Included, 43 threats:** all 29 registry classes plus 14 C-specific ones.

| C-specific threat | CWE | Why it is in |
|---|---|---|
| `c.signed-overflow-ub` | 191 | Signed overflow is undefined behaviour in C, not wraparound; `1 - a` is the one subtraction |
| `c.implicit-integer-conversion` | 681 (also 192, 194, 195, 196) | C converts and promotes integer types silently; mixed signed/unsigned |
| `c.caller-side-narrowing` | 197 | A caller's `long` narrows to `int` before the library sees it; **the library cannot detect this**, so the row cannot close and every release must say so |
| `c.unchecked-status-code` | 252 | The error model is a status code, and `-1` is truthy: `if (lg_and(a,b))` treats invalid input as true. **Finding:** no `warn_unused_result` is on the declarations (`include/logic_gates.h:43-58`), so ignoring the status is not a build error |
| `c.format-string` | 134 | printf-family with a controlled format; closed by a banned-function rule |
| `c.dangerous-functions` | 242 (also 676) | `gets`, `strcpy`, `sprintf`, `atoi` |
| `c.uninitialized-variable` | 457 (also 824) | C locals are not zeroed; ASan does not detect it, MSan (clang-only) does |
| `c.memory-leak` | 401 | A resource-exhaustion path for a long-running caller |
| `c.bitwise-shift` | 1335 | Negative or oversized shift is undefined behaviour |
| `c.operator-precedence-and-misuse` | 783 (also 480-482) | `a & b == 1` parses as `a & (b == 1)`; a gate is only operators |
| `c.range-check-minimum` | 839 (also 193) | A check `a <= 1` accepts every negative `int` |
| `c.short-circuit` | 768 | `and(0, 2)` returning 0 without looking at the second argument |
| `c.incorrect-call-arity` | 685 | Pre-C23 `int f();` accepts any call; full prototypes close it |
| `c.implementation-defined-behaviour` | 758 | Results may differ by compiler; run under more than one |

CWE-252 and CWE-758 are not in the MITRE C view; they are included because the owner's brief names undefined behaviour and the status-code error model makes ignored returns the C lane's characteristic misuse. The file's `sources` says so.

**Considered and not included, 82 classes.** The source is the 103 members of MITRE CWE-658 ("Weaknesses in Software Written in C") fetched 2026-11-24. Of the 103, 21 are in the 43 threats; the other 82 are each listed in `considered_not_included` with a reason, grouped as: needs a pointer, array, string, buffer or allocation (the library's public surface takes and returns `int` only, `include/logic_gates.h:46-58`, so the specific variant cannot arise and the general registry rows close it); needs a signal handler or threads (none; registry row CWE-362); needs an operating-system, file-descriptor, ioctl or hardware interface (empty capability declaration, `required_capabilities.json:5`); needs an object or data structure; needs a `switch` or an unbraced block (none, `src/logic_gates.c:22-93`); folded into an included threat (off-by-one into `c.range-check-minimum`, the sign and coercion variants into `c.implicit-integer-conversion`, and so on); or is covered by the registry (CWE-190, 787, 125, 416, 415, 476, 843, 704).

**What the example shows about the current state of `c/logic-gates`: 43 rows, 0 closed, 43 open.**

* **No row is closed, and that is the honest state today.** Every item of evidence the C lane has (strict-warning builds, ASan+UBSan runs, the truth-table and rejection suite, full prototypes) is `claimed`: it exists in PR #19926 but no merged tool reads it when a report is generated, so section 5.1 does not let it close a row. The refs point at the actual files (`tests/logic_gates_test.c:39-123`, `tools/run.sh:24-60`, `include/logic_gates.h:43-58`), not at a backlog row. The suite's "about 1.35 million checks" is the arithmetic of the loops at `tests/logic_gates_test.c:105-123` (2,051 probes), not a count anyone re-ran for this file.
* CWE-1321 (prototype pollution) is a *proposed* `excluded-by-language`: it is on the section 5.5 whitelist but awaits its reviewer sign-off, so it is `open` too.
* The memory-safety, capability and package-structure rows are also waiting on analyzer rules that do not exist (LGE-A1). Their `becomes_when` lists every blocker: the analyzer rule, the reporter (LGE-W1), the validator (LGE-S3v), and for rows that cite the sanitizer run, LGE-C3.
* CWE-20, 843, 704 and the registry floors: CWE-20 is `open` with `no_planned_closure` (its floor is `proved`, the rejection set is a boundary-complete probe set and not all 2^64 pairs of 32-bit ints, and no proof route is planned). CWE-843 and CWE-704 wait on the no-cast analyzer rule.
* Two findings from the review are recorded in the file and the backlog:
  * **LGE-C2:** no `warn_unused_result` (or C23 `[[nodiscard]]`) on the `lg_*` declarations (`include/logic_gates.h:43-58`). `LG_INVALID` is `-1`, which is truthy, so `if (lg_and(a, b))` treats invalid input as true (CWE-252). Row `c.unchecked-status-code` lists LGE-C2 in `becomes_when`.
  * **LGE-C3:** `tools/run.sh:58-60` prints "SKIPPED sanitizer run" and carries on when a compiler cannot build a sanitized program, so a green build does not prove the sanitizer ran. The memory-safety rows that cite the sanitizer list LGE-C3 in `becomes_when`; the fix is a build failure on CI when the run is skipped.

The owner's brief described the memory rows as closed by "no pointers/arrays per analyzer, ASan/UBSan exhaustive runs, `-Werror -Wconversion`". The file keeps those as the closing checks and records the route (`lane-extension:c_no_pointers_arrays_allocation`), but a row is not closed on a check that nothing runs (section 5.4). When the reporter, the validator and the C analyzer land, the checks change state to `live` and the rows' `becomes_status` takes effect with no change to the file's structure.

---

## 7. How an open row renders in a release

The generated `ASSURANCE.md` (VER31 section 8) gets one block per lane matrix, after the parent matrix. Open rows are listed, never summarized away.

```text
Weakness matrix, lane c/logic-gates (registry v2: 29 classes + 14 C threats = 43)
  excluded (machine-checked) 0   excluded by language 0   discharged 0   open 43

  open: CWE-416 Use After Free
        evidence claimed, not machine-checked: no tool reads the strict-warning and ASan+UBSan runs
        at report time, and no analyzer rule checks "no pointer, array or allocation"
        (analyzer-rule:c.no-pointer-array-heap, planned LGE-A1).
        conditional on the package being free of undefined behaviour
        (analyzer-rule:c.no-ub-reachable, planned LGE-A1; c.signed-overflow-ub, c.bitwise-shift
        and c.uninitialized-variable must be closed first).
        reached: tested.
        becomes excluded when: LGE-A1, LGE-W1, LGE-S3v, LGE-C3.
  open: CWE-190 Integer Overflow to Buffer Boundary
        (narrow class; this says nothing about integer overflow in general, which is c.signed-overflow-ub)
  open: c.caller-side-narrowing (CWE-197)
        outside the library: a caller's wider integer narrows to int before the call.
        reached: tested.  No planned closure: this package cannot close it.
  open: CWE-1321 Prototype Pollution
        proposed excluded-by-language, reviewer sign-off pending (LGE-S3b).

Rows cover registry classes and listed lane threats, not every possible bug.
```

Rules:

1. The first line states the counts, so "43 open" cannot be missed.
2. An open row begins with the literal text `open:`, then the class, then the reason from the row's `reason` field, then `reached:` (the label) and, if present, `becomes excluded/discharged when:` or `No planned closure`.
3. The reason of a row whose evidence is `claimed` begins "evidence claimed, not machine-checked".
4. The release tier line (VER31 section 8) is unchanged: a lane package is `tested` at most. The block does not say "failed" and does not lower the tier; it says what is and is not verified.
5. **The footer line is mandatory**, verbatim: `Rows cover registry classes and listed lane threats, not every possible bug.`
6. **Both documents carry the block.** The same block appears in the release notes (VER31 section 8) and in the generated `ASSURANCE.md`; one without the other is a failed disclosure.
7. **Until LGE-S5 and LGE-W1 land, this is a manual step.** Neither the gate nor the reporter produces the block yet, so the release checklist item **RC-WEAKNESS-DISCLOSURE** applies: before a lane package is tagged, the releaser pastes the block, generated by hand from the lane file with the counts of section 4.4, into both documents and ticks the item. A release without it is not made.
8. A memory-addressing row (CWE-416, 787, 125, 415, 476, 190) that depends on the C extension renders "conditional on the package being free of undefined behaviour" until `requires_closed` holds. CWE-190 is rendered with its full registry name, never as "integer overflow".
9. A row `excluded-by-language` renders as `excluded by language semantics (not machine-checked)` and is counted on its own line, never inside `excluded`.
10. No wording may imply a class is closed when its row is `open`: the release text may say "lane-local exhaustive" only where LG02 section 9 allows it, and "memory safety is excluded" never while a memory-safety row is `open`.

---

## 8. Migration

* **v1 reports are unchanged.** A report without `weakness_registry` is v1: 26 rows, `weakness-registry-v1.json`, the VER15 rules, no `lane_weaknesses`. Every existing `assurance_report.json` stays byte-identical and `--check` keeps passing without regeneration. The v1 file is never edited again except to fix a typo, and then with a spec note.
* **Adoption is per package, opt-in, one direction.** A package moves to v2 by regenerating its report with `weakness_registry: 2`. Moving back is not offered: dropping rows would hide classes.
* **Order.** (1) `rust/logic-gates` and its lanes; (2) other packages as their owners choose. No package is forced.
* **What the Rust parent matrix does on v2 for the three new classes** is decided when LGE-W1 writes their evaluators, and whatever it decides must be *computed from evidence*, never declared (VER15 section 1, invariant 2). The expectation is: CWE-843 `excluded` under safe Rust (no `unsafe`, no `transmute`, per the analyzer); CWE-20 and CWE-704 discharged by the Warrant `rejects` rows and the `warrant-corpus-v2` evidence if the floor is met, otherwise a computed exemption in the existing form.
* **A lane threat list can be added before its package adopts v2.** The file is data; nothing reads it until a report names `weakness_registry: 2`. That is what lets LGE-S3b (the other 17 lane files) proceed in parallel with LGE-W1 (the reporter).

---

## 9. What the validator (a later item) must check

A validator script is not part of this change. When written (suggested: `code/scripts/validate_weakness_lanes.py`, standard library only, with falsification tests) it must check, at minimum:

0. **Mapping of registry floors:** every registry-v2 threat has `registry_floor` equal to the registry class's `minimum_proof_label`, a `floor_label` at least `proved`->`proved`, `bounded(k)`->`bounded`, `proved-to-gate-level`->`proved-to-gate-level`, and (for a discharge route) a `registry_evidence_map` covering **every** `required_evidence` id of the class with ids inside `discharge_requires`. A `discharged` row with `reached` below `floor_label` fails.
1. **Registry v2 file:** `version` is 2; the first 26 classes equal `weakness-registry-v1.json` after JSON parsing, comparing the `classes` arrays only (top-level `version` and `last_updated` are excluded from the comparison) and allowing exactly the section 1.3 deltas, defined testably: for CWE-94 and CWE-1321 only, `safe_subsets` and `applies_to` are the v1 list followed by the appended entries, and `exclusion_rationale` is the v1 text verbatim followed by appended text (`startswith`); every other field, and every other class, is equal; the three new CWEs (20, 843, 704) are present; no duplicate CWE; every class has the v1 fields; every `minimum_proof_label` is a known label.
2. **Schema:** each lane file validates against `lane-threat-list.schema.json`; `lane` equals the file name; the `family` is one of the four.
3. **Completeness:** every CWE of the registry appears in exactly one threat with `origin: registry-v2`; every threat has a row in every package; every row's `threat` resolves; no threat id repeats.
4. **Closure coherence** (mandatory, and not expressible in the schema): every id in `exclude_requires`, `discharge_requires` and `registry_evidence_map` appears in the same threat's `closed_by`; a `planned` item has a `planned_item` that exists in the backlog (rule 13); every `closed_by` is non-empty; a threat has at least one route (the schema checks the list is non-empty, this rule checks it resolves).
5. **Status is derived, not declared:** recompute each row's status from the rules of section 5.1 and fail if the stored status differs. A row with any `claimed` item in its route is `open`. A row `open` whose route items are all `live` **and whose `reached` is at least the threat's `floor_label`** is a failure (stale); an open row below its floor, or one that is open only because evidence is `claimed`, is correct. An `excluded-by-language` row must be on the section 5.5 whitelist with `signed_off: true`.
6. **No overclaim:** a row with `reached` above what its items can support by kind (for example `proved` without a `proof` item; `exhaustive` on a sanitizer run) fails. A `live` item must name a tool that exists on `main`; a `claimed` item must not name one.
7. **Evidence is real:** every `ref` of the form `path:line` points at an existing file and line (or, for a package not yet on main, a PR number that exists); a `ref` is never a backlog row; every `live` closure item is established by its tool (the table of section 5.2): a `compiler-flag` item whose flags are not on the lane's `BUILD` line fails, a `sanitizer-run` item whose run the `BUILD` can skip must say so in `detail` and is never `live` for a row that uses a lane extension (section 5.5: `live` needs an affirmative run record).
8. **Isolation:** no `ref` or `closure_id` names a path under `code/packages/rust/` for a non-Rust lane, a Rust report, or a Rust proof file (`LaneEvidenceCrossing`).
9. **Honest list:** `considered_not_included` has no CWE that is also in `threats`; every `considered_not_included.cwe` is a member of a source named in `sources` (when the source is a MITRE view, checked against a checked-in snapshot of the view's member list, to be added by LGE-S3b so the validator does not need the network).
10. **Exclusion routes (L9) and extension gating:** the `safe_subset` of every registry-v2 threat with an `exclude_requires` is one of that registry class's own `safe_subsets`, or a `lane-extension:<name>` that appears in the section 5.5 extension table for that lane and CWE. A threat using `lane-extension:c_no_pointers_arrays_allocation` must have `analyzer-rule:c.no-pointer-array-heap` **and** `analyzer-rule:c.no-ub-reachable` in `exclude_requires` plus every per-CWE extra of the table (for CWE-190, `analyzer-rule:c.integer-value-range`), and a `requires_closed` that lists `c.signed-overflow-ub`, `c.bitwise-shift` and `c.uninitialized-variable`; a threat using `lane-extension:cpp_address_surface_allowlist` must likewise have `analyzer-rule:cpp.no-address-surface` and `analyzer-rule:cpp.no-ub-reachable` (and for CWE-190 `analyzer-rule:cpp.integer-value-range`) and a `requires_closed` that lists `cpp.signed-overflow-ub`, `cpp.bitwise-shift` and `cpp.uninitialized-variable`; a row using it is `excluded` only if each `requires_closed` threat's row is `excluded` or `discharged`. A lane threat with `origin: lane` has no registry subset to check. The table and the file must agree.
11. **`becomes_status` is reachable:** an open row's `becomes_status` must be backed by a declared route of its threat: `excluded` needs `exclude_requires`, `discharged` needs `discharge_requires`, `excluded-by-language` needs a `language_exclusion`; and every id in `becomes_when` that is a closure id is in the threat's `closed_by`. (A row whose `becomes_status` has no route can never become what it says.)
12. **Evidence resolves (F6):** every `evidence[].closure_id` of a row appears in the threat's `closed_by` and carries the same `state` there.
13. **Planned items resolve, by exact match, templates included:** every `planned_item` and every `becomes_when` backlog id (schema patterns `^[A-Z][A-Za-z0-9-]*$` and the hyphenated form of section 5.1) exists **verbatim** (exact case; no case folding, the widened pattern is what lets `LGE-R-python` be written as the backlog writes it) as a row id of a `VER-BACKLOG` shard. A backlog row id that contains the placeholder `<lane>` is a **template**. `LGE-L-<lane>` matches `LGE-L-<x>` for any lane id `<x>` from the schema's `lane` enum. Any other template (`LGE-R-<lane>`) matches only the lanes its row names in a machine-readable marker in the row text, `(lanes: queued-a, queued-b)`; a template row with no marker matches no lane, so each lane needs an exact row. An exact row, when present, wins over a template. A `becomes_when` entry is either a closure id of the threat (rule 11) or a backlog id; anything else is a schema failure (section 5.1).
14. **Reachability of the sanitizer claim:** a lane whose `BUILD` can skip the sanitizer (as c/logic-gates can, `tools/run.sh:58-60`) must have the row's `reason` say the run may be skipped; the validator greps the reason for it.
15. **`source-ref` is never a route and never live:** every `closed_by` id has the prefix `<kind>:` of its own kind; no id resolving to a `source-ref` item appears in `exclude_requires`, `discharge_requires` or `registry_evidence_map`; no `source-ref` item has state `live`; every one has a `ref`; a row whose evidence items are all `source-ref` has `reached: none`.
16. **Whitelist and `becomes_when`:** every `language_exclusion` is for a (lane, CWE) pair on the section 5.5 table, and a `signed_off: true` for a pair the table lists as unsigned is refused until the table says otherwise; every `becomes_when` entry matches the section 5.1 identifier forms.

---

## 10. Integration points in `ca-assurance-report` (work item LGE-W1)

Read at the time of writing, so LGE-W1 starts from the real code:

* `registry.rs:86-90` rejects any `version` other than 1 ("expected 1"). It must accept 1 and 2 and take the version from the report.
* `registry.rs:71-74` embeds `weakness-registry-v1.json` with `include_str!`. v2 needs a second embedded snapshot and a chooser.
* `main.rs:82-100` (`find_default_registry`) searches four relative paths for the v1 file; `--registry <FILE>` (main.rs:67) exists. The chooser must pick by the report's `weakness_registry`.
* `registry.rs:189` requires at least 20 classes, which 29 satisfies.
* `matrix.rs:331` builds one row per registry class by calling `evaluate_row`; `evaluate_row` matches on `class.cwe` and its final arm returns `MatrixError::MissingClass` for a CWE it does not know (`matrix.rs:1075`). Evaluators for 20, 843 and 704 must be added before a v2 registry can be loaded for a Rust report, or the build fails closed. That is the correct failure; it is why the registry file ships before the reporter.
* `checker.rs:185` already requires every registry CWE to be in the report's matrix, so completeness is checked against whichever registry is loaded.
* The `lanes` array and `LaneMismatch`/`LabelOverclaim` checks (VER04i 8.3, 9) are the template for `lane_weaknesses`; a new check `LaneWeaknessStale` is added.

---

## 11. Honest limits

* **A registry class is not a bug.** The registry and the threat lists name **classes that recur in CVEs and that we can check for**. Closing every row does not mean the package has no bugs; it means these classes were examined and each is closed, or labelled open, by named evidence. "Every CVE class" anywhere in the program means this and nothing wider.
* **The lists are written by people, from sources.** The C file is seeded from MITRE's C view as fetched on one day. A language will have pitfalls nobody listed. The adversarial review of LGE-S3b exists to find them, and a missing threat is found by adding a row and its evidence, not by amending history.
* **`claimed` is not `live`.** Today the evidence of every non-Rust lane is claimed. A lane matrix can therefore tell a reader what the lane has and what it lacks, but it cannot yet close a row. That is a statement about our tooling, and it is why the C example has no closed row.
* **`live` is only as strong as the tool.** A compiler flag on a `BUILD` line is checked by reading the line; a sanitizer run is a run, which shows no error on the inputs run.
* **`tested` is not `proved`.** Non-Rust lanes reach `tested` (and `exhaustive` over a truly finite domain) until a lane front end and proof tool exist (VER04i section 10). Open rows are the honest way to show that.
* **Some rows can never close inside the package** (`c.caller-side-narrowing`). They stay open and are stated in every release.
* **A closed row is a claim about a revision.** `reviewed_at_commit`, the threat-list hash in the report and the freshness check of F10 bind the matrix to the source it describes.
* **No lane has an analyzer today.** Until one lands (LGE-A1/A2), most exclusions are `open` with `becomes_when`. That is a statement about our tools, not about the packages.

---

## 12. Follow-ups

| Item | Work |
|---|---|
| LGE-S3b | The other 17 lane files, one agent per lane with an independent adversarial reviewer; the CWE-658 member snapshot for the validator |
| (validator) | `validate_weakness_lanes.py` and its falsification tests, section 9 |
| LGE-W1 | Reporter: registry v2, evaluators for CWE-20/843/704, `lane_weaknesses`, `LaneWeaknessStale`, section 10; makes lane evidence `live` by naming the reporter as the tool |
| LGE-C2 | `c/logic-gates`: `warn_unused_result` (and `[[nodiscard]]` for C23) on every `lg_*` function; the status code `-1` is truthy so an ignored result reads as true (CWE-252) |
| LGE-C3 | `c/logic-gates` `tools/run.sh`: a skipped sanitizer run (line 58-60) is a build failure on CI; MSan and TSan runs where the compiler supports them |
| LGE-CPP-EXT | The C++ extension `cpp_address_surface_allowlist` is named in section 5.5; this item builds its analyzer rule `analyzer-rule:cpp.no-address-surface` (references, `this`, containers, `string_view`/`span`, iterators, lambdas capturing by reference, `new`, RAII, with the rejection path allow-listed). No C++ memory row is `excluded` until the rule is `live` |
| RC-WEAKNESS-DISCLOSURE | Manual release-checklist item until LGE-S5/LGE-W1: paste the lane matrix block into the release notes and `ASSURANCE.md` (section 7) |
| LGE-S5 | VER31 amendment: print the lane matrix block in release notes and `ASSURANCE.md` |
| (pointers) | One-line pointers from VER15, VER04i 8.3 and VER31 6.2 to this spec (not done here) |
