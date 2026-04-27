"""logic-vm — a dispatch runtime for standardized logic instructions.

`logic-instructions` made logic programs explicit executable data. `logic-vm`
adds the missing runtime shell around that data: a VM with a program counter,
an instruction dispatch table, mutable runtime state, and traceable step-by-
step loading.

This first VM is intentionally high-level. It executes the existing LP07
instructions directly, then delegates actual proof search to `logic-engine`
once the program has been loaded.
"""

from logic_vm.vm import (
    LogicInstructionHandler,
    LogicVM,
    LogicVMError,
    LogicVMState,
    LogicVMTraceEntry,
    LogicVMValidationError,
    UnknownInstructionOpcodeError,
    create_logic_vm,
    execute,
    execute_all,
)

__all__ = [
    "__version__",
    "LogicInstructionHandler",
    "LogicVM",
    "LogicVMError",
    "LogicVMState",
    "LogicVMTraceEntry",
    "LogicVMValidationError",
    "UnknownInstructionOpcodeError",
    "create_logic_vm",
    "execute",
    "execute_all",
]

__version__ = "0.2.0"
