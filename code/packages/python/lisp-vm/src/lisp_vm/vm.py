"""Lisp VM Factory — Creates a GenericVM configured for McCarthy's Lisp.

==========================================================================
Chapter 1: The Factory Pattern
==========================================================================

The ``create_lisp_vm()`` function creates a GenericVM with all Lisp opcodes
registered. This follows the same pattern as Starlark and Brainfuck:

1. Create a blank GenericVM
2. Register all opcode handlers
3. Return the configured VM

The factory also creates the GC and symbol table, passing them to handlers
via closures. This means each Lisp VM instance has its own heap and symbol
table — no shared mutable state between VMs.

==========================================================================
Chapter 2: Pluggable GC
==========================================================================

The factory accepts an optional ``gc`` parameter. If not provided, it
creates a default ``MarkAndSweepGC``. You can pass any GC implementation::

    vm = create_lisp_vm()                          # default mark-and-sweep
    vm = create_lisp_vm(gc=MarkAndSweepGC())       # explicit mark-and-sweep
    vm = create_lisp_vm(gc=RefCountGC())           # future: ref counting
"""

from __future__ import annotations

from garbage_collector import GarbageCollector, MarkAndSweepGC, SymbolTable
from virtual_machine import GenericVM

from lisp_vm.handlers import (
    call_function_handler,
    car_handler,
    cdr_handler,
    cons_handler,
    handle_add,
    handle_cmp_eq,
    handle_cmp_gt,
    handle_cmp_lt,
    handle_div,
    handle_halt,
    handle_is_nil,
    handle_jump,
    handle_jump_if_false,
    handle_jump_if_true,
    handle_load_const,
    handle_load_local,
    handle_load_name,
    handle_load_nil,
    handle_load_true,
    handle_mul,
    handle_pop,
    handle_return,
    handle_store_local,
    handle_store_name,
    handle_sub,
    is_atom_handler,
    make_closure_handler,
    make_symbol_handler,
    print_handler,
    tail_call_handler,
)
from lisp_vm.opcodes import LispOp


def create_lisp_vm(
    gc: GarbageCollector | None = None,
) -> GenericVM:
    """Create a GenericVM configured for McCarthy's 1960 Lisp.

    This registers all Lisp opcodes with the VM and sets up the GC
    and symbol table for heap management.

    Args:
        gc: An optional garbage collector instance. If not provided,
            a new ``MarkAndSweepGC`` is created.

    Returns:
        A ``GenericVM`` with all Lisp opcodes registered, ready to
        execute Lisp bytecode.

    Example::

        vm = create_lisp_vm()
        code = CodeObject(
            instructions=[
                Instruction(opcode=LispOp.LOAD_CONST, operand=0),
                Instruction(opcode=LispOp.HALT),
            ],
            constants=[42],
            names=[],
        )
        output = vm.execute(code)
    """
    vm = GenericVM()
    actual_gc = gc or MarkAndSweepGC()
    symbol_table = SymbolTable(actual_gc)

    # Store GC and symbol table on the VM for external access (e.g., tests)
    vm.gc = actual_gc  # type: ignore[attr-defined]
    vm.symbol_table = symbol_table  # type: ignore[attr-defined]

    # -----------------------------------------------------------------
    # Register all opcode handlers
    # -----------------------------------------------------------------

    # Stack operations
    vm.register_opcode(LispOp.LOAD_CONST, handle_load_const)
    vm.register_opcode(LispOp.POP, handle_pop)
    vm.register_opcode(LispOp.LOAD_NIL, handle_load_nil)
    vm.register_opcode(LispOp.LOAD_TRUE, handle_load_true)

    # Variable operations
    vm.register_opcode(LispOp.STORE_NAME, handle_store_name)
    vm.register_opcode(LispOp.LOAD_NAME, handle_load_name)
    vm.register_opcode(LispOp.STORE_LOCAL, handle_store_local)
    vm.register_opcode(LispOp.LOAD_LOCAL, handle_load_local)

    # Arithmetic
    vm.register_opcode(LispOp.ADD, handle_add)
    vm.register_opcode(LispOp.SUB, handle_sub)
    vm.register_opcode(LispOp.MUL, handle_mul)
    vm.register_opcode(LispOp.DIV, handle_div)

    # Comparison
    vm.register_opcode(LispOp.CMP_EQ, handle_cmp_eq)
    vm.register_opcode(LispOp.CMP_LT, handle_cmp_lt)
    vm.register_opcode(LispOp.CMP_GT, handle_cmp_gt)

    # Control flow
    vm.register_opcode(LispOp.JUMP, handle_jump)
    vm.register_opcode(LispOp.JUMP_IF_FALSE, handle_jump_if_false)
    vm.register_opcode(LispOp.JUMP_IF_TRUE, handle_jump_if_true)

    # Functions — these need GC access via closures
    vm.register_opcode(LispOp.MAKE_CLOSURE, make_closure_handler(actual_gc))
    vm.register_opcode(
        LispOp.CALL_FUNCTION, call_function_handler(actual_gc),
    )
    vm.register_opcode(LispOp.TAIL_CALL, tail_call_handler(actual_gc))
    vm.register_opcode(LispOp.RETURN, handle_return)

    # Lisp-specific — these need GC and/or symbol table
    vm.register_opcode(LispOp.CONS, cons_handler(actual_gc))
    vm.register_opcode(LispOp.CAR, car_handler(actual_gc))
    vm.register_opcode(LispOp.CDR, cdr_handler(actual_gc))
    vm.register_opcode(
        LispOp.MAKE_SYMBOL, make_symbol_handler(actual_gc, symbol_table),
    )
    vm.register_opcode(LispOp.IS_ATOM, is_atom_handler(actual_gc))
    vm.register_opcode(LispOp.IS_NIL, handle_is_nil)

    # I/O
    vm.register_opcode(LispOp.PRINT, print_handler(actual_gc))

    # VM control
    vm.register_opcode(LispOp.HALT, handle_halt)

    return vm
