"""Top-level orchestration for the modular JVM prototype."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from jvm_bytecode_disassembler import JVMMethodBody, JVMVersion, disassemble_method_body
from jvm_class_file import (
    JVMClassFile,
    JVMFieldReference,
    JVMMethodReference,
    parse_class_file,
)
from jvm_simulator import JVMSimulator, JVMTrace


@dataclass(frozen=True)
class JVMPrintStream:
    class_name: str = "java/io/PrintStream"


class JVMStdlibHost:
    """Tiny host bridge for the first real Hello World execution path."""

    def __init__(self, stdout: Callable[[str], None] | None = None) -> None:
        self._stdout = stdout
        self.output: list[str] = []

    def get_static(self, reference: object) -> object:
        if (
            isinstance(reference, JVMFieldReference)
            and reference.class_name == "java/lang/System"
            and reference.name == "out"
            and reference.descriptor == "Ljava/io/PrintStream;"
        ):
            return JVMPrintStream()
        msg = f"Unsupported static field reference: {reference!r}"
        raise RuntimeError(msg)

    def invoke_virtual(
        self,
        reference: object,
        receiver: object,
        args: list[object],
    ) -> object | None:
        if (
            isinstance(reference, JVMMethodReference)
            and isinstance(receiver, JVMPrintStream)
            and reference.class_name == "java/io/PrintStream"
            and reference.name == "println"
            and reference.descriptor == "(Ljava/lang/String;)V"
            and len(args) == 1
            and isinstance(args[0], str)
        ):
            message = f"{args[0]}\n"
            self.output.append(message)
            if self._stdout is not None:
                self._stdout(message)
            return None
        msg = f"Unsupported virtual method reference: {reference!r}"
        raise RuntimeError(msg)


@dataclass(frozen=True)
class JVMRunResult:
    class_file: JVMClassFile
    method: JVMMethodBody
    traces: tuple[JVMTrace, ...]
    output: str
    return_value: object | None


class JVMRuntime:
    """Compose class-file decode, disassembly, and simulator execution."""

    def __init__(self, *, stdout: Callable[[str], None] | None = None) -> None:
        self.host = JVMStdlibHost(stdout=stdout)
        self.simulator = JVMSimulator(host=self.host)

    def load_class(self, class_bytes: bytes) -> JVMClassFile:
        return parse_class_file(class_bytes)

    def disassemble_method(
        self,
        class_file: JVMClassFile,
        *,
        method_name: str,
        descriptor: str,
    ) -> JVMMethodBody:
        method = class_file.find_method(method_name, descriptor)
        if method is None or method.code_attribute is None:
            msg = (
                f"Method {method_name}{descriptor} was not found "
                "or has no Code attribute"
            )
            raise RuntimeError(msg)

        constant_pool_lookup: dict[int, object] = {}
        for index in range(1, len(class_file.constant_pool)):
            entry = class_file.constant_pool[index]
            if entry is None:
                continue
            try:
                constant_pool_lookup[index] = class_file.resolve_constant(index)
                continue
            except ValueError:
                pass
            if type(entry).__name__ == "JVMFieldrefInfo":
                constant_pool_lookup[index] = class_file.resolve_fieldref(index)
            elif type(entry).__name__ == "JVMMethodrefInfo":
                constant_pool_lookup[index] = class_file.resolve_methodref(index)

        return disassemble_method_body(
            method.code_attribute.code,
            version=JVMVersion(class_file.version.major, class_file.version.minor),
            max_stack=method.code_attribute.max_stack,
            max_locals=method.code_attribute.max_locals,
            constant_pool=constant_pool_lookup,
        )

    def run_method(
        self,
        class_file_or_bytes: JVMClassFile | bytes,
        *,
        method_name: str,
        descriptor: str,
    ) -> JVMRunResult:
        class_file = (
            class_file_or_bytes
            if isinstance(class_file_or_bytes, JVMClassFile)
            else self.load_class(class_file_or_bytes)
        )
        method = self.disassemble_method(
            class_file,
            method_name=method_name,
            descriptor=descriptor,
        )
        self.host.output.clear()
        self.simulator.load_method(method)
        traces = tuple(self.simulator.run())
        return JVMRunResult(
            class_file=class_file,
            method=method,
            traces=traces,
            output="".join(self.host.output),
            return_value=self.simulator.return_value,
        )

    def run_main(
        self,
        class_file_or_bytes: JVMClassFile | bytes,
        args: list[str] | None = None,
    ) -> JVMRunResult:
        _ = args
        return self.run_method(
            class_file_or_bytes,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )
