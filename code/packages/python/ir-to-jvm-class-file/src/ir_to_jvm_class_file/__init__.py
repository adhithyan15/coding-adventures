"""Prototype backend lowering compiler-ir programs to JVM class files."""

from ir_to_jvm_class_file.backend import (
    JvmBackendConfig,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
    write_class_file,
)

__all__ = [
    "JVMClassArtifact",
    "JvmBackendConfig",
    "lower_ir_to_jvm_class_file",
    "write_class_file",
]
