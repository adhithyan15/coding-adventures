"""End-to-end tests for ``twig-jvm-compiler`` running on real ``java``.

These tests:

1. Compile Twig source to a real JVM ``.class`` file via the
   in-house ``ir-to-jvm-class-file`` pipeline.
2. Write the file to a fresh temp dir.
3. Invoke ``java -cp <tmp> <ClassName>`` as a subprocess.
4. Assert on the captured stdout bytes.

The tests skip cleanly when ``java`` isn't on PATH — same pattern
the existing ``test_oct_8bit_e2e.py`` uses for Oct-on-JVM tests.
"""

from __future__ import annotations

import pytest

from twig_jvm_compiler import java_available, run_source

requires_java = pytest.mark.skipif(
    not java_available(),
    reason="'java' binary not found on PATH",
)


@requires_java
def test_arithmetic_addition() -> None:
    """``(+ 1 2)`` writes byte 3 to stdout."""
    result = run_source("(+ 1 2)", class_name="TwigAdd")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([3])


@requires_java
def test_arithmetic_subtraction() -> None:
    result = run_source("(- 10 3)", class_name="TwigSub")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([7])


@requires_java
def test_arithmetic_multiplication() -> None:
    result = run_source("(* 6 7)", class_name="TwigMul")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([42])


@requires_java
def test_let_binding() -> None:
    result = run_source("(let ((x 5)) (* x x))", class_name="TwigLet")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([25])


@requires_java
def test_if_taken() -> None:
    result = run_source("(if (= 1 1) 100 200)", class_name="TwigIfT")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([100])


@requires_java
def test_if_not_taken() -> None:
    result = run_source("(if (= 1 2) 100 200)", class_name="TwigIfF")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([200])


@requires_java
def test_top_level_function_call() -> None:
    """``(define (square x) (* x x)) (square 7)`` writes byte 49."""
    src = "(define (square x) (* x x)) (square 7)"
    result = run_source(src, class_name="TwigSquare")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([49])


@requires_java
def test_function_with_two_params() -> None:
    """Verifies the param-slot calling convention for arity > 1."""
    src = "(define (add3 a b) (+ a (+ b 3))) (add3 10 20)"
    result = run_source(src, class_name="TwigAdd3")
    assert result.returncode == 0, result.stderr
    # 10 + 20 + 3 = 33
    assert result.stdout == bytes([33])


@requires_java
def test_recursion_factorial() -> None:
    """Recursive ``fact`` — the JVM01 fix preserves param values
    across a recursive call.  Two cooperating mechanisms:

    1. ``ir-to-jvm-class-file`` snapshots the static register array
       to JVM locals around every CALL and restores after (skipping
       r1, the return-value slot).
    2. ``twig-jvm-compiler`` copies each param out of its arrival
       slot into a body-local holding register at function entry,
       so call-site arg marshalling never clobbers the live param.

    ``5! = 120`` and 120 = 0x78 = ``b'x'``.
    """
    src = """
        (define (fact n)
          (if (= n 0) 1 (* n (fact (- n 1)))))
        (fact 5)
    """
    result = run_source(src, class_name="TwigFact")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([120])


@requires_java
def test_top_level_value_define_inlined() -> None:
    """``(define x 42)`` value-defines fold to compile-time constants."""
    src = "(define x 42) x"
    result = run_source(src, class_name="TwigVal")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([42])


@requires_java
def test_nested_function_calls() -> None:
    """``(f (g x))`` — the holding-register convention must
    survive a nested call without clobbering arg slots."""
    src = """
        (define (inc x) (+ x 1))
        (define (dbl x) (* x 2))
        (inc (dbl 5))
    """
    result = run_source(src, class_name="TwigNested")
    assert result.returncode == 0, result.stderr
    # dbl(5) = 10, inc(10) = 11
    assert result.stdout == bytes([11])


# ── Closures (JVM02 Phase 2d) ────────────────────────────────────────────


