//! # lisp_semantics — the Lisp family's value rules, written by the compiler (LANG83 PR7f-2).
//!
//! ## Representation is the IIR's; meaning is the language's
//!
//! A Lisp-family value on the tagged-word backends (native x86-64 / AArch64,
//! LLVM, and `lang-aot`'s McCarthy JIT) is one 64-bit word whose low three
//! bits are its **tag** — the LANG83 PR7f `Dynamic` representation:
//!
//! ```text
//!   tag  value     encoding            dyn_tag
//!   000  integer   n << 3              0
//!   001  nil       the whole word 1    1
//!   010  symbol    (id << 32) | 010    2
//!   011  #f        the whole word 3    3
//!   101  #t        the whole word 5    5
//!   111  pair      pointer | 111       7
//! ```
//!
//! The registry's `dyn_*` primitives build and take that word apart; they
//! say nothing about what a value *means*. Four Lisp rules sit on top:
//!
//! | Rule | Lisp meaning | Emitted as |
//! |------|--------------|------------|
//! | truthiness | only nil and `#f` are false; `0` is true | inline: `(x ≠ 3) × (x ≠ 1)` over the **whole word**, a raw 0/1 for `jmp_if_false` |
//! | `not` | `#t` for nil and `#f`, else `#f` | inline: `5 − 2 × truthy` (the words of `#t` and `#f`) |
//! | `equal?` | atoms equal as words, pairs by structure | `__lisp_equal a b`, a module-local recursive function |
//! | exit code | an integer exits with its value, `#t` with 1, nil and `#f` with 0, a symbol or pair with its word | `__lisp_exit_code x`, a module-local function |
//!
//! Until LANG83 PR7f-2 all four were runtime calls (`dyn_truthy`, `dyn_not`,
//! `dyn_equal`, `dyn_to_exit_code` → `__dyn_truthy` … in `dynval_runtime.c`),
//! so every tagged-word backend carried Lisp's rules. A JavaScript compiler
//! adopting the same representation would need `0` to be false — a different
//! truthiness over the *same* words. So the rules belong to the compiler, as
//! Python's `repr(float)` layout does (PR7c), written only with registry
//! primitives (`dyn_tag`, `dyn_unbox_int`, `dyn_pair_first`,
//! `dyn_pair_rest`), `const`, `cmp_eq` / `cmp_ne`, `mul` / `sub`, `mov`,
//! `jmp` / `jmp_if_false` / `label`, `call` and `ret`. No backend learns
//! anything.
//!
//! ## Why two are inline and two are functions
//!
//! Truthiness and `not` are five and nine straight-line instructions, every
//! one typed `i64`. Written inline, a function that tests a value or returns
//! `(ATOM 7)` stays straight-line and fully typed, so the generic JIT still
//! compiles it — as it compiled the `call_builtin "dyn_truthy"` it replaces.
//! A `call` would send that function to the interpreter tier (the JIT has no
//! call bytecode). `equal?` must recurse, and the exit-code switch branches
//! four ways; both are module-local functions. The exit code applies only to
//! a `call`'s result, so its caller already contained a call.
//!
//! The lowering pass marks a use of truthiness or `not` with a `call` of
//! [`TRUTHY`] / [`NOT`] while it classifies registers, then
//! [`expand_inline_rules`] replaces each marker with its instructions; no
//! function of those names is ever emitted.
//!
//! ## Truthiness reads the whole word, not the tag
//!
//! Only two *words* are false: `#f` is exactly `3` and nil is exactly `1`.
//! The tag alone (`dyn_tag x`, the low three bits) is not the same question.
//! Every well-formed value with tag `011` or `001` is one of those two words,
//! but a malformed word — a raw machine integer that reached a tagged
//! position unboxed, such as `9` (`0b1001`) or `11` (`0b1011`) — has tag
//! `001` or `011` without being nil or `#f`. Such words do occur: Twig passes
//! an unboxed parameter straight into `dyn_pair_new`, so
//! `(define (f n) (if (car (cons n (list))) 1 2)) (f 9)` tests the raw word 9.
//!
//! ```text
//!   word   tag    whole-word rule     tag-only rule
//!   1      001    false (nil)         false
//!   3      011    false (#f)          false
//!   9      001    TRUE                false   <- the tag-only rule disagrees
//!   11     011    TRUE                false   <- …and here
//!   5, 0   …      true                true
//! ```
//!
//! The runtime's `__dyn_truthy` (and the generic JIT's `is_truthy`) always
//! compared the whole word, so that is the rule this module emits: two
//! word comparisons (`cmp_ne x, 3` and `cmp_ne x, 1`, both typed `i64`)
//! and a `mul`. A first draft tested `dyn_tag x` instead and silently turned
//! `(f 9)` from `1` into `2` on the native and LLVM columns.
//!
//! ## Module-local, compiler-owned, only when used
//!
//! [`add_used_helpers`] appends `__lisp_equal` / `__lisp_exit_code` only if
//! some instruction `call`s it, and only once. The names are the Lisp
//! lowering's own (`__lisp_*`), never a runtime's: the native runtimes export
//! `__twig_*`, `__dyn_*` and `__gc_*`, and a module function that shared a
//! name with a linked runtime symbol could be bound to the wrong definition
//! at link time
//! (`lessons.d/module-local-helpers-in-generated-code-must-not-share-a-name-with.md`).
//!
//! ## The `__lisp_` prefix is reserved
//!
//! The pass recognises its rules **by name**: a `call __lisp_not x` is a
//! marker to expand, and a function named `__lisp_equal` is set aside as
//! already lowered. A name is only a safe signal if no program can spell it.
//! Twig's identifiers can (`[A-Za-z…_$][…]*` admits `__lisp_equal`), so before
//! this was closed `(define (__lisp_equal a b) 42)` made every `equal?` call
//! the user's function, and `(define (__lisp_not x) 0) (__lisp_not 5)` was
//! silently replaced by Lisp's `not`.
//!
//! Two layers now close it:
//!
//! 1. **This pass refuses** ([`check_reserved_names`], run first by
//!    `lower_dyn_repr`): a module that defines, or `call`s, one of the four
//!    [`RESERVED_NAMES`] is a [`ReservedNameError`] — unless the
//!    function is *exactly* the helper [`add_used_helpers`] would emit (the
//!    pass ran before, or a test built the module with this module's own
//!    builders). So every marker the pass later sees is its own. Other
//!    `__lisp_`-prefixed names pass: this pass also runs on Nib's and
//!    ALGOL's modules, and nothing here recognises them.
//! 2. **The frontends reserve the prefix** (Twig rejects any identifier that
//!    begins with `__lisp_` or `__dyn_`; McCarthy's atoms are upper-case and
//!    cannot spell one), so a user sees a source-level error with a position
//!    instead of an IIR-level refusal.
//!
//! ## Typing
//!
//! A helper's dynamic-value parameters and the pair fields it reads are typed
//! `ref<any>`, like every other tagged value, so the precise collector sees
//! them as roots: the native backends poll a safepoint at every function
//! entry, and a compacting collection there must relocate the pairs
//! `__lisp_equal` is walking. Tags, `#t`/`#f` words and exit codes are plain
//! words (`i64`). The helper functions contain `jmp`/`label` (`void`), so
//! `infer_type_status` makes them `PartiallyTyped`, and the McCarthy JIT runs
//! them on its interpreter tier.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::IIRModule;

