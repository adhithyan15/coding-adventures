"""CLR runtime orchestration built from reusable decoder and simulator pieces."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from clr_bytecode_disassembler import CLRMethodBody, disassemble_clr_method
from clr_pe_file import (
    CLRMemberReference,
    CLRMethodDef,
    CLRPEFile,
    decode_clr_pe_file,
)
from clr_simulator import CLRSimulator, CLRTrace


class CLRHost(Protocol):
    """Host calls exposed to simulated CLR methods."""

    output: list[str]

    def call_method(
        self,
        method: CLRMemberReference,
        args: list[object | None],
    ) -> object | None:
        """Handle an external CLR method call."""


@dataclass(frozen=True)
class CLRDecodeStage:
    """Result of decoding a managed PE/CLI assembly."""

    assembly_bytes: bytes
    assembly: CLRPEFile


@dataclass(frozen=True)
class CLRMethodSelectionStage:
    """Result of selecting the method to execute or inspect."""

    assembly: CLRPEFile
    method: CLRMethodDef


@dataclass(frozen=True)
class CLRDisassemblyStage:
    """Result of disassembling one CLR method body."""

    assembly: CLRPEFile
    method: CLRMethodDef
    method_body: CLRMethodBody


@dataclass(frozen=True)
class CLRExecutionStage:
    """Result of executing a disassembled CLR method body."""

    method_body: CLRMethodBody
    output: str
    traces: tuple[CLRTrace, ...]


@dataclass(frozen=True)
class CLRRuntimeResult:
    """Result of running a CLR entry point."""

    assembly: CLRPEFile
    method_body: CLRMethodBody
    output: str
    traces: tuple[CLRTrace, ...]
    decode_stage: CLRDecodeStage
    method_selection_stage: CLRMethodSelectionStage
    disassembly_stage: CLRDisassemblyStage
    execution_stage: CLRExecutionStage


class CLRStdlibHost:
    """Tiny host bridge for the CLR hello-world vertical slice."""

    def __init__(self) -> None:
        self.output: list[str] = []

    def call_method(
        self,
        method: CLRMemberReference,
        args: list[object | None],
    ) -> None:
        if (
            method.declaring_type == "System.Console"
            and method.name == "WriteLine"
            and method.signature.parameter_types == ("string",)
        ):
            self.output.append(f"{args[0]}\n")
            return None
        msg = (
            "Unsupported CLR host call "
            f"{method.declaring_type}.{method.name}"
            f"{method.signature.parameter_types}"
        )
        raise RuntimeError(msg)


type DecodeAssemblyStage = Callable[[bytes], CLRPEFile]
type SelectMethodStage = Callable[[CLRPEFile], CLRMethodDef]
type DisassembleMethodStage = Callable[[CLRPEFile, CLRMethodDef], CLRMethodBody]
type ExecuteMethodStage = Callable[
    [CLRMethodBody, CLRHost, int],
    tuple[CLRTrace, ...],
]


class CLRRuntimePipeline:
    """Composable CLR assembly pipeline.

    The default pipeline is:

    assembly bytes -> PE/CLI decoder -> method selector -> CIL disassembler
    -> CLR simulator.

    Each stage can be replaced independently so compiler backends can validate
    generated assemblies, tooling can stop after disassembly, and tests can
    inject exact boundaries without reimplementing the whole runtime.
    """

    def __init__(
        self,
        host: CLRHost | None = None,
        *,
        decode_stage: DecodeAssemblyStage | None = None,
        select_method_stage: SelectMethodStage | None = None,
        disassemble_stage: DisassembleMethodStage | None = None,
        execute_stage: ExecuteMethodStage | None = None,
    ) -> None:
        self.host = host or CLRStdlibHost()
        self._decode_stage = decode_stage or decode_clr_pe_file
        self._select_method_stage = (
            select_method_stage or self._default_select_entry_point
        )
        self._disassemble_stage = disassemble_stage or disassemble_clr_method
        self._execute_stage = execute_stage or self._default_execute_method

    def decode(self, assembly_bytes: bytes) -> CLRPEFile:
        """Decode managed PE/CLI assembly bytes."""
        return self._decode_stage(assembly_bytes)

    def select_entry_point(self, assembly: CLRPEFile) -> CLRMethodDef:
        """Select the assembly entry point method."""
        return self._select_method_stage(assembly)

    def disassemble_entry_point(self, assembly: CLRPEFile) -> CLRMethodBody:
        """Disassemble the selected entry point method."""
        method = self.select_entry_point(assembly)
        return self.disassemble_method(assembly, method)

    def disassemble_method(
        self,
        assembly: CLRPEFile,
        method: CLRMethodDef,
    ) -> CLRMethodBody:
        """Disassemble a chosen CLR method."""
        return self._disassemble_stage(assembly, method)

    def execute_method_body(
        self,
        method_body: CLRMethodBody,
        *,
        max_steps: int = 10000,
    ) -> CLRExecutionStage:
        """Execute a disassembled method body through the configured stage."""
        traces = self._execute_stage(method_body, self.host, max_steps)
        return CLRExecutionStage(
            method_body=method_body,
            output="".join(self.host.output),
            traces=traces,
        )

    def run_entry_point(
        self,
        assembly_bytes: bytes,
        *,
        max_steps: int = 10000,
    ) -> CLRRuntimeResult:
        """Run an assembly entry point while preserving every pipeline boundary."""
        self.host.output.clear()

        assembly = self.decode(assembly_bytes)
        decode_stage = CLRDecodeStage(
            assembly_bytes=assembly_bytes,
            assembly=assembly,
        )

        method = self.select_entry_point(assembly)
        method_selection_stage = CLRMethodSelectionStage(
            assembly=assembly,
            method=method,
        )

        method_body = self.disassemble_method(assembly, method)
        disassembly_stage = CLRDisassemblyStage(
            assembly=assembly,
            method=method,
            method_body=method_body,
        )

        execution_stage = self.execute_method_body(method_body, max_steps=max_steps)
        return CLRRuntimeResult(
            assembly=assembly,
            method_body=method_body,
            output=execution_stage.output,
            traces=execution_stage.traces,
            decode_stage=decode_stage,
            method_selection_stage=method_selection_stage,
            disassembly_stage=disassembly_stage,
            execution_stage=execution_stage,
        )

    def decode_assembly(self, assembly_bytes: bytes) -> CLRDecodeStage:
        """Run only the decode stage."""
        assembly = self.decode(assembly_bytes)
        return CLRDecodeStage(assembly_bytes=assembly_bytes, assembly=assembly)

    def select_method(
        self,
        decode_stage: CLRDecodeStage,
    ) -> CLRMethodSelectionStage:
        """Run only the method-selection stage."""
        method = self.select_entry_point(decode_stage.assembly)
        return CLRMethodSelectionStage(
            assembly=decode_stage.assembly,
            method=method,
        )

    def disassemble_selected_method(
        self,
        method_selection_stage: CLRMethodSelectionStage,
    ) -> CLRDisassemblyStage:
        """Run only the disassembly stage for a selected method."""
        method_body = self.disassemble_method(
            method_selection_stage.assembly,
            method_selection_stage.method,
        )
        return CLRDisassemblyStage(
            assembly=method_selection_stage.assembly,
            method=method_selection_stage.method,
            method_body=method_body,
        )

    def execute_disassembled_method(
        self,
        disassembly_stage: CLRDisassemblyStage,
        *,
        max_steps: int = 10000,
    ) -> CLRExecutionStage:
        """Run only the execution stage for a disassembled method."""
        self.host.output.clear()
        return self.execute_method_body(
            disassembly_stage.method_body,
            max_steps=max_steps,
        )

    @staticmethod
    def _default_select_entry_point(assembly: CLRPEFile) -> CLRMethodDef:
        return assembly.get_entry_point_method()

    @staticmethod
    def _default_execute_method(
        method_body: CLRMethodBody,
        host: CLRHost,
        max_steps: int,
    ) -> tuple[CLRTrace, ...]:
        simulator = CLRSimulator(host=host)
        simulator.load_method_body(method_body)
        return tuple(simulator.run(max_steps=max_steps))


class CLRRuntime(CLRRuntimePipeline):
    """Backward-compatible name for the default composable CLR pipeline."""