@requires_java
def test_closure_make_adder() -> None:
    """``((make-adder 7) 35) → 42`` — the headline JVM02 Phase 2d test.

    Exercises the full closure pipeline from real Twig source on
    real ``java -jar``:

    * Free-variable analysis lifts the ``(lambda (x) (+ x n))``
      to a top-level ``_lambda_0`` region.
    * ``MAKE_CLOSURE`` instantiates a ``Closure__lambda_0``
      subclass and stores it via the parallel ``Object[]
      __ca_objregs`` static pool (Phase 2c.5).
    * ``APPLY_CLOSURE`` dispatches via
      ``invokeinterface Closure.apply([I)I``.
    * The Closure subclass's ``apply`` forwards via
      ``invokestatic`` to ``TwigClosure._lambda_0(I, I)I`` — the
      lifted lambda lives as a public static method on the main
      class with widened arity (Phase 2c.5).
    * The Twig output convention writes the final value as a
      byte to stdout; ``42 == 0x2a == b'*'``.
    """
    src = """
        (define (make-adder n) (lambda (x) (+ x n)))
        ((make-adder 7) 35)
    """
    result = run_source(src, class_name="TwigClosure")
    assert result.returncode == 0, (
        f"closure pipeline broke at runtime.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout == b"*", (
        f"expected b'*' (= 42), got {result.stdout!r}"
    )


@requires_java
def test_closure_let_bound() -> None:
    """``(let ((adder (make-adder 7))) (adder 35)) → 42``.

    Exercises the let-bound closure path: ``adder`` is a local
    binding (not in ``_fn_params``) holding a closure value;
    the call ``(adder 35)`` falls through to APPLY_CLOSURE.
    """
    src = """
        (define (make-adder n) (lambda (x) (+ x n)))
        (let ((adder (make-adder 7))) (adder 35))
    """
    result = run_source(src, class_name="TwigLetClos")
    assert result.returncode == 0, result.stderr
    assert result.stdout == b"*"


# ── Heap primitives (TW03 Phase 3e) — real-java end-to-end ────────────────