/// Lisp truthiness, marked as `dest = call __lisp_truthy x : i64` while the
/// lowering classifies registers and expanded inline by
/// [`expand_inline_rules`] (a raw `0` for nil or `#f`, `1` for anything else,
/// the integer 0 included). What a `COND` clause's `jmp_if_false` tests.
pub const TRUTHY: &str = "__lisp_truthy";
/// Lisp `not`, marked as `dest = call __lisp_not x` and expanded inline: the
/// word of `#t` (5) for nil or `#f`, else `#f` (3).
pub const NOT: &str = "__lisp_not";
/// Lisp `equal?`: `__lisp_equal(a: ref<any>, b: ref<any>) -> ref<any>`,
/// `#t` or `#f`. A module-local function, recursive over pairs.
pub const EQUAL: &str = "__lisp_equal";
/// The program-exit coercion of a value whose tag is known only at run time
/// (a lambda's result): `__lisp_exit_code(x: ref<any>) -> i64`. A
/// module-local function.
pub const EXIT_CODE: &str = "__lisp_exit_code";

/// The prefix every name above shares. Lisp-family frontends reserve the
/// whole prefix in source (Twig's `reserved_names`), so a later rule can
/// take a new `__lisp_` name without a source program already using it.
pub const RESERVED_PREFIX: &str = "__lisp_";

/// The four names this pass recognises, refused by [`check_reserved_names`]
/// when a module defines or calls one the compiler did not write.
///
/// Only these four — not the whole [`RESERVED_PREFIX`]: `lower_dyn_repr`
/// runs on every module the tagged-word pipelines compile, Nib's and
/// ALGOL's included, and a Nib function named `__lisp_helper` collides with
/// nothing here. Refusing it would reject a correct program for no reason.
pub const RESERVED_NAMES: [&str; 4] = [TRUTHY, NOT, EQUAL, EXIT_CODE];

/// The rules emitted inline (their names are pass-internal markers).
pub const INLINE_RULES: [&str; 2] = [TRUTHY, NOT];
/// The rules emitted as module-local functions, in the order
/// [`add_used_helpers`] appends them.
pub const HELPERS: [&str; 2] = [EQUAL, EXIT_CODE];

/// The tags the rules test (see the module table).
const TAG_INT: i64 = 0;
const TAG_NIL: i64 = 1;
const TAG_FALSE: i64 = 3;
const TAG_TRUE: i64 = 5;
const TAG_PAIR: i64 = 7;
/// nil, `#f` and `#t` are whole-word singletons: the word *is* the tag.
const WORD_NIL: i64 = TAG_NIL;
const WORD_FALSE: i64 = TAG_FALSE;
const WORD_TRUE: i64 = TAG_TRUE;

/// The type hint of a tagged dynamic value.
const DYN: &str = "ref<any>";
/// The type hint of a plain machine word.
const WORD: &str = "i64";

/// `dest = call rule, args… : ty` — a use of one of the four rules.
pub fn call(dest: &str, rule: &str, args: &[&str], ty: &str) -> IIRInstr {
    let mut srcs = vec![Operand::Var(rule.to_string())];
    srcs.extend(args.iter().map(|a| Operand::Var((*a).to_string())));
    IIRInstr::new("call", Some(dest.to_string()), srcs, ty)
}

/// If `instr` is a `call` of one of the four rules, its name.
///
/// This recognises a call **by name**, so it is only meaningful on a module
/// that has passed [`check_reserved_names`]: there, every such call is one
/// the compiler wrote.
pub fn rule_called(instr: &IIRInstr) -> Option<&'static str> {
    if instr.op != "call" {
        return None;
    }
    match instr.srcs.first() {
        Some(Operand::Var(name)) => INLINE_RULES.iter().chain(HELPERS.iter()).copied().find(|h| h == name),
        _ => None,
    }
}

