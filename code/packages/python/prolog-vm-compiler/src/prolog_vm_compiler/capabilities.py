"""Capability manifest for the Prolog-on-Logic-VM implementation track."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Literal

type CapabilityStatus = Literal["complete", "deferred"]


@dataclass(frozen=True, slots=True)
class PrologVMCapability:
    """One reviewable Prolog VM capability batch and its implementation status."""

    id: str
    title: str
    status: CapabilityStatus
    specs: tuple[str, ...]
    packages: tuple[str, ...]
    features: tuple[str, ...]

    def as_dict(self) -> dict[str, object]:
        """Return a JSON-serializable representation of this capability."""

        return asdict(self)


@dataclass(frozen=True, slots=True)
class PrologVMCapabilityManifest:
    """Machine-readable summary of the finished core Prolog VM surface."""

    track: str
    status: str
    dialects: tuple[str, ...]
    backends: tuple[str, ...]
    capabilities: tuple[PrologVMCapability, ...]
    deferred_capabilities: tuple[PrologVMCapability, ...]

    def as_dict(self) -> dict[str, object]:
        """Return a JSON-serializable representation of the manifest."""

        return {
            "track": self.track,
            "status": self.status,
            "dialects": list(self.dialects),
            "backends": list(self.backends),
            "capabilities": [
                capability.as_dict() for capability in self.capabilities
            ],
            "deferred_capabilities": [
                capability.as_dict() for capability in self.deferred_capabilities
            ],
        }

    @property
    def complete_count(self) -> int:
        """Return the number of completed core capability batches."""

        return sum(
            1 for capability in self.capabilities if capability.status == "complete"
        )

    @property
    def deferred_count(self) -> int:
        """Return the number of explicitly deferred advanced-dialect batches."""

        return len(self.deferred_capabilities)


def prolog_vm_capabilities() -> tuple[PrologVMCapability, ...]:
    """Return the completed Prolog-on-Logic-VM capability batches."""

    return (
        PrologVMCapability(
            id="frontend-loader",
            title="Frontend, directives, modules, files, and expansion",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(0, 17)),
            packages=(
                "prolog-lexer",
                "prolog-parser",
                "iso-prolog-parser",
                "swi-prolog-parser",
                "prolog-operator-parser",
                "prolog-loader",
            ),
            features=(
                "ISO/SWI dialect profiles",
                "operator declarations and operator-aware parsing",
                "predicate registry and directive handling",
                "module metadata, imports, exports, and qualification",
                "file loading, include/consult resolution, and linked projects",
                "DCG and expansion pipeline lowering",
            ),
        ),
        PrologVMCapability(
            id="vm-runtime",
            title="Compiler, VM execution, runtime API, and initialization",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(17, 23)),
            packages=(
                "logic-instructions",
                "logic-vm",
                "logic-bytecode",
                "logic-bytecode-vm",
                "prolog-vm-compiler",
            ),
            features=(
                "structured Logic VM instruction compilation",
                "bytecode VM convergence path",
                "source/file/project runtime creation",
                "module-context top-level queries",
                "initialization query slots and explicit initialization control",
                "named answers and residual constraints",
            ),
        ),
        PrologVMCapability(
            id="stdlib-core",
            title="Core Prolog stdlib and list/arithmetic compatibility",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(23, 30)),
            packages=("logic-builtins", "prolog-loader", "prolog-vm-compiler"),
            features=(
                "list constructors, append/member/select/reverse predicates",
                "length, sort, nth0/nth1, nth-rest, between, succ, and integer",
                "source-level builtin adaptation through the VM path",
            ),
        ),
        PrologVMCapability(
            id="clpfd",
            title="CLP(FD) modeling surface",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(30, 38)),
            packages=("logic-core", "logic-engine", "logic-builtins"),
            features=(
                "finite-domain constraints and labeling",
                "CLP(FD) infix syntax and nested arithmetic",
                "labeling options",
                "sum and scalar-product globals",
                "modeling helpers, reification, and boolean constraints",
            ),
        ),
        PrologVMCapability(
            id="meta-control",
            title="Meta-call, higher-order predicates, control, and exceptions",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(38, 48)),
            packages=("logic-engine", "logic-builtins", "prolog-loader"),
            features=(
                "call/N and module-aware apply closures",
                "maplist/include/exclude/foldl/apply-family predicates",
                "term equality, dif/2, residual constraints, and aggregation stress",
                "strict arithmetic errors, throw/catch, and cleanup control",
            ),
        ),
        PrologVMCapability(
            id="term-text-reflection",
            title="Term, text, flags, and reflection predicates",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(48, 70)),
            packages=("logic-builtins", "prolog-loader", "prolog-vm-compiler"),
            features=(
                "term variables, generality, hashes, shape, and unifiability",
                "term read/write and write-option support",
                "atom/string/number/char/code conversion and composition",
                "current_op/current_atom/current_functor/current_predicate",
                "branch-local Prolog flags",
                "repeat, control aliases, and atom-number conversion",
            ),
        ),
        PrologVMCapability(
            id="convergence-tooling",
            title="Dialect convergence, bytecode parity, runners, and CLI",
            status="complete",
            specs=tuple(f"PR{index:02d}" for index in range(70, 78)),
            packages=("prolog-vm-compiler",),
            features=(
                "generic dialect-routed compile/run APIs",
                "structured and bytecode backend selector",
                "bytecode stress parity",
                "source/file/project one-shot runner APIs",
                "top-level query APIs",
                "prolog-vm CLI with diagnostics, JSON/JSONL, summaries, and REPL",
            ),
        ),
        PrologVMCapability(
            id="host-file-text-io",
            title="Bounded host file text I/O",
            status="complete",
            specs=("PR78",),
            packages=("logic-builtins", "prolog-loader", "prolog-vm-compiler"),
            features=(
                "exists_file/1 over bound atom/string paths",
                "read_file_to_string/2 for UTF-8 files",
                "read_file_to_codes/2 for UTF-8 code-point lists",
                "structured and bytecode VM stress coverage",
            ),
        ),
        PrologVMCapability(
            id="host-file-stream-io",
            title="Bounded host file stream I/O",
            status="complete",
            specs=("PR79",),
            packages=("logic-builtins", "prolog-loader", "prolog-vm-compiler"),
            features=(
                "open/3 and close/1 for bounded UTF-8 file streams",
                "read_string/3, read_line_to_string/2, get_char/2",
                "at_end_of_stream/1 cursor checks",
                "write/2 and nl/1 for file-backed write and append streams",
                "structured and bytecode VM stress coverage",
            ),
        ),
        PrologVMCapability(
            id="host-stream-metadata",
            title="Bounded host stream options and metadata",
            status="complete",
            specs=("PR80",),
            packages=("logic-builtins", "prolog-loader", "prolog-vm-compiler"),
            features=(
                "open/4 option lists with alias, encoding(utf8), and type(text)",
                "alias-addressable close/read/write/flush stream operations",
                "current_stream/3 enumeration",
                "stream_property/2 metadata for file_name, mode, alias, and position",
                "structured and bytecode VM stress coverage",
            ),
        ),
    )


def deferred_prolog_vm_capabilities() -> tuple[PrologVMCapability, ...]:
    """Return deliberately deferred work beyond the completed core track."""

    return (
        PrologVMCapability(
            id="full-dialect-emulation",
            title="Full external Prolog dialect emulation",
            status="deferred",
            specs=("PR02",),
            packages=("future",),
            features=(
                "complete SWI/SICStus/GNU/XSB/YAP/Ciao compatibility modes",
                "dialect-specific dicts, quasiquotations, packs, and extensions",
            ),
        ),
        PrologVMCapability(
            id="advanced-solver-services",
            title="Advanced solver services",
            status="deferred",
            specs=("PR02",),
            packages=("future",),
            features=(
                "tabling and well-founded negation",
                "attributed variables and generalized coroutining",
                "CHR and non-FD constraint domains",
            ),
        ),
        PrologVMCapability(
            id="host-runtime-services",
            title="Host runtime services",
            status="deferred",
            specs=("PR02",),
            packages=("future",),
            features=(
                "standard streams, binary streams, and repositioning",
                "rich ISO/SWI stream options beyond the bounded UTF-8 subset",
                "foreign predicates and host callbacks",
                "engines, concurrency, and async integration",
            ),
        ),
    )


def prolog_vm_capability_manifest() -> PrologVMCapabilityManifest:
    """Return the canonical capability manifest for this package."""

    return PrologVMCapabilityManifest(
        track="Prolog-on-Logic-VM PR00-PR80",
        status="core-plus-stream-metadata",
        dialects=("iso", "swi"),
        backends=("structured", "bytecode"),
        capabilities=prolog_vm_capabilities(),
        deferred_capabilities=deferred_prolog_vm_capabilities(),
    )
