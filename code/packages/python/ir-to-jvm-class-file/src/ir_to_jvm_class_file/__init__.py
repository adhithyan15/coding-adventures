"""Prototype backend lowering compiler-ir programs to JVM class files.

LANG20: ``JVMCodeGenerator`` implements ``CodeGenerator[IrProgram, JVMClassArtifact]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.
"""

from ir_to_jvm_class_file.backend import (
    JvmBackendConfig,
    JvmBackendError,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
    validate_for_jvm,
    write_class_file,
)
from ir_to_jvm_class_file.generator import JVMCodeGenerator

__all__ = [
    "JVMClassArtifact",
    "JVMCodeGenerator",
    "JvmBackendConfig",
    "JvmBackendError",
    "lower_ir_to_jvm_class_file",
    "validate_for_jvm",
    "write_class_file",
]
