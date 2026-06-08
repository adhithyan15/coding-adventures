"""Comprehensive tests for vm-core.

Coverage targets: 95%+ across all modules.

Test organisation
-----------------
Each section targets a specific subsystem:

    TestRegisterFile        — RegisterFile slot operations
    TestVMFrame             — VMFrame construction, resolve, assign
    TestVMProfiler          — type observation and mapping
    TestBuiltinRegistry     — registration, call, error paths
    TestVMMetrics           — dataclass construction
    TestVMCoreInit          — constructor defaults and overrides
    TestExecuteArithmetic   — add/sub/mul/div/mod/neg
    TestExecuteBitwise      — and/or/xor/not/shl/shr
    TestExecuteComparisons  — cmp_eq/ne/lt/le/gt/ge
    TestExecuteControlFlow  — jmp, jmp_if_true, jmp_if_false, labels
    TestExecuteMemory       — load_mem, store_mem, load_reg, store_reg
    TestExecuteCalls        — call (interpreter path), call_builtin
    TestExecuteIO           — io_in, io_out
    TestExecuteCoercions    — cast, type_assert
    TestJITHandler          — register_jit_handler, priority, metrics
    TestMetricsAccumulation — instruction/frame/JIT hit counters
    TestInterrupt           — interrupt() signal delivery
    TestReset               — reset() clears execution state
    TestU8Wrap              — arithmetic wraps to 8 bits
    TestFrameOverflow       — FrameOverflowError on deep recursion
    TestErrors              — UnknownOpcodeError, UndefinedVariableError
    TestRepl                — incremental module + add_or_replace pattern
    TestMultiFunction       — cross-function calls, return values
"""

from __future__ import annotations

import pytest
from interpreter_ir import IIRFunction, IIRInstr, IIRModule