/// Replace every `call` of [`TRUTHY`] or [`NOT`] in `func` with the rule's
/// straight-line instructions, writing the same `dest`.
///
/// The calls are the compiler's own markers; a module from a frontend must
/// pass [`check_reserved_names`] first (as `lower_dyn_repr` does), or a
/// user's `call __lisp_not` would be expanded too.
pub fn expand_inline_rules(func: &mut IIRFunction) {
    if !func.instructions.iter().any(|i| matches!(rule_called(i), Some(TRUTHY | NOT))) {
        return;
    }
    let old = std::mem::take(&mut func.instructions);
    let mut out = Vec::with_capacity(old.len() + 8);
    for instr in old {
        let rule = rule_called(&instr);
        let (Some(rule @ (TRUTHY | NOT)), Some(dest), Some(Operand::Var(x))) =
            (rule, instr.dest.as_deref(), instr.srcs.get(1))
        else {
            out.push(instr);
            continue;
        };
        if rule == TRUTHY {
            truthy_into(&mut out, dest, x);
        } else {
            not_into(&mut out, dest, x);
        }
    }
    func.instructions = out;
}

/// ```text
/// dest.nf  = x ≠ 3                 ; not the word #f   (compared as i64)
/// dest.nn  = x ≠ 1                 ; not the word nil  (compared as i64)
/// dest     = dest.nf × dest.nn     ; 1 unless nil or #f — 0 is true
/// ```
///
/// The **whole word** is compared, not `dyn_tag x`: a malformed word such as
/// a raw `9` has nil's tag without being nil, and Lisp (like the runtime
/// rule this replaces) calls it true — see the module docs. The comparisons
/// carry the `i64` hint: a comparison's hint is its operand type, and these
/// operands are compared as plain 64-bit words even though `x` itself is a
/// `ref<any>` the collector still tracks.
fn truthy_into(out: &mut Vec<IIRInstr>, dest: &str, x: &str) {
    let not_false = format!("{dest}.nf");
    out.extend(cmp_const("cmp_ne", &not_false, x, WORD_FALSE));
    let not_nil = format!("{dest}.nn");
    out.extend(cmp_const("cmp_ne", &not_nil, x, WORD_NIL));
    out.push(binop("mul", dest, &not_false, &not_nil));
}

/// ```text
/// dest.truthy = truthy x
/// dest        = 5 − 2 × dest.truthy   ; #t (5) when false-y, #f (3) when true
/// ```
fn not_into(out: &mut Vec<IIRInstr>, dest: &str, x: &str) {
    let truthy = format!("{dest}.truthy");
    truthy_into(out, &truthy, x);
    let two = format!("{dest}.k2");
    out.push(konst(&two, 2));
    let twice = format!("{dest}.twice");
    out.push(binop("mul", &twice, &two, &truthy));
    let five = format!("{dest}.k5");
    out.push(konst(&five, WORD_TRUE));
    out.push(binop("sub", dest, &five, &twice));
}

/// Append, once each, the helper functions some instruction in `module`
/// calls (`__lisp_equal` calls only itself). A module that calls none is
/// left untouched.
///
/// A function that already carries a helper's name is accepted only when it
/// *is* that helper, instruction for instruction (this function ran before);
/// anything else is a [`ReservedNameError`], never silently kept in the
/// helper's place.
pub fn add_used_helpers(module: &mut IIRModule) -> Result<(), ReservedNameError> {
    for helper in HELPERS {
        if let Some(f) = module.functions.iter().find(|f| f.name == helper) {
            if !is_compiler_helper(f) {
                return Err(ReservedNameError::defined(&f.name));
            }
            continue;
        }
        let used = module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter())
            .any(|i| rule_called(i) == Some(helper));
        if used {
            module.functions.push(build_helper(helper));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reserved names
// ---------------------------------------------------------------------------

/// A module used a name the Lisp lowering reserves ([`RESERVED_NAMES`]).
///
/// | `call`er  | meaning                                                      |
/// |-----------|--------------------------------------------------------------|
/// | `None`    | the module **defines** a function named `name`               |
/// | `Some(f)` | function `f` **calls** `name`, which the compiler did not write |
///
/// Either way the pass would otherwise have mistaken the program's function
/// for its own: used it as `equal?`, or expanded a call of it inline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedNameError {
    /// The reserved name the module used.
    pub name: String,
    /// The function containing the offending `call`, or `None` for a
    /// definition.
    pub caller: Option<String>,
}

impl ReservedNameError {
    fn defined(name: &str) -> Self {
        ReservedNameError { name: name.to_string(), caller: None }
    }
}

impl std::fmt::Display for ReservedNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.caller {
            None => write!(f, "the program defines a function named {:?}", self.name)?,
            Some(caller) => write!(f, "function {caller:?} calls {:?}", self.name)?,
        }
        write!(f, ": {RESERVED_NAMES:?} are reserved for the compiler's Lisp rules")
    }
}

impl std::error::Error for ReservedNameError {}

