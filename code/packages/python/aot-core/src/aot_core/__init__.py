"""aot-core: ahead-of-time compilation path for the LANG pipeline (LANG04).

Public surface
--------------
``AOTCore``        — the top-level AOT compilation controller.
``AOTStats``       — compilation statistics snapshot.
``AOTSnapshot``    — parsed ``.aot`` binary contents.
``VmRuntime``      — pre-compiled vm-runtime library wrapper.
``infer_types``    — static type inference pass (IIRFunction → dict[str, str]).
``aot_specialise`` — AOT specialization pass (IIRFunction → list[CIRInstr]).
``link``           — linker module (concatenate per-function binaries).
``snapshot``       — ``.aot`` binary writer/reader module.
"""

from aot_core.core import AOTCore
from aot_core.infer import infer_types
from aot_core.snapshot import AOTSnapshot
from aot_core.specialise import aot_specialise
from aot_core.stats import AOTStats
from aot_core.vm_runtime import VmRuntime

__all__ = [
    "AOTCore",
    "AOTStats",
    "AOTSnapshot",
    "VmRuntime",
    "infer_types",
    "aot_specialise",
]
