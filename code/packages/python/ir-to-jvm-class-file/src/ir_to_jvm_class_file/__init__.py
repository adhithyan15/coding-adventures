"""Prototype backend lowering compiler-ir programs to JVM class files.

LANG20: ``JVMCodeGenerator`` implements ``CodeGenerator[IrProgram, JVMClassArtifact]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.
"""

from ir_to_jvm_class_file.backend import (
    CLOSURE_INTERFACE_BINARY_NAME,
    CLOSURE_INTERFACE_METHOD_DESCRIPTOR,
    CLOSURE_INTERFACE_METHOD_NAME,
    CONS_BINARY_NAME,
    NIL_BINARY_NAME,
    SYMBOL_BINARY_NAME,
    TWIG_RUNTIME_BINARY_NAME,
    JvmBackendConfig,
    JvmBackendError,
    JVMClassArtifact,
    JVMMultiClassArtifact,
    build_closure_interface_artifact,
    build_closure_subclass_artifact,
    build_cons_class_artifact,
    build_nil_class_artifact,
    build_runtime_class_artifact,
    build_symbol_class_artifact,
    lower_ir_to_jvm_class_file,
    lower_ir_to_jvm_classes,
    validate_for_jvm,
    write_class_file,
)
from ir_to_jvm_class_file.generator import JVMCodeGenerator

__all__ = [
    "CLOSURE_INTERFACE_BINARY_NAME",
    "CLOSURE_INTERFACE_METHOD_DESCRIPTOR",
    "CLOSURE_INTERFACE_METHOD_NAME",
    "CONS_BINARY_NAME",
    "JVMClassArtifact",
    "JVMCodeGenerator",
    "JVMMultiClassArtifact",
    "JvmBackendConfig",
    "JvmBackendError",
    "NIL_BINARY_NAME",
    "SYMBOL_BINARY_NAME",
    "TWIG_RUNTIME_BINARY_NAME",
    "build_closure_interface_artifact",
    "build_closure_subclass_artifact",
    "build_cons_class_artifact",
    "build_nil_class_artifact",
    "build_runtime_class_artifact",
    "build_symbol_class_artifact",
    "lower_ir_to_jvm_class_file",
    "lower_ir_to_jvm_classes",
    "validate_for_jvm",
    "write_class_file",
]