/// Refuse a module that defines or calls one of the [`RESERVED_NAMES`] the
/// compiler did not write. Other `__lisp_`-prefixed names pass: nothing
/// here recognises them.
///
/// The one exception is a helper function identical to the one
/// [`add_used_helpers`] emits, and `call`s of it: that is a module this
/// pass (or a test using this module's builders) already lowered, and
/// lowering it again must keep working.
///
/// ```text
/// (define (__lisp_equal a b) 42)    → Err: defines "__lisp_equal"
/// main: call __lisp_not, x          → Err: "main" calls "__lisp_not"
/// main: call __lisp_equal, a, b     → Ok only if the module holds the
///   + the compiler's __lisp_equal     compiler's own __lisp_equal
/// ```
pub fn check_reserved_names(module: &IIRModule) -> Result<(), ReservedNameError> {
    let mut own: Vec<&str> = Vec::new();
    for f in &module.functions {
        if !RESERVED_NAMES.contains(&f.name.as_str()) {
            continue;
        }
        if !is_compiler_helper(f) {
            return Err(ReservedNameError::defined(&f.name));
        }
        own.push(f.name.as_str());
    }
    for f in &module.functions {
        for i in &f.instructions {
            if i.op != "call" {
                continue;
            }
            if let Some(Operand::Var(callee)) = i.srcs.first() {
                if RESERVED_NAMES.contains(&callee.as_str()) && !own.contains(&callee.as_str()) {
                    return Err(ReservedNameError { name: callee.clone(), caller: Some(f.name.clone()) });
                }
            }
        }
    }
    Ok(())
}

/// The helper function this module emits for `name` (one of [`HELPERS`]).
fn build_helper(name: &str) -> IIRFunction {
    if name == EQUAL {
        build_equal()
    } else {
        build_exit_code()
    }
}

/// Is `f` exactly the helper [`add_used_helpers`] emits under its name —
/// same parameters, return type and instructions (opcode, destination,
/// operands, type hint, allocation flag)? Profiling state is ignored.
pub fn is_compiler_helper(f: &IIRFunction) -> bool {
    if !HELPERS.contains(&f.name.as_str()) {
        return false;
    }
    let want = build_helper(&f.name);
    let same_instr = |a: &IIRInstr, b: &IIRInstr| {
        a.op == b.op && a.dest == b.dest && a.srcs == b.srcs && a.type_hint == b.type_hint && a.may_alloc == b.may_alloc
    };
    f.params == want.params
        && f.return_type == want.return_type
        && f.instructions.len() == want.instructions.len()
        && f.instructions.iter().zip(&want.instructions).all(|(a, b)| same_instr(a, b))
}

// ---------------------------------------------------------------------------
// Tiny instruction builders, so each rule reads as the program it is
// ---------------------------------------------------------------------------

fn konst(dest: &str, n: i64) -> IIRInstr {
    IIRInstr::new("const", Some(dest.into()), vec![Operand::Int(n)], WORD)
}
fn builtin(name: &str, dest: &str, args: &[&str], ty: &str) -> IIRInstr {
    let mut srcs = vec![Operand::Var(name.into())];
    srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
    IIRInstr::new("call_builtin", Some(dest.into()), srcs, ty)
}
fn binop(op: &str, dest: &str, a: &str, b: &str) -> IIRInstr {
    IIRInstr::new(op, Some(dest.into()), vec![Operand::Var(a.into()), Operand::Var(b.into())], WORD)
}
fn mov(dest: &str, src: &str) -> IIRInstr {
    IIRInstr::new("mov", Some(dest.into()), vec![Operand::Var(src.into())], WORD)
}
fn jmp_if_false(cond: &str, label: &str) -> IIRInstr {
    IIRInstr::new("jmp_if_false", None, vec![Operand::Var(cond.into()), Operand::Var(label.into())], "void")
}
fn jmp(label: &str) -> IIRInstr {
    IIRInstr::new("jmp", None, vec![Operand::Var(label.into())], "void")
}
fn label(name: &str) -> IIRInstr {
    IIRInstr::new("label", None, vec![Operand::Var(name.into())], "void")
}
fn ret(v: &str, ty: &str) -> IIRInstr {
    IIRInstr::new("ret", None, vec![Operand::Var(v.into())], ty)
}

/// `dest = (a <op> k)` as a 0/1 word, `a` and the constant `k` compared as
/// `i64` — two instructions.
fn cmp_const(op: &str, dest: &str, a: &str, k: i64) -> [IIRInstr; 2] {
    let k_reg = format!("{dest}.k");
    [konst(&k_reg, k), binop(op, dest, a, &k_reg)]
}

// ---------------------------------------------------------------------------
// The helper functions
// ---------------------------------------------------------------------------

/// ```text
/// fn __lisp_equal(a: ref<any>, b: ref<any>) -> ref<any> {
///     r = #f
///     a_pair = dyn_tag a == 7;  b_pair = dyn_tag b == 7
///     if a_pair: goto pairs
///     if b_pair: goto done            ; atom vs pair: #f
///     if a == b: r = #t               ; atoms: identical words
///     goto done
/// pairs:
///     if !b_pair: goto done           ; pair vs atom: #f
///     if __lisp_equal(first a, first b) != #t: goto done
///     r = __lisp_equal(rest a, rest b)
/// done:
///     return r
/// }
/// ```
///
/// Atoms compare as words: an integer's word is `n << 3`, a symbol's carries
/// its interned id, and nil, `#f` and `#t` are single words, so two atoms are
/// `equal?` exactly when their words are. The recursion is as deep as the
/// structure, as the runtime's C version was.
fn build_equal() -> IIRFunction {
    let mut b = vec![
        konst("r", WORD_FALSE),
        builtin("dyn_tag", "ta", &["a"], WORD),
        builtin("dyn_tag", "tb", &["b"], WORD),
    ];
    b.extend(cmp_const("cmp_eq", "a_pair", "ta", TAG_PAIR));
    b.extend(cmp_const("cmp_eq", "b_pair", "tb", TAG_PAIR));
    b.push(jmp_if_false("a_pair", "a_atom"));
    b.push(jmp("pairs"));
    // a is an atom
    b.push(label("a_atom"));
    b.push(jmp_if_false("b_pair", "atoms"));
    b.push(jmp("done"));
    b.push(label("atoms"));
    // The two words, compared as words (`i64`): an operand type of
    // `ref<any>` would ask the native backends for a reference comparison.
    b.push(binop("cmp_eq", "same", "a", "b"));
    b.push(jmp_if_false("same", "done"));
    b.push(konst("w_true", WORD_TRUE));
    b.push(mov("r", "w_true"));
    b.push(jmp("done"));
    // a is a pair
    b.push(label("pairs"));
    b.push(jmp_if_false("b_pair", "done"));
    b.push(builtin("dyn_pair_first", "fa", &["a"], DYN));
    b.push(builtin("dyn_pair_first", "fb", &["b"], DYN));
    b.push(call("firsts", EQUAL, &["fa", "fb"], DYN));
    b.push(builtin("dyn_tag", "tf", &["firsts"], WORD));
    b.extend(cmp_const("cmp_eq", "firsts_equal", "tf", TAG_TRUE));
    b.push(jmp_if_false("firsts_equal", "done"));
    b.push(builtin("dyn_pair_rest", "ra", &["a"], DYN));
    b.push(builtin("dyn_pair_rest", "rb", &["b"], DYN));
    b.push(call("rests", EQUAL, &["ra", "rb"], DYN));
    b.push(mov("r", "rests"));
    b.push(label("done"));
    b.push(ret("r", DYN));
    IIRFunction::new(EQUAL, vec![("a".into(), DYN.into()), ("b".into(), DYN.into())], DYN, b)
}

