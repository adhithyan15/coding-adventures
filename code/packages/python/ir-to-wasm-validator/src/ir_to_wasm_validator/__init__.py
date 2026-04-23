"""Validate whether IR is translatable into the current WASM backend."""

from dataclasses import dataclass

from compiler_ir import IrProgram
from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler, WasmLoweringError


@dataclass(frozen=True)
class ValidationError:
    rule: str
    message: str


class WasmIrValidator:
    """Validate the generic IR against the current WASM lowering rules."""

    def validate(
        self,
        program: IrProgram,
        function_signatures: list[FunctionSignature] | None = None,
        *,
        strategy: str = "structured",
    ) -> list[ValidationError]:
        try:
            IrToWasmCompiler().compile(
                program,
                function_signatures,
                strategy=strategy,
            )
        except WasmLoweringError as exc:
            return [ValidationError(rule="lowering", message=str(exc))]
        return []


def validate(
    program: IrProgram,
    function_signatures: list[FunctionSignature] | None = None,
    *,
    strategy: str = "structured",
) -> list[ValidationError]:
    return WasmIrValidator().validate(
        program,
        function_signatures,
        strategy=strategy,
    )


__all__ = [
    "ValidationError",
    "WasmIrValidator",
    "validate",
]
