"""Prototype backend lowering compiler-ir programs to JVM class files."""

from ir_to_jvm_class_file.backend import (
    JvmBackendConfig,
    JVMClassArtifact,
    JvmBackendError,
    lower_ir_to_jvm_class_file,
    validate_for_jvm,
    write_class_file,
)

__all__ = [
    "JVMClassArtifact",
    "JvmBackendConfig",
    "JvmBackendError",
    "lower_ir_to_jvm_class_file",
    "validate_for_jvm",
    "write_class_file",
]