/// ```text
/// fn __lisp_exit_code(x: ref<any>) -> i64 {
///     t = dyn_tag x
///     r = x                           ; symbol / pair: the word itself
///     if t == 0: r = dyn_unbox_int x  ; integer: its value
///     if t == 5: r = 1                ; #t
///     if t == 3 or t == 1: r = 0      ; #f, nil
///     return r
/// }
/// ```
///
/// The coercion for a result whose tag is known only at run time. A result
/// known to be an integer is unboxed and one known to be a boolean is made
/// truthy at the call site instead; both agree with this.
fn build_exit_code() -> IIRFunction {
    let mut b = vec![builtin("dyn_tag", "t", &["x"], WORD), mov("r", "x")];
    b.extend(cmp_const("cmp_eq", "is_int", "t", TAG_INT));
    b.push(jmp_if_false("is_int", "not_int"));
    b.push(builtin("dyn_unbox_int", "n", &["x"], WORD));
    b.push(mov("r", "n"));
    b.push(jmp("done"));
    b.push(label("not_int"));
    b.extend(cmp_const("cmp_eq", "is_true", "t", TAG_TRUE));
    b.push(jmp_if_false("is_true", "not_true"));
    b.push(konst("one", 1));
    b.push(mov("r", "one"));
    b.push(jmp("done"));
    b.push(label("not_true"));
    b.extend(cmp_const("cmp_eq", "is_false", "t", TAG_FALSE));
    b.push(jmp_if_false("is_false", "not_false"));
    b.push(jmp("zero"));
    b.push(label("not_false"));
    b.extend(cmp_const("cmp_eq", "is_nil", "t", TAG_NIL));
    b.push(jmp_if_false("is_nil", "done"));
    b.push(label("zero"));
    b.push(konst("z", 0));
    b.push(mov("r", "z"));
    b.push(label("done"));
    b.push(ret("r", WORD));
    IIRFunction::new(EXIT_CODE, vec![("x".into(), DYN.into())], WORD, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::op_registry::{registry_entry, OpFamily};
    use std::collections::HashMap;

    // ── A tiny reference machine for the helpers ──────────────────────────
    //
    // The helpers use a dozen instructions, so the tests run them here
    // directly, over 64-bit words and a toy pair heap: a pair's word is
    // `(index << 3) | 7`. Each helper's answer is compared with Lisp's rule
    // stated independently below (`oracle_*`), over every combination of a
    // set of values that covers every tag, and pairs that are equal in
    // structure but not in identity.

    struct Machine<'m> {
        module: &'m IIRModule,
        heap: Vec<(u64, u64)>,
        steps: usize,
    }

    impl Machine<'_> {
        fn pair(&mut self, a: u64, b: u64) -> u64 {
            self.heap.push((a, b));
            (((self.heap.len() - 1) as u64) << 3) | 7
        }

        fn run(&mut self, name: &str, args: &[u64]) -> u64 {
            let module: &IIRModule = self.module;
            let f = module.functions.iter().find(|f| f.name == name).expect("function");
            let mut regs: HashMap<String, u64> =
                f.params.iter().map(|(p, _)| p.clone()).zip(args.iter().copied()).collect();
            let labels: HashMap<String, usize> = f
                .instructions
                .iter()
                .enumerate()
                .filter(|(_, i)| i.op == "label")
                .map(|(k, i)| (var(&i.srcs[0]).to_string(), k))
                .collect();
            let mut pc = 0;
            loop {
                self.steps += 1;
                assert!(self.steps < 1_000_000, "runaway helper");
                let i = &f.instructions[pc];
                pc += 1;
                let get = |regs: &HashMap<String, u64>, k: usize| regs[var(&i.srcs[k])];
                match i.op.as_str() {
                    "const" => {
                        let Operand::Int(n) = i.srcs[0] else { panic!("const {i:?}") };
                        regs.insert(dest(i), n as u64);
                    }
                    "mov" => {
                        let v = get(&regs, 0);
                        regs.insert(dest(i), v);
                    }
                    "cmp_eq" | "cmp_ne" | "mul" | "sub" => {
                        let (a, b) = (get(&regs, 0), get(&regs, 1));
                        let v = match i.op.as_str() {
                            "cmp_eq" => (a == b) as u64,
                            "cmp_ne" => (a != b) as u64,
                            "mul" => a.wrapping_mul(b),
                            _ => a.wrapping_sub(b),
                        };
                        regs.insert(dest(i), v);
                    }
                    "call_builtin" => {
                        let x = get(&regs, 1);
                        let v = match var(&i.srcs[0]) {
                            "dyn_tag" => x & 7,
                            "dyn_unbox_int" => ((x as i64) >> 3) as u64,
                            "dyn_pair_first" => self.heap[(x >> 3) as usize].0,
                            "dyn_pair_rest" => self.heap[(x >> 3) as usize].1,
                            other => panic!("unexpected builtin {other}"),
                        };
                        regs.insert(dest(i), v);
                    }
                    "call" => {
                        let args: Vec<u64> = (1..i.srcs.len()).map(|k| get(&regs, k)).collect();
                        let v = self.run(var(&i.srcs[0]), &args);
                        regs.insert(dest(i), v);
                    }
                    "jmp" => pc = labels[var(&i.srcs[0])],
                    "jmp_if_false" => {
                        if get(&regs, 0) == 0 {
                            pc = labels[var(&i.srcs[1])];
                        }
                    }
                    "label" => {}
                    "ret" => return get(&regs, 0),
                    other => panic!("unexpected op {other}"),
                }
            }
        }
    }

    fn var(o: &Operand) -> &str {
        match o {
            Operand::Var(v) => v,
            other => panic!("expected a variable, got {other:?}"),
        }
    }
    fn dest(i: &IIRInstr) -> String {
        i.dest.clone().expect("dest")
    }

    const NIL: u64 = 1;
    const FALSE: u64 = 3;
    const TRUE: u64 = 5;
    fn int(n: i64) -> u64 {
        (n << 3) as u64
    }
    fn sym(id: u64) -> u64 {
        (id << 32) | 2
    }

    // Lisp's rules, stated directly over the words (the runtime's former C).
    //
    // `__dyn_truthy` was `(v == LISPY_FALSE || v == LISPY_NIL) ? 0 : 1` — a
    // test of the whole word, so a malformed word with nil's or #f's tag
    // (a raw 9 or 11) is true. The oracle keeps that rule.
    fn oracle_truthy(x: u64) -> u64 {
        (x != FALSE && x != NIL) as u64
    }
    fn oracle_equal(m: &Machine<'_>, a: u64, b: u64) -> bool {
        let (pa, pb) = (a & 7 == 7, b & 7 == 7);
        match (pa, pb) {
            (false, false) => a == b,
            (true, true) => {
                let (x, y) = (m.heap[(a >> 3) as usize], m.heap[(b >> 3) as usize]);
                oracle_equal(m, x.0, y.0) && oracle_equal(m, x.1, y.1)
            }
            _ => false,
        }
    }
    fn oracle_exit(x: u64) -> u64 {
        match x & 7 {
            0 => ((x as i64) >> 3) as u64,
            5 => 1,
            3 | 1 => 0,
            _ => x,
        }
    }

    /// A module with one function per rule (`truthy`, `not`, `equal`, `exit`),
    /// lowered exactly as the pass lowers a module: the inline rules
    /// expanded, the helper functions appended.
    fn all_rules() -> IIRModule {
        let mut m = IIRModule::new("m", "test");
        let x = || vec![("x".to_string(), DYN.to_string())];
        m.add_or_replace(IIRFunction::new("truthy", x(), WORD, vec![call("r", TRUTHY, &["x"], WORD), ret("r", WORD)]));
        m.add_or_replace(IIRFunction::new("not", x(), DYN, vec![call("r", NOT, &["x"], DYN), ret("r", DYN)]));
        m.add_or_replace(IIRFunction::new(
            "equal",
            vec![("x".into(), DYN.into()), ("y".into(), DYN.into())],
            DYN,
            vec![call("r", EQUAL, &["x", "y"], DYN), ret("r", DYN)],
        ));
        m.add_or_replace(IIRFunction::new("exit", x(), WORD, vec![call("r", EXIT_CODE, &["x"], WORD), ret("r", WORD)]));
        for f in &mut m.functions {
            expand_inline_rules(f);
        }
        add_used_helpers(&mut m).expect("no program function carries a helper's name");
        m
    }

    /// Every kind of value, plus pairs equal in structure but not identity.
    fn values(m: &mut Machine<'_>) -> Vec<u64> {
        let mut v = vec![int(0), int(7), int(-3), int(1 << 40), NIL, FALSE, TRUE, sym(0), sym(1)];
        let p12 = m.pair(int(1), int(2));
        let p12b = m.pair(int(1), int(2));
        let p13 = m.pair(int(1), int(3));
        let inner = m.pair(int(2), NIL);
        let list = m.pair(int(1), inner);
        let inner_b = m.pair(int(2), NIL);
        let list_b = m.pair(int(1), inner_b);
        let nested = m.pair(list, sym(0));
        let nested_b = m.pair(list_b, sym(0));
        let nested_c = m.pair(list_b, sym(1));
        v.extend([p12, p12b, p13, list, list_b, nested, nested_b, nested_c]);
        v
    }

    #[test]
    fn helpers_agree_with_lisp_rules_over_every_kind_of_value() {
        let module = all_rules();
        let mut m = Machine { module: &module, heap: Vec::new(), steps: 0 };
        let vals = values(&mut m);
        for &x in &vals {
            assert_eq!(m.run("truthy", &[x]), oracle_truthy(x), "truthy {x:#x}");
            let want_not = if oracle_truthy(x) == 1 { FALSE } else { TRUE };
            assert_eq!(m.run("not", &[x]), want_not, "not {x:#x}");
            assert_eq!(m.run("exit", &[x]), oracle_exit(x), "exit code {x:#x}");
            for &y in &vals {
                let want = if oracle_equal(&m, x, y) { TRUE } else { FALSE };
                assert_eq!(m.run("equal", &[x, y]), want, "equal {x:#x} {y:#x}");
            }
        }
        // The integer 0 is true, as in every Lisp.
        assert_eq!(m.run("truthy", &[int(0)]), 1);
        assert_eq!(m.run("exit", &[int(-3)]) as i64, -3);
    }

    /// Malformed words — raw machine integers that reached a tagged position
    /// unboxed. Their low three bits look like a tag (9 and 17 like nil's
    /// `001`, 11 and 19 like `#f`'s `011`, 7 and 15 like a pair's `111`, 13
    /// like `#t`'s `101`) but only the words 1 and 3 are nil and `#f`.
    const MALFORMED: [u64; 9] = [9, 11, 17, 19, 13, 25, 27, 0x1_0000_0001, 0x1_0000_0003];

    /// Security review of PR7f-2: truthiness and `not` answer exactly what the
    /// runtime's `__dyn_truthy` / `__dyn_not` answered for a malformed word —
    /// true, so `not` is `#f` — where a tag-only test would call 9 and 11
    /// false. The exit code still switches on the tag, as `__dyn_to_exit_code`
    /// did, and `equal?` of two such atoms still compares their words.
    #[test]
    fn rules_agree_with_the_runtime_on_malformed_words() {
        let module = all_rules();
        let mut m = Machine { module: &module, heap: Vec::new(), steps: 0 };
        for &x in &MALFORMED {
            assert_eq!(m.run("truthy", &[x]), 1, "truthy {x:#x}: only the words 1 and 3 are false");
            assert_eq!(m.run("truthy", &[x]), oracle_truthy(x), "truthy {x:#x}");
            assert_eq!(m.run("not", &[x]), FALSE, "not {x:#x}");
            assert_eq!(m.run("exit", &[x]), oracle_exit(x), "exit code {x:#x}");
            for &y in &MALFORMED {
                let want = if x == y { TRUE } else { FALSE };
                assert_eq!(m.run("equal", &[x, y]), want, "equal {x:#x} {y:#x}");
            }
        }
        // A raw word with the pair tag (7 here addresses the toy heap's pair
        // 0) is true, as any non-nil, non-#f word is.
        let p = m.pair(int(1), int(2));
        assert_eq!(p, 7);
        assert_eq!(m.run("truthy", &[p]), 1);
        assert_eq!(m.run("not", &[p]), FALSE);
        // And the two false words stay false.
        for x in [NIL, FALSE] {
            assert_eq!(m.run("truthy", &[x]), 0, "{x}");
            assert_eq!(m.run("not", &[x]), TRUE, "{x}");
        }
    }

    /// Truthiness is two whole-word comparisons typed `i64` and a `mul` — no
    /// `dyn_tag`, whose answer is the wrong question for a malformed word.
    #[test]
    fn truthiness_compares_the_whole_word_as_i64() {
        let module = all_rules();
        let f = module.functions.iter().find(|f| f.name == "truthy").unwrap();
        let ops: Vec<&str> = f.instructions.iter().map(|i| i.op.as_str()).collect();
        assert_eq!(ops, ["const", "cmp_ne", "const", "cmp_ne", "mul", "ret"]);
        for i in f.instructions.iter().filter(|i| i.op == "cmp_ne") {
            assert_eq!(i.type_hint, WORD, "{i:?}");
            assert_eq!(i.srcs[0], Operand::Var("x".into()), "the word itself is compared: {i:?}");
        }
    }

    /// `add_used_helpers` never keeps a program's function in a helper's
    /// place, and `check_reserved_names` refuses a definition or a call of
    /// any of the [`RESERVED_NAMES`] the compiler did not write.
    #[test]
    fn reserved_names_are_refused_unless_the_compiler_wrote_them() {
        let impostor = IIRFunction::new(
            EQUAL,
            vec![("a".into(), DYN.into()), ("b".into(), DYN.into())],
            DYN,
            vec![konst("k", 42), ret("k", DYN)],
        );
        let mut m = IIRModule::new("m", "test");
        m.add_or_replace(IIRFunction::new(
            "main",
            vec![("x".into(), DYN.into())],
            DYN,
            vec![call("r", EQUAL, &["x", "x"], DYN), ret("r", DYN)],
        ));
        m.add_or_replace(impostor);
        let err = add_used_helpers(&mut m).expect_err("an impostor __lisp_equal");
        assert_eq!(err, ReservedNameError { name: EQUAL.into(), caller: None });
        assert_eq!(check_reserved_names(&m), Err(err));

        // A call with no definition at all is refused too: the compiler would
        // otherwise append its helper and bind the program's call to it.
        let mut m = IIRModule::new("m", "test");
        m.add_or_replace(IIRFunction::new("main", vec![], DYN, vec![call("r", NOT, &["x"], DYN), ret("r", DYN)]));
        assert_eq!(
            check_reserved_names(&m),
            Err(ReservedNameError { name: NOT.into(), caller: Some("main".into()) })
        );

        // The fully lowered module — the compiler's own helpers, called — passes.
        assert_eq!(check_reserved_names(&all_rules()), Ok(()));
        for helper in HELPERS {
            assert!(is_compiler_helper(&build_helper(helper)), "{helper}");
        }
        assert!(!is_compiler_helper(&IIRFunction::new("main", vec![], WORD, vec![])));
    }

    /// This pass runs on every module the tagged-word pipelines compile, Nib's
    /// and ALGOL's too. A function whose name merely begins with `__lisp_`
    /// collides with nothing the pass recognises, so it — and calls of it —
    /// must still compile: only the four [`RESERVED_NAMES`] are refused.
    #[test]
    fn other_lisp_prefixed_names_are_not_reserved() {
        let mut m = IIRModule::new("m", "nib");
        m.add_or_replace(IIRFunction::new("__lisp_helper", vec![], WORD, vec![konst("k", 4), ret("k", WORD)]));
        m.add_or_replace(IIRFunction::new(
            "main",
            vec![],
            WORD,
            vec![call("r", "__lisp_helper", &[], WORD), ret("r", WORD)],
        ));
        assert_eq!(check_reserved_names(&m), Ok(()));
        assert_eq!(RESERVED_NAMES, [TRUTHY, NOT, EQUAL, EXIT_CODE]);
        for name in RESERVED_NAMES {
            assert!(name.starts_with(RESERVED_PREFIX), "{name}");
        }
    }

    #[test]
    fn rules_use_only_registry_primitives_and_plain_control_flow() {
        let module = all_rules();
        for f in &module.functions {
            for i in &f.instructions {
                match i.op.as_str() {
                    "const" | "cmp_eq" | "cmp_ne" | "mul" | "sub" | "mov" | "jmp" | "jmp_if_false" | "label"
                    | "ret" => {}
                    "call_builtin" => {
                        let spec = registry_entry(i).unwrap_or_else(|| panic!("{}: {i:?}", f.name));
                        assert_eq!(spec.family, OpFamily::Dynamic, "{}: {}", f.name, spec.name);
                    }
                    // Only the two helper functions are ever called; the
                    // inline rules leave no call behind.
                    "call" => assert!(
                        matches!(rule_called(i), Some(EQUAL | EXIT_CODE)),
                        "{}: {i:?}",
                        f.name
                    ),
                    other => panic!("{}: unexpected op {other}", f.name),
                }
            }
            assert_eq!(f.instructions.iter().filter(|i| i.op == "ret").count(), 1, "{}: one ret", f.name);
        }
        let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["truthy", "not", "equal", "exit", EQUAL, EXIT_CODE]);
    }

    /// The inline rules are straight-line and typed `i64` throughout, so a
    /// function that uses them stays `FullyTyped` (the generic JIT compiles
    /// it), exactly as with the runtime call they replace.
    #[test]
    fn inline_rules_keep_a_function_straight_line_and_fully_typed() {
        use interpreter_ir::function::FunctionTypeStatus;
        let module = all_rules();
        for name in ["truthy", "not"] {
            let f = module.functions.iter().find(|f| f.name == name).unwrap();
            assert!(f.instructions.iter().all(|i| i.op != "call" && i.op != "label"), "{name}");
            assert_eq!(f.infer_type_status(), FunctionTypeStatus::FullyTyped, "{name}");
        }
    }

    #[test]
    fn helpers_are_added_once_and_only_when_called() {
        let mut empty = IIRModule::new("m", "test");
        empty.add_or_replace(IIRFunction::new("main", vec![], WORD, vec![konst("z", 0), ret("z", WORD)]));
        add_used_helpers(&mut empty).expect("nothing to add");
        assert_eq!(empty.functions.len(), 1, "a module that calls no helper gets none");

        let mut m = IIRModule::new("m", "test");
        m.add_or_replace(IIRFunction::new(
            "main",
            vec![("x".into(), DYN.into())],
            WORD,
            vec![call("a", EQUAL, &["x", "x"], DYN), call("b", EQUAL, &["a", "x"], DYN), ret("b", DYN)],
        ));
        add_used_helpers(&mut m).expect("first");
        add_used_helpers(&mut m).expect("again: the helper is the compiler's own");
        let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["main", EQUAL], "one copy, and no exit-code helper nobody calls");
    }

    /// The names belong to the Lisp lowering: no linked runtime's namespace
    /// (`__twig_*`, `__dyn_*`, `__gc_*`) and not the LLVM backend's own
    /// module helpers (`__iir_*`), so no runtime symbol can be bound in their
    /// place at link time.
    #[test]
    fn helper_names_avoid_runtime_namespaces() {
        for name in HELPERS.iter().chain(INLINE_RULES.iter()) {
            assert!(name.starts_with("__lisp_"), "{name}");
            for taken in ["__twig_", "__dyn_", "__gc_", "__iir_"] {
                assert!(!name.starts_with(taken), "{name} is in {taken}*");
            }
        }
    }
}
