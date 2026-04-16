"""CLR runtime orchestration built from reusable decoder and simulator pieces."""

from __future__ import annotations

from dataclasses import dataclass

from clr_bytecode_disassembler import CLRMethodBody, disassemble_clr_method
from clr_pe_file import CLRMemberReference, CLRPEFile, decode_clr_pe_file
from clr_simulator import CLRSimulator, CLRTrace


@dataclass(frozen=True)
class CLRRuntimeResult:
    """Result of running a CLR entry point."""

    assembly: CLRPEFile
    method_body: CLRMethodBody
    output: str
    traces: tuple[CLRTrace, ...]


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


class CLRRuntime:
    """Compose CLR assembly decoding, disassembly, and simulation."""

    def __init__(self, host: CLRStdlibHost | None = None) -> None:
        self.host = host or CLRStdlibHost()

    def decode(self, assembly_bytes: bytes) -> CLRPEFile:
        return decode_clr_pe_file(assembly_bytes)

    def disassemble_entry_point(self, assembly: CLRPEFile) -> CLRMethodBody:
        return disassemble_clr_method(assembly, assembly.get_entry_point_method())

    def run_entry_point(self, assembly_bytes: bytes) -> CLRRuntimeResult:
        self.host.output.clear()
        assembly = self.decode(assembly_bytes)
        method_body = self.disassemble_entry_point(assembly)
        simulator = CLRSimulator(host=self.host)
        simulator.load_method_body(method_body)
        traces = tuple(simulator.run())
        return CLRRuntimeResult(
            assembly=assembly,
            method_body=method_body,
            output="".join(self.host.output),
            traces=traces,
        )
