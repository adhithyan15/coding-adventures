from __future__ import annotations

import pytest

from jvm_class_file import build_minimal_class_file, parse_class_file
from jvm_class_file.class_file import (
    JVMClassInfo,
    JVMFieldReference,
    JVMMethodReference,
    JVMUtf8Info,
)

HELLO_WORLD_CLASS_BYTES = bytes.fromhex(
    "cafebabe00000041001d0a000200030700040c000500060100106a6176612f6c"
    "616e672f4f626a6563740100063c696e69743e010003282956090008000907000a"
    "0c000b000c0100106a6176612f6c616e672f53797374656d0100036f7574010015"
    "4c6a6176612f696f2f5072696e7453747265616d3b08000e01000d48656c6c6f2c"
    "20776f726c64210a001000110700120c001300140100136a6176612f696f2f5072"
    "696e7453747265616d0100077072696e746c6e010015284c6a6176612f6c616e672f"
    "537472696e673b295607001601000a48656c6c6f576f726c64010004436f64650100"
    "0f4c696e654e756d6265725461626c650100046d61696e010016285b4c6a6176612f"
    "6c616e672f537472696e673b295601000a536f7572636546696c6501000f48656c6c"
    "6f576f726c642e6a61766100210015000200000000000200010005000600010017"
    "0000001d00010001000000052ab70001b100000001001800000006000100000001"
    "00090019001a00010017000000250002000100000009b20007120db6000fb10000"
    "000100180000000a000200000003000800040001001b00000002001c"
)


def test_parse_minimal_class_file() -> None:
    class_bytes = build_minimal_class_file(
        class_name="Example",
        method_name="compute",
        descriptor="()I",
        code=bytes([0x04, 0x05, 0x60, 0xAC]),
        max_stack=2,
        max_locals=0,
        constants=(300,),
    )

    parsed = parse_class_file(class_bytes)

    assert str(parsed.version) == "61.0"
    assert parsed.this_class_name == "Example"
    assert parsed.super_class_name == "java/lang/Object"
    method = parsed.find_method("compute", "()I")
    assert method is not None
    assert method.code_attribute is not None
    assert method.code_attribute.max_stack == 2
    assert method.code_attribute.code == bytes([0x04, 0x05, 0x60, 0xAC])
    assert 300 in parsed.ldc_constants().values()


def test_invalid_magic_raises() -> None:
    with pytest.raises(ValueError, match="Invalid class-file magic"):
        parse_class_file(b"nope")


def test_resolve_string_constant_and_lookup_helpers() -> None:
    class_bytes = build_minimal_class_file(
        class_name="Example",
        method_name="compute",
        descriptor="()I",
        code=bytes([0x04, 0xAC]),
        max_stack=1,
        max_locals=0,
        constants=("hello", 7),
    )
    parsed = parse_class_file(class_bytes)

    string_index = next(
        index for index, value in parsed.ldc_constants().items() if value == "hello"
    )
    assert parsed.resolve_constant(string_index) == "hello"
    assert parsed.find_method("missing") is None

    class_index = next(
        index
        for index, entry in enumerate(parsed.constant_pool)
        if isinstance(entry, JVMClassInfo)
    )
    with pytest.raises(ValueError, match="not a loadable constant"):
        parsed.resolve_constant(class_index)

    utf8_index = next(
        index
        for index, entry in enumerate(parsed.constant_pool)
        if isinstance(entry, JVMUtf8Info) and entry.value == "Example"
    )
    assert parsed.get_utf8(utf8_index) == "Example"


def test_resolve_field_and_method_references_from_real_class() -> None:
    parsed = parse_class_file(HELLO_WORLD_CLASS_BYTES)

    assert parsed.resolve_fieldref(7) == JVMFieldReference(
        class_name="java/lang/System",
        name="out",
        descriptor="Ljava/io/PrintStream;",
    )
    assert parsed.resolve_methodref(15) == JVMMethodReference(
        class_name="java/io/PrintStream",
        name="println",
        descriptor="(Ljava/lang/String;)V",
    )
    assert parsed.resolve_class_name(21) == "HelloWorld"
    assert parsed.resolve_name_and_type(17) == ("println", "(Ljava/lang/String;)V")