from vm_core import (
    BuiltinRegistry,
    FrameOverflowError,
    RegisterFile,
    UndefinedVariableError,
    UnknownOpcodeError,
    VMCore,
    VMError,
    VMFrame,
    VMInterrupt,
    VMMetrics,
    VMProfiler,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _fn(
    name: str,
    params: list[tuple[str, str]],
    *instrs: IIRInstr,
    return_type: str = "any",
) -> IIRFunction:
    """Build an IIRFunction with auto-computed register_count."""
    return IIRFunction(
        name=name,
        params=params,
        return_type=return_type,
        instructions=list(instrs),
        register_count=max(8, len(params) + len(instrs)),
    )


def _mod(*fns: IIRFunction) -> IIRModule:
    return IIRModule(name="test", functions=list(fns))


def _i(op: str, dest: str | None = None, srcs: list | None = None,
       type_hint: str = "any") -> IIRInstr:
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


def _const(dest: str, value) -> IIRInstr:
    return _i("const", dest=dest, srcs=[value])


def _ret(src: str) -> IIRInstr:
    return _i("ret", srcs=[src])


def _ret_void() -> IIRInstr:
    return _i("ret_void")


def _simple_fn(name: str, *instrs: IIRInstr) -> IIRFunction:
    return _fn(name, [], *instrs)


def run(fn: IIRFunction, args: list | None = None, **kwargs) -> object:
    """Convenience: create VMCore, execute a single-function module."""
    vm = VMCore(**kwargs)
    mod = _mod(fn)
    return vm.execute(mod, fn=fn.name, args=args)


# ---------------------------------------------------------------------------
# TestRegisterFile
# ---------------------------------------------------------------------------

class TestRegisterFile:
    def test_initial_slots_are_zero(self):
        rf = RegisterFile(4)
        for i in range(4):
            assert rf[i] == 0

    def test_set_and_get(self):
        rf = RegisterFile(4)
        rf[2] = 42
        assert rf[2] == 42

    def test_len(self):
        rf = RegisterFile(6)
        assert len(rf) == 6

    def test_snapshot_is_copy(self):
        rf = RegisterFile(3)
        rf[0] = 10
        rf[1] = 20
        rf[2] = 30
        snap = rf.snapshot()
        rf[0] = 99
        assert snap == [10, 20, 30]

    def test_restore(self):
        rf = RegisterFile(3)
        rf[0] = 1
        rf[1] = 2
        rf[2] = 3
        snap = rf.snapshot()
        rf[0] = 99
        rf.restore(snap)
        assert rf[0] == 1

    def test_reset_zeros_all_slots(self):
        rf = RegisterFile(4)
        rf[0] = 5
        rf[3] = 7
        rf.reset()
        for i in range(4):
            assert rf[i] == 0


# ---------------------------------------------------------------------------
# TestVMFrame
# ---------------------------------------------------------------------------

class TestVMFrame:
    def _make_fn(self) -> IIRFunction:
        return _fn("f", [("x", "u8"), ("y", "u8")])

    def test_for_function_assigns_params(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        assert frame.name_to_reg == {"x": 0, "y": 1}

    def test_resolve_literal(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        assert frame.resolve(42) == 42
        assert frame.resolve(3.14) == 3.14
        assert frame.resolve(True) is True

    def test_resolve_variable(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        frame.registers[0] = 10
        assert frame.resolve("x") == 10

    def test_resolve_undefined_raises(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        with pytest.raises(UndefinedVariableError, match="z"):
            frame.resolve("z")

    def test_assign_new_variable(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        frame.assign("z", 99)
        assert frame.resolve("z") == 99

    def test_assign_overwrites(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        frame.registers[0] = 10
        frame.assign("x", 20)
        assert frame.resolve("x") == 20

    def test_return_dest_stored(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn, return_dest=5)
        assert frame.return_dest == 5

    def test_ip_starts_at_zero(self):
        fn = self._make_fn()
        frame = VMFrame.for_function(fn)
        assert frame.ip == 0


# ---------------------------------------------------------------------------
# TestVMProfiler
# ---------------------------------------------------------------------------

class TestVMProfiler:
    def _make_instr(self, type_hint="any") -> IIRInstr:
        return IIRInstr(op="add", dest="r", srcs=["a", "b"], type_hint=type_hint)

    def test_observe_bool(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, True)
        assert instr.observed_type == "bool"

    def test_observe_small_int_u8(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, 100)
        assert instr.observed_type == "u8"

    def test_observe_u16(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, 1000)
        assert instr.observed_type == "u16"

    def test_observe_u32(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, 100_000)
        assert instr.observed_type == "u32"

    def test_observe_large_int_u64(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, 2**33)
        assert instr.observed_type == "u64"

    def test_observe_float(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, 3.14)
        assert instr.observed_type == "f64"

    def test_observe_str(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, "hello")
        assert instr.observed_type == "str"

    def test_observe_unknown(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, [1, 2, 3])
        assert instr.observed_type == "any"

    def test_skips_typed_instructions(self):
        p = VMProfiler()
        instr = self._make_instr(type_hint="u8")
        p.observe(instr, 42)
        assert instr.observed_type is None
        assert p.total_observations == 0

    def test_total_observations_increments(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, 1)
        p.observe(instr, 2)
        assert p.total_observations == 2

    def test_negative_int_maps_to_u64(self):
        p = VMProfiler()
        instr = self._make_instr()
        p.observe(instr, -1)
        assert instr.observed_type == "u64"


# ---------------------------------------------------------------------------
# TestBuiltinRegistry
# ---------------------------------------------------------------------------

class TestBuiltinRegistry:
    def test_noop_pre_registered(self):
        r = BuiltinRegistry()
        assert r.is_registered("noop")
        assert r.call("noop", []) is None

    def test_assert_eq_passes(self):
        r = BuiltinRegistry()
        assert r.call("assert_eq", [42, 42]) is None

    def test_assert_eq_fails(self):
        r = BuiltinRegistry()
        with pytest.raises(AssertionError, match="assert_eq"):
            r.call("assert_eq", [1, 2])

    def test_register_and_call(self):
        r = BuiltinRegistry()
        r.register("double", lambda args: args[0] * 2)
        assert r.call("double", [5]) == 10

    def test_call_unknown_raises_key_error(self):
        r = BuiltinRegistry()
        with pytest.raises(KeyError, match="undefined builtin"):
            r.call("undefined", [])

    def test_is_registered_false(self):
        r = BuiltinRegistry()
        assert not r.is_registered("missing")

    def test_registered_names_includes_builtins(self):
        r = BuiltinRegistry()
        names = r.registered_names()
        assert "noop" in names
        assert "assert_eq" in names

    def test_register_overwrites(self):
        r = BuiltinRegistry()
        r.register("noop", lambda _: 42)
        assert r.call("noop", []) == 42


# ---------------------------------------------------------------------------
# TestVMMetrics
# ---------------------------------------------------------------------------

class TestVMMetrics:
    def test_defaults(self):
        m = VMMetrics()
        assert m.function_call_counts == {}
        assert m.total_instructions_executed == 0
        assert m.total_frames_pushed == 0
        assert m.total_jit_hits == 0

    def test_snapshot_is_independent(self):
        vm = VMCore()
        fn = _simple_fn("main", _const("x", 1), _ret("x"))
        vm.execute(_mod(fn), fn="main")
        m = vm.metrics()
        # Modifying the snapshot does not affect the VM's internal state.
        m.function_call_counts["main"] = 999
        m2 = vm.metrics()
        assert m2.function_call_counts["main"] == 1


# ---------------------------------------------------------------------------
# TestVMCoreInit
# ---------------------------------------------------------------------------

class TestVMCoreInit:
    def test_defaults(self):
        vm = VMCore()
        assert vm.u8_wrap is False
        assert vm.profiler_enabled is True
        assert not vm.is_executing

    def test_u8_wrap(self):
        vm = VMCore(u8_wrap=True)
        assert vm.u8_wrap is True

    def test_profiler_disabled(self):
        vm = VMCore(profiler_enabled=False)
        assert vm.profiler_enabled is False

    def test_profiler_enabled_setter(self):
        vm = VMCore(profiler_enabled=True)
        vm.profiler_enabled = False
        assert vm.profiler_enabled is False

    def test_custom_builtins(self):
        r = BuiltinRegistry()
        r.register("custom", lambda _: 77)
        vm = VMCore(builtins=r)
        assert vm.builtins.call("custom", []) == 77

    def test_execute_unknown_fn_raises(self):
        vm = VMCore()
        mod = _mod(_simple_fn("main", _ret_void()))
        with pytest.raises(KeyError, match="missing"):
            vm.execute(mod, fn="missing")


# ---------------------------------------------------------------------------
# TestExecuteArithmetic
# ---------------------------------------------------------------------------

class TestExecuteArithmetic:
    def test_add(self):
        fn = _simple_fn("main",
            _const("a", 3), _const("b", 4),
            _i("add", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 7

    def test_sub(self):
        fn = _simple_fn("main",
            _const("a", 10), _const("b", 3),
            _i("sub", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 7

    def test_mul(self):
        fn = _simple_fn("main",
            _const("a", 6), _const("b", 7),
            _i("mul", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 42

    def test_div(self):
        fn = _simple_fn("main",
            _const("a", 10), _const("b", 3),
            _i("div", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 3

    def test_mod(self):
        fn = _simple_fn("main",
            _const("a", 10), _const("b", 3),
            _i("mod", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 1

    def test_neg(self):
        fn = _simple_fn("main",
            _const("a", 5),
            _i("neg", "b", ["a"]), _ret("b"))
        assert run(fn) == -5

    def test_u8_wrap_add(self):
        fn = _simple_fn("main",
            _const("a", 250), _const("b", 10),
            _i("add", "c", ["a", "b"]), _ret("c"))
        assert run(fn, u8_wrap=True) == 4  # 260 & 0xFF

    def test_u8_wrap_sub(self):
        fn = _simple_fn("main",
            _const("a", 0), _const("b", 1),
            _i("sub", "c", ["a", "b"]), _ret("c"))
        assert run(fn, u8_wrap=True) == 255  # -1 & 0xFF

    def test_u8_wrap_mul(self):
        fn = _simple_fn("main",
            _const("a", 16), _const("b", 16),
            _i("mul", "c", ["a", "b"]), _ret("c"))
        assert run(fn, u8_wrap=True) == 0  # 256 & 0xFF

    def test_u8_wrap_neg(self):
        fn = _simple_fn("main",
            _const("a", 1),
            _i("neg", "b", ["a"]), _ret("b"))
        assert run(fn, u8_wrap=True) == 255

    def test_no_dest_add(self):
        # Handler still returns value; frame not updated.
        fn = _simple_fn("main",
            _const("a", 5), _const("b", 3),
            _i("add", None, ["a", "b"]),
            _const("r", 0), _ret("r"))
        assert run(fn) == 0


# ---------------------------------------------------------------------------
# TestExecuteBitwise
# ---------------------------------------------------------------------------

class TestExecuteBitwise:
    def test_and(self):
        fn = _simple_fn("main",
            _const("a", 0b1010), _const("b", 0b1100),
            _i("and", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 0b1000

    def test_or(self):
        fn = _simple_fn("main",
            _const("a", 0b1010), _const("b", 0b0101),
            _i("or", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 0b1111

    def test_xor(self):
        fn = _simple_fn("main",
            _const("a", 0b1010), _const("b", 0b1100),
            _i("xor", "c", ["a", "b"]), _ret("c"))
        assert run(fn) == 0b0110

    def test_not(self):
        fn = _simple_fn("main",
            _const("a", 0),
            _i("not", "b", ["a"]), _ret("b"))
        assert run(fn) == -1

    def test_shl(self):
        fn = _simple_fn("main",
            _const("a", 1), _const("n", 4),
            _i("shl", "c", ["a", "n"]), _ret("c"))
        assert run(fn) == 16

    def test_shr(self):
        fn = _simple_fn("main",
            _const("a", 16), _const("n", 2),
            _i("shr", "c", ["a", "n"]), _ret("c"))
        assert run(fn) == 4


# ---------------------------------------------------------------------------
# TestExecuteComparisons
# ---------------------------------------------------------------------------

class TestExecuteComparisons:
    def _cmp(self, op, a, b):
        fn = _simple_fn("main",
            _const("a", a), _const("b", b),
            _i(op, "c", ["a", "b"]), _ret("c"))
        return run(fn)

    def test_cmp_eq_true(self):
        assert self._cmp("cmp_eq", 5, 5) is True

    def test_cmp_eq_false(self):
        assert self._cmp("cmp_eq", 5, 6) is False

    def test_cmp_ne_true(self):
        assert self._cmp("cmp_ne", 1, 2) is True

    def test_cmp_ne_false(self):
        assert self._cmp("cmp_ne", 2, 2) is False

    def test_cmp_lt_true(self):
        assert self._cmp("cmp_lt", 3, 5) is True

    def test_cmp_lt_false(self):
        assert self._cmp("cmp_lt", 5, 3) is False

    def test_cmp_le_equal(self):
        assert self._cmp("cmp_le", 5, 5) is True

    def test_cmp_le_less(self):
        assert self._cmp("cmp_le", 4, 5) is True

    def test_cmp_le_greater(self):
        assert self._cmp("cmp_le", 6, 5) is False

    def test_cmp_gt_true(self):
        assert self._cmp("cmp_gt", 7, 3) is True

    def test_cmp_gt_false(self):
        assert self._cmp("cmp_gt", 2, 3) is False

    def test_cmp_ge_equal(self):
        assert self._cmp("cmp_ge", 5, 5) is True

    def test_cmp_ge_greater(self):
        assert self._cmp("cmp_ge", 6, 5) is True

    def test_cmp_ge_less(self):
        assert self._cmp("cmp_ge", 4, 5) is False


# ---------------------------------------------------------------------------
# TestExecuteControlFlow
# ---------------------------------------------------------------------------

class TestExecuteControlFlow:
    def test_label_is_noop(self):
        # Labels don't change execution flow on their own.
        fn = _simple_fn("main",
            _i("label", None, ["start"]),
            _const("r", 42),
            _ret("r"))
        assert run(fn) == 42

    def test_jmp_unconditional(self):
        # Jump over the "bad" const to the "good" one.
        fn = _simple_fn("main",
            _i("jmp", None, ["done"]),
            _const("r", 0),      # skipped
            _i("label", None, ["done"]),
            _const("r", 99),
            _ret("r"))
        assert run(fn) == 99

    def test_jmp_if_true_taken(self):
        fn = _simple_fn("main",
            _const("cond", True),
            _i("jmp_if_true", None, ["cond", "yes"]),
            _const("r", 0),
            _i("label", None, ["yes"]),
            _const("r", 1),
            _ret("r"))
        assert run(fn) == 1

    def test_jmp_if_true_not_taken(self):
        fn = _simple_fn("main",
            _const("cond", False),
            _i("jmp_if_true", None, ["cond", "yes"]),
            _const("r", 0),
            _i("jmp", None, ["done"]),
            _i("label", None, ["yes"]),
            _const("r", 1),
            _i("label", None, ["done"]),
            _ret("r"))
        assert run(fn) == 0

    def test_jmp_if_false_taken(self):
        fn = _simple_fn("main",
            _const("cond", False),
            _i("jmp_if_false", None, ["cond", "no"]),
            _const("r", 0),
            _i("label", None, ["no"]),
            _const("r", 2),
            _ret("r"))
        assert run(fn) == 2

    def test_jmp_if_false_not_taken(self):
        fn = _simple_fn("main",
            _const("cond", True),
            _i("jmp_if_false", None, ["cond", "no"]),
            _const("r", 5),
            _i("jmp", None, ["done"]),
            _i("label", None, ["no"]),
            _const("r", 0),
            _i("label", None, ["done"]),
            _ret("r"))
        assert run(fn) == 5

    def test_ret_void_returns_none(self):
        fn = _simple_fn("main", _ret_void())
        assert run(fn) is None

    def test_function_falls_off_end_returns_none(self):
        # No ret instruction — frame is popped when ip exceeds instructions.
        fn = _simple_fn("main", _const("r", 7))
        assert run(fn) is None


# ---------------------------------------------------------------------------
# TestExecuteMemory
# ---------------------------------------------------------------------------

class TestExecuteMemory:
    def test_store_and_load_mem(self):
        fn = _simple_fn("main",
            _const("addr", 100), _const("val", 42),
            _i("store_mem", None, ["addr", "val"]),
            _i("load_mem", "r", ["addr"]),
            _ret("r"))
        assert run(fn) == 42

    def test_load_mem_unwritten_is_zero(self):
        fn = _simple_fn("main",
            _const("addr", 999),
            _i("load_mem", "r", ["addr"]),
            _ret("r"))
        assert run(fn) == 0

    def test_store_and_load_reg(self):
        fn = _simple_fn("main",
            _const("val", 55),
            _i("store_reg", None, [0, "val"]),
            _i("load_reg", "r", [0]),
            _ret("r"))
        assert run(fn) == 55

    def test_memory_persists_across_calls(self):
        vm = VMCore()
        fn = _simple_fn("main",
            _const("addr", 0), _const("val", 7),
            _i("store_mem", None, ["addr", "val"]),
            _ret_void())
        mod = _mod(fn)
        vm.execute(mod)
        assert vm.memory[0] == 7

    def test_io_out_and_io_in(self):
        fn = _simple_fn("main",
            _const("port", 3), _const("val", 0xAB),
            _i("io_out", None, ["port", "val"]),
            _i("io_in", "r", ["port"]),
            _ret("r"))
        assert run(fn) == 0xAB

    def test_io_in_unwritten_is_zero(self):
        fn = _simple_fn("main",
            _const("port", 42),
            _i("io_in", "r", ["port"]),
            _ret("r"))
        assert run(fn) == 0


# ---------------------------------------------------------------------------
# TestExecuteCalls
# ---------------------------------------------------------------------------

class TestExecuteCalls:
    def test_call_simple_function(self):
        double = _fn("double", [("x", "u8")],
            _const("two", 2),
            _i("mul", "r", ["x", "two"]),
            _ret("r"))
        main = _simple_fn("main",
            _const("arg", 5),
            _i("call", "result", ["double", "arg"]),
            _ret("result"))
        vm = VMCore()
        mod = _mod(double, main)
        assert vm.execute(mod, fn="main") == 10

    def test_call_updates_fn_call_counts(self):
        add = _fn("add", [("a", "u8"), ("b", "u8")],
            _i("add", "r", ["a", "b"]),
            _ret("r"))
        main = _simple_fn("main",
            _const("a", 1), _const("b", 2),
            _i("call", "r", ["add", "a", "b"]),
            _ret("r"))
        vm = VMCore()
        mod = _mod(add, main)
        vm.execute(mod, fn="main")
        m = vm.metrics()
        assert m.function_call_counts["add"] == 1

    def test_call_builtin(self):
        fn = _simple_fn("main",
            _const("a", 10),
            _i("call_builtin", None, ["noop", "a"]),
            _const("r", 1), _ret("r"))
        vm = VMCore()
        mod = _mod(fn)
        assert vm.execute(mod, fn="main") == 1

    def test_call_builtin_with_result(self):
        fn = _simple_fn("main",
            _const("a", 5), _const("b", 5),
            _i("call_builtin", None, ["assert_eq", "a", "b"]),
            _const("r", 0), _ret("r"))
        vm = VMCore()
        mod = _mod(fn)
        assert vm.execute(mod, fn="main") == 0

    def test_call_unknown_function_raises(self):
        fn = _simple_fn("main",
            _i("call", "r", ["missing"]),
            _ret("r"))
        vm = VMCore()
        mod = _mod(fn)
        with pytest.raises(UnknownOpcodeError, match="missing"):
            vm.execute(mod, fn="main")

    def test_call_with_args_passed_from_execute(self):
        fn = _fn("main", [("x", "u8")], _ret("x"))
        vm = VMCore()
        mod = _mod(fn)
        assert vm.execute(mod, fn="main", args=[99]) == 99


# ---------------------------------------------------------------------------
# TestJITHandler
# ---------------------------------------------------------------------------

class TestJITHandler:
    def test_jit_handler_called(self):
        called_with = []
        def jit_handler(args):
            called_with.append(args)
            return args[0] * 2

        vm = VMCore()
        vm.register_jit_handler("double", jit_handler)

        double = _fn("double", [("x", "u8")],
            _const("two", 2), _i("mul", "r", ["x", "two"]), _ret("r"))
        main = _simple_fn("main",
            _const("a", 7),
            _i("call", "r", ["double", "a"]),
            _ret("r"))

        mod = _mod(double, main)
        result = vm.execute(mod, fn="main")
        assert result == 14
        assert called_with == [[7]]

    def test_jit_handler_bypasses_interpreter(self):
        # If JIT fires, the interpreter instructions in "add" never execute.
        def jit_add(args):
            return args[0] + args[1]

        vm = VMCore()
        vm.register_jit_handler("add", jit_add)

        add_fn = _fn("add", [("a", "u8"), ("b", "u8")],
            _const("bogus", 999), _ret("bogus"))  # would return 999 if interpreted
        main = _simple_fn("main",
            _const("a", 3), _const("b", 4),
            _i("call", "r", ["add", "a", "b"]),
            _ret("r"))

        mod = _mod(add_fn, main)
        assert vm.execute(mod, fn="main") == 7  # JIT path

    def test_jit_hit_metric_increments(self):
        vm = VMCore()
        vm.register_jit_handler("noop_fn", lambda _: None)

        noop_fn = _fn("noop_fn", [], _ret_void())
        main = _simple_fn("main",
            _i("call", None, ["noop_fn"]),
            _i("call", None, ["noop_fn"]),
            _ret_void())

        mod = _mod(noop_fn, main)
        vm.execute(mod, fn="main")
        assert vm.metrics().total_jit_hits == 2

    def test_unregister_jit_handler(self):
        vm = VMCore()
        vm.register_jit_handler("fn", lambda _: 99)
        vm.unregister_jit_handler("fn")

        fn = _fn("fn", [], _const("r", 1), _ret("r"))
        main = _simple_fn("main",
            _i("call", "r", ["fn"]),
            _ret("r"))
        mod = _mod(fn, main)
        # After unregister, interpreter path is used (returns 1, not 99).
        assert vm.execute(mod, fn="main") == 1


# ---------------------------------------------------------------------------
# TestMetricsAccumulation
# ---------------------------------------------------------------------------

class TestMetricsAccumulation:
    def test_instruction_count_accumulates(self):
        vm = VMCore()
        fn = _simple_fn("main", _const("r", 1), _ret("r"))
        mod = _mod(fn)
        vm.execute(mod)
        first = vm.metrics().total_instructions_executed
        vm.execute(mod)
        second = vm.metrics().total_instructions_executed
        assert second == first * 2

    def test_frame_count_includes_root(self):
        vm = VMCore()
        fn = _simple_fn("main", _ret_void())
        vm.execute(_mod(fn))
        assert vm.metrics().total_frames_pushed >= 1

    def test_fn_call_counts_accumulate(self):
        vm = VMCore()
        fn = _simple_fn("main", _ret_void())
        mod = _mod(fn)
        vm.execute(mod)
        vm.execute(mod)
        assert vm.metrics().function_call_counts["main"] == 2


# ---------------------------------------------------------------------------
# TestInterrupt
# ---------------------------------------------------------------------------

class TestInterrupt:
    def test_interrupt_raises_vm_interrupt(self):
        # Build an infinite loop; interrupt it externally via a builtin side-effect.
        vm = VMCore()
        vm.register_builtin("do_interrupt", lambda args: vm.interrupt())

        fn = _simple_fn("main",
            _i("label", None, ["loop"]),
            _i("call_builtin", None, ["do_interrupt"]),
            _i("jmp", None, ["loop"]))

        mod = _mod(fn)
        with pytest.raises(VMInterrupt):
            vm.execute(mod, fn="main")

    def test_interrupt_flag_cleared_after_raise(self):
        vm = VMCore()
        vm.register_builtin("stop", lambda args: vm.interrupt())
        fn = _simple_fn("main", _i("call_builtin", None, ["stop"]), _ret_void())
        mod = _mod(fn)
        with pytest.raises(VMInterrupt):
            vm.execute(mod, fn="main")
        # After exception, internal flag should be False.
        assert vm._interrupted is False


# ---------------------------------------------------------------------------
# TestReset
# ---------------------------------------------------------------------------

class TestReset:
    def test_reset_clears_memory(self):
        vm = VMCore()
        fn = _simple_fn("main",
            _const("addr", 0), _const("val", 42),
            _i("store_mem", None, ["addr", "val"]),
            _ret_void())
        vm.execute(_mod(fn))
        assert vm.memory[0] == 42
        vm.reset()
        assert vm.memory == {}

    def test_reset_clears_io_ports(self):
        vm = VMCore()
        fn = _simple_fn("main",
            _const("p", 1), _const("v", 7),
            _i("io_out", None, ["p", "v"]),
            _ret_void())
        vm.execute(_mod(fn))
        assert vm.io_ports[1] == 7
        vm.reset()
        assert vm.io_ports == {}

    def test_metrics_survive_reset(self):
        vm = VMCore()
        fn = _simple_fn("main", _const("r", 1), _ret("r"))
        vm.execute(_mod(fn))
        before = vm.metrics().total_instructions_executed
        vm.reset()
        assert vm.metrics().total_instructions_executed == before


# ---------------------------------------------------------------------------
# TestFrameOverflow
# ---------------------------------------------------------------------------

class TestFrameOverflow:
    def test_deep_recursion_raises(self):
        recurse = _fn("recurse", [],
            _i("call", None, ["recurse"]),
            _ret_void())
        mod = _mod(recurse)
        vm = VMCore(max_frames=10)
        with pytest.raises(FrameOverflowError, match="10"):
            vm.execute(mod, fn="recurse")


# ---------------------------------------------------------------------------
# TestErrors
# ---------------------------------------------------------------------------

class TestErrors:
    def test_unknown_opcode_raises(self):
        fn = _simple_fn("main", _i("bogus_op"))
        vm = VMCore()
        with pytest.raises(UnknownOpcodeError, match="bogus_op"):
            vm.execute(_mod(fn), fn="main")

    def test_undefined_variable_raises(self):
        fn = _simple_fn("main", _ret("undefined_var"))
        vm = VMCore()
        with pytest.raises(UndefinedVariableError, match="undefined_var"):
            vm.execute(_mod(fn), fn="main")

    def test_vm_error_is_base(self):
        assert issubclass(UnknownOpcodeError, VMError)
        assert issubclass(FrameOverflowError, VMError)
        assert issubclass(UndefinedVariableError, VMError)
        assert issubclass(VMInterrupt, VMError)


# ---------------------------------------------------------------------------
# TestExecuteCoercions
# ---------------------------------------------------------------------------

class TestExecuteCoercions:
    def test_cast_to_u8(self):
        fn = _simple_fn("main",
            _const("v", 300),
            _i("cast", "r", ["v", "u8"]),
            _ret("r"))
        assert run(fn) == 44  # 300 & 0xFF

    def test_cast_to_u16(self):
        fn = _simple_fn("main",
            _const("v", 70000),
            _i("cast", "r", ["v", "u16"]),
            _ret("r"))
        assert run(fn) == 70000 & 0xFFFF

    def test_cast_to_u32(self):
        fn = _simple_fn("main",
            _const("v", 2**33),
            _i("cast", "r", ["v", "u32"]),
            _ret("r"))
        assert run(fn) == (2**33) & 0xFFFFFFFF

    def test_cast_to_bool(self):
        fn = _simple_fn("main",
            _const("v", 0),
            _i("cast", "r", ["v", "bool"]),
            _ret("r"))
        assert run(fn) is False

    def test_cast_to_str(self):
        fn = _simple_fn("main",
            _const("v", 42),
            _i("cast", "r", ["v", "str"]),
            _ret("r"))
        assert run(fn) == "42"

    def test_type_assert_passes(self):
        fn = _simple_fn("main",
            _const("v", 100),
            _i("type_assert", None, ["v", "u8"]),
            _ret("v"))
        assert run(fn) == 100

    def test_type_assert_fails(self):
        fn = _simple_fn("main",
            _const("v", 3.14),
            _i("type_assert", None, ["v", "u8"]),
            _ret("v"))
        with pytest.raises(VMError, match="type_assert"):
            run(fn)

    def test_cast_uses_type_hint_when_no_second_src(self):
        fn = _simple_fn("main",
            _const("v", 300),
            IIRInstr(op="cast", dest="r", srcs=["v"], type_hint="u8"),
            _ret("r"))
        assert run(fn) == 44


# ---------------------------------------------------------------------------
# TestRepl (incremental module pattern)
# ---------------------------------------------------------------------------

class TestRepl:
    def test_execute_different_functions_sequentially(self):
        """Simulate REPL: define two functions, call them in sequence."""
        add = _fn("add", [("a", "u8"), ("b", "u8")],
            _i("add", "r", ["a", "b"]), _ret("r"))
        sub = _fn("sub", [("a", "u8"), ("b", "u8")],
            _i("sub", "r", ["a", "b"]), _ret("r"))

        vm = VMCore()
        mod = _mod(add, sub)

        assert vm.execute(mod, fn="add", args=[10, 3]) == 13
        assert vm.execute(mod, fn="sub", args=[10, 3]) == 7

    def test_add_or_replace_function(self):
        """Simulate re-defining a function in the REPL."""
        fn_v1 = _fn("compute", [], _const("r", 1), _ret("r"))
        fn_v2 = _fn("compute", [], _const("r", 2), _ret("r"))

        mod = _mod(fn_v1)
        vm = VMCore()
        assert vm.execute(mod, fn="compute") == 1

        mod.add_or_replace(fn_v2)
        assert vm.execute(mod, fn="compute") == 2


# ---------------------------------------------------------------------------
# TestMultiFunction
# ---------------------------------------------------------------------------

class TestMultiFunction:
    def test_nested_calls(self):
        """a(b(x)) — two levels of call."""
        inner = _fn("inner", [("x", "u8")],
            _const("two", 2),
            _i("mul", "r", ["x", "two"]),
            _ret("r"))
        outer = _fn("outer", [("x", "u8")],
            _i("call", "r", ["inner", "x"]),
            _const("three", 3),
            _i("add", "r2", ["r", "three"]),
            _ret("r2"))
        main = _simple_fn("main",
            _const("a", 4),
            _i("call", "result", ["outer", "a"]),
            _ret("result"))

        vm = VMCore()
        mod = _mod(inner, outer, main)
        assert vm.execute(mod, fn="main") == 11  # (4*2)+3

    def test_multiple_callee_args(self):
        add3 = _fn("add3", [("a", "u8"), ("b", "u8"), ("c", "u8")],
            _i("add", "ab", ["a", "b"]),
            _i("add", "r", ["ab", "c"]),
            _ret("r"))
        main = _simple_fn("main",
            _const("x", 1), _const("y", 2), _const("z", 3),
            _i("call", "r", ["add3", "x", "y", "z"]),
            _ret("r"))

        vm = VMCore()
        mod = _mod(add3, main)
        assert vm.execute(mod, fn="main") == 6

    def test_profiler_observes_results(self):
        fn = _simple_fn("main",
            _const("a", 5), _const("b", 3),
            _i("add", "r", ["a", "b"]),
            _ret("r"))
        vm = VMCore(profiler_enabled=True)
        vm.execute(_mod(fn))
        assert vm.profiler.total_observations > 0

    def test_profiler_disabled_makes_no_observations(self):
        fn = _simple_fn("main",
            _const("a", 5), _const("b", 3),
            _i("add", "r", ["a", "b"]),
            _ret("r"))
        vm = VMCore(profiler_enabled=False)
        vm.execute(_mod(fn))
        assert vm.profiler.total_observations == 0

    def test_custom_opcode_overrides_standard(self):
        """Language-specific handler takes priority over standard table."""
        calls = []
        def custom_add(vm, frame, instr):
            calls.append("custom_add")
            a = frame.resolve(instr.srcs[0])
            b = frame.resolve(instr.srcs[1])
            result = a + b + 1000  # adds 1000 to distinguish
            if instr.dest:
                frame.assign(instr.dest, result)
            return result

        fn = _simple_fn("main",
            _const("a", 1), _const("b", 2),
            _i("add", "r", ["a", "b"]),
            _ret("r"))
        vm = VMCore(opcodes={"add": custom_add})
        assert vm.execute(_mod(fn), fn="main") == 1003
        assert calls == ["custom_add"]

    def test_is_executing_false_after_done(self):
        vm = VMCore()
        fn = _simple_fn("main", _ret_void())
        vm.execute(_mod(fn))
        assert not vm.is_executing