@requires_java
def test_heap_list_of_ints_length() -> None:
    """``(length (cons 1 (cons 2 (cons 3 nil)))) → 3``.

    The headline TW03 Phase 3 acceptance criterion — Twig source
    with cons / cdr / null? + a recursive define compiles to a
    multi-class JAR and runs end-to-end on real ``java``,
    producing stdout = ``b'\\x03'``.

    Was xfail-strict in JVM Phase 3b because the obj-pool
    caller-saves were missing (recursion clobbered the cons ref
    in the obj register).  Now passes after the obj-pool
    caller-saves landed (mirrors the JVM01 fix that unblocked
    recursion through int registers)."""
    src = """
        (define (length xs)
          (if (null? xs) 0 (+ 1 (length (cdr xs)))))
        (length (cons 1 (cons 2 (cons 3 nil))))
    """
    result = run_source(src, class_name="TwigLength")
    assert result.returncode == 0, (
        f"length pipeline broke at runtime.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    # 3 → byte 0x03.
    assert result.stdout == b"\x03", (
        f"expected b'\\x03' (= 3), got {result.stdout!r}"
    )


@requires_java
def test_mutual_recursion_even_odd() -> None:
    """``(evp 8) → 1`` via mutually-recursive ``evp`` / ``odp``.

    Was a "Duplicate IR label" failure pre-fix because
    twig-jvm-compiler's ``_fresh_label`` used a per-region counter
    so ``evp``'s ``_else_0`` collided with ``odp``'s ``_else_0``.
    """
    src = """
        (define (evp n) (if (= n 0) 1 (odp (- n 1))))
        (define (odp n) (if (= n 0) 0 (evp (- n 1))))
        (evp 8)
    """
    result = run_source(src, class_name="TwMutEvenOdd")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([1])


@requires_java
def test_three_deep_curry() -> None:
    """``(((mk2 a) b) c) → 42`` — closure that returns a closure
    that returns an int.

    Was a NullReferenceException pre-fix because APPLY_CLOSURE
    only stored its int result into ``__ca_regs[dst]``.  When the
    callee actually returned a closure ref, the lifted lambda's
    body had propagated it into ``__ca_objregs[1]`` via the
    obj-typed RET, but APPLY_CLOSURE didn't carry it onward to
    ``__ca_objregs[dst]`` — so the next ``APPLY_CLOSURE
    closure_reg=v13`` read null.

    Fix (in ir-to-jvm-class-file): after APPLY_CLOSURE, when the
    dst register is obj-typed in the current region, also copy
    ``__ca_objregs[1] → __ca_objregs[dst]``.  Mirror of the
    obj-pool caller-restore's "skip index 1" convention.
    """
    src = """
        (define (mk2 a) (lambda (b) (lambda (c) (+ a (+ b c)))))
        (((mk2 10) 20) 12)
    """
    result = run_source(src, class_name="TwCurry3")
    assert result.returncode == 0, result.stderr
    # 10 + 20 + 12 = 42 = 0x2a = b'*'
    assert result.stdout == b"*"


@requires_java
def test_let_bound_closure_called_twice() -> None:
    """``(let ((add5 (mk-adder 5))) (+ (add5 10) (add5 27))) → 47``.

    Was a NullReferenceException pre-fix because the JVM
    backend's ADD_IMM-0 obj-slot propagation fired
    unconditionally — the lifted lambda body's
    ``ADD_IMM v11, v3, 0`` (v3 is an int arg) would clobber
    ``__ca_objregs[11]`` (the caller's closure-holding slot)
    with null.  Fix: gate obj-slot propagation on per-region
    obj-typed register analysis.
    """
    src = """
        (define (mk-adder n) (lambda (x) (+ x n)))
        (let ((add5 (mk-adder 5)))
          (+ (add5 10) (add5 27)))
    """
    result = run_source(src, class_name="TwLetTwiceClos")
    assert result.returncode == 0, result.stderr
    # 15 + 32 = 47 = 0x2f = b'/'
    assert result.stdout == b"/"


@requires_java
def test_heap_car_of_singleton_returns_int() -> None:
    """``(car (cons 42 nil)) → 42`` exercises the ``CAR`` int-read
    path: the cons head is written from an int register and CAR
    reads it back into another int register, ready for SYSCALL
    output."""
    src = "(car (cons 42 nil))"
    result = run_source(src, class_name="TwigCarOne")
    assert result.returncode == 0, result.stderr
    # 42 = 0x2a = b'*'.
    assert result.stdout == b"*"


@requires_java
def test_heap_car_of_symbol_succeeds() -> None:
    """``(symbol? (car (cons 'foo nil))) → 1``.

    Heterogeneous-cons: head can hold a symbol (or any obj ref),
    not just int.  Pre-fix, ``Cons.head`` was typed ``int`` and
    storing a symbol ref into the int field truncated it to
    garbage; the subsequent ``symbol?`` instanceof check failed.

    Post-fix, head is ``Object``-typed.  MAKE_CONS detects the
    head register's obj-typing in this region and stores the
    Symbol ref directly; CAR's dst is obj-typed (because the
    next op reads it as obj-source for IS_SYMBOL) so it's
    loaded directly from the Object field.
    """
    src = "(if (symbol? (car (cons (quote foo) nil))) 1 0)"
    result = run_source(src, class_name="TwCarSym")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([1])


@requires_java
def test_heap_car_of_nested_cons_succeeds() -> None:
    """``(pair? (car (cons (cons 1 nil) nil))) → 1``.

    Heterogeneous-cons: head can hold another cons cell.  This
    is the canonical AST-shaped data pattern that any real Lisp
    program (including a self-hosted compiler) needs.
    """
    src = "(if (pair? (car (cons (cons 1 nil) nil))) 1 0)"
    result = run_source(src, class_name="TwCarPair")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([1])
