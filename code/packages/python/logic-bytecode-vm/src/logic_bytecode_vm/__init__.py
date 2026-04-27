"""logic-bytecode-vm — a direct runtime for LP09 loader bytecode.

`logic-vm` proved that the logic stack could execute a structured instruction
stream through a VM. `logic-bytecode-vm` takes the next step: it consumes the
compact LP09 bytecode format directly, resolves pool operands at runtime, and
still exposes a friendly, traceable interface for loading and query execution.
"""

from logic_bytecode_vm.vm import (
    LogicBytecodeVM,
    LogicBytecodeVMError,
    LogicBytecodeVMState,
    LogicBytecodeVMTraceEntry,
    LogicBytecodeVMValidationError,
    UnknownLogicBytecodeOpcodeError,
    compile_and_execute,
    compile_and_execute_all,
    create_logic_bytecode_vm,
    execute,
    execute_all,
)

__all__ = [
    "__version__",
    "LogicBytecodeVM",
    "LogicBytecodeVMError",
    "LogicBytecodeVMState",
    "LogicBytecodeVMTraceEntry",
    "LogicBytecodeVMValidationError",
    "UnknownLogicBytecodeOpcodeError",
    "create_logic_bytecode_vm",
    "execute",
    "execute_all",
    "compile_and_execute",
    "compile_and_execute_all",
]

__version__ = "0.1.0"
