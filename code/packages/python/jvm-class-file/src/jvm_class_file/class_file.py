"""Minimal JVM class-file support for the simulator prototype."""

from __future__ import annotations

import struct
from dataclasses import dataclass

CONSTANT_UTF8 = 1
CONSTANT_INTEGER = 3
CONSTANT_LONG = 5
CONSTANT_DOUBLE = 6
CONSTANT_CLASS = 7
CONSTANT_STRING = 8
CONSTANT_FIELDREF = 9
CONSTANT_METHODREF = 10
CONSTANT_NAME_AND_TYPE = 12

ACC_PUBLIC = 0x0001
ACC_STATIC = 0x0008
ACC_SUPER = 0x0020


class ClassFileFormatError(ValueError):
    """Raised when bytes do not match the minimal class-file format we expect."""


@dataclass(frozen=True)
class JVMClassVersion:
    major: int
    minor: int

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}"


@dataclass(frozen=True)
class JVMUtf8Info:
    value: str


@dataclass(frozen=True)
class JVMIntegerInfo:
    value: int


@dataclass(frozen=True)
class JVMLongInfo:
    value: int


@dataclass(frozen=True)
class JVMDoubleInfo:
    value: float


@dataclass(frozen=True)
class JVMClassInfo:
    name_index: int


@dataclass(frozen=True)
class JVMStringInfo:
    string_index: int


@dataclass(frozen=True)
class JVMNameAndTypeInfo:
    name_index: int
    descriptor_index: int


@dataclass(frozen=True)
class JVMFieldrefInfo:
    class_index: int
    name_and_type_index: int


@dataclass(frozen=True)
class JVMMethodrefInfo:
    class_index: int
    name_and_type_index: int


@dataclass(frozen=True)
class JVMFieldReference:
    class_name: str
    name: str
    descriptor: str


@dataclass(frozen=True)
class JVMMethodReference:
    class_name: str
    name: str
    descriptor: str


JVMConstantPoolEntry = (
    JVMUtf8Info
    | JVMIntegerInfo
    | JVMLongInfo
    | JVMDoubleInfo
    | JVMClassInfo
    | JVMStringInfo
    | JVMNameAndTypeInfo
    | JVMFieldrefInfo
    | JVMMethodrefInfo
    | None
)


@dataclass(frozen=True)
class JVMAttributeInfo:
    name: str
    info: bytes


@dataclass(frozen=True)
class JVMCodeAttribute:
    name: str
    max_stack: int
    max_locals: int
    code: bytes
    nested_attributes: tuple[JVMAttributeInfo, ...] = ()


JVMMethodAttribute = JVMAttributeInfo | JVMCodeAttribute


@dataclass(frozen=True)
class JVMMethodInfo:
    access_flags: int
    name: str
    descriptor: str
    attributes: tuple[JVMMethodAttribute, ...]

    @property
    def code_attribute(self) -> JVMCodeAttribute | None:
        for attribute in self.attributes:
            if isinstance(attribute, JVMCodeAttribute):
                return attribute
        return None


@dataclass(frozen=True)
class JVMClassFile:
    version: JVMClassVersion
    access_flags: int
    this_class_name: str
    super_class_name: str | None
    constant_pool: tuple[JVMConstantPoolEntry, ...]
    methods: tuple[JVMMethodInfo, ...]

    def get_utf8(self, index: int) -> str:
        entry = self._entry(index)
        if not isinstance(entry, JVMUtf8Info):
            msg = f"Constant pool entry {index} is not a UTF-8 string"
            raise ClassFileFormatError(msg)
        return entry.value

    def resolve_class_name(self, index: int) -> str:
        entry = self._entry(index)
        if not isinstance(entry, JVMClassInfo):
            msg = f"Constant pool entry {index} is not a Class entry"
            raise ClassFileFormatError(msg)
        return self.get_utf8(entry.name_index)

    def resolve_name_and_type(self, index: int) -> tuple[str, str]:
        entry = self._entry(index)
        if not isinstance(entry, JVMNameAndTypeInfo):
            msg = f"Constant pool entry {index} is not a NameAndType entry"
            raise ClassFileFormatError(msg)
        return (
            self.get_utf8(entry.name_index),
            self.get_utf8(entry.descriptor_index),
        )

    def resolve_constant(self, index: int) -> int | float | str:
        entry = self._entry(index)

        if isinstance(entry, JVMUtf8Info):
            return entry.value
        if isinstance(entry, JVMIntegerInfo):
            return entry.value
        if isinstance(entry, JVMLongInfo):
            return entry.value
        if isinstance(entry, JVMDoubleInfo):
            return entry.value
        if isinstance(entry, JVMStringInfo):
            return self.get_utf8(entry.string_index)

        msg = f"Constant pool entry {index} is not a loadable constant: {entry!r}"
        raise ClassFileFormatError(msg)

    def resolve_fieldref(self, index: int) -> JVMFieldReference:
        entry = self._entry(index)
        if not isinstance(entry, JVMFieldrefInfo):
            msg = f"Constant pool entry {index} is not a Fieldref entry"
            raise ClassFileFormatError(msg)
        name, descriptor = self.resolve_name_and_type(entry.name_and_type_index)
        return JVMFieldReference(
            class_name=self.resolve_class_name(entry.class_index),
            name=name,
            descriptor=descriptor,
        )

    def resolve_methodref(self, index: int) -> JVMMethodReference:
        entry = self._entry(index)
        if not isinstance(entry, JVMMethodrefInfo):
            msg = f"Constant pool entry {index} is not a Methodref entry"
            raise ClassFileFormatError(msg)
        name, descriptor = self.resolve_name_and_type(entry.name_and_type_index)
        return JVMMethodReference(
            class_name=self.resolve_class_name(entry.class_index),
            name=name,
            descriptor=descriptor,
        )

    def ldc_constants(self) -> dict[int, int | str]:
        lookup: dict[int, int | str] = {}
        for index in range(1, len(self.constant_pool)):
            entry = self.constant_pool[index]
            if isinstance(entry, JVMIntegerInfo):
                lookup[index] = entry.value
            elif isinstance(entry, JVMStringInfo):
                lookup[index] = self.get_utf8(entry.string_index)
        return lookup

    def find_method(
        self,
        name: str,
        descriptor: str | None = None,
    ) -> JVMMethodInfo | None:
        for method in self.methods:
            if method.name != name:
                continue
            if descriptor is not None and method.descriptor != descriptor:
                continue
            return method
        return None

    def _entry(self, index: int) -> JVMConstantPoolEntry:
        if index <= 0 or index >= len(self.constant_pool):
            msg = f"Constant pool index {index} is out of range"
            raise ClassFileFormatError(msg)
        entry = self.constant_pool[index]
        if entry is None:
            msg = f"Constant pool index {index} points at a reserved wide slot"
            raise ClassFileFormatError(msg)
        return entry


class _ClassReader:
    def __init__(self, data: bytes) -> None:
        self._data = data
        self._offset = 0

    @property
    def remaining(self) -> int:
        return len(self._data) - self._offset

    def read(self, length: int) -> bytes:
        end = self._offset + length
        if end > len(self._data):
            msg = "Unexpected end of class-file data"
            raise ClassFileFormatError(msg)
        chunk = self._data[self._offset : end]
        self._offset = end
        return chunk

    def u1(self) -> int:
        return self.read(1)[0]

    def u2(self) -> int:
        return int.from_bytes(self.read(2), byteorder="big", signed=False)

    def u4(self) -> int:
        return int.from_bytes(self.read(4), byteorder="big", signed=False)


def parse_class_file(data: bytes) -> JVMClassFile:
    reader = _ClassReader(data)

    magic = reader.u4()
    if magic != 0xCAFEBABE:
        msg = f"Invalid class-file magic: 0x{magic:08X}"
        raise ClassFileFormatError(msg)

    minor = reader.u2()
    major = reader.u2()
    version = JVMClassVersion(major=major, minor=minor)

    constant_pool_count = reader.u2()
    constant_pool: list[JVMConstantPoolEntry] = [None] * constant_pool_count
    index = 1
    while index < constant_pool_count:
        tag = reader.u1()
        if tag == CONSTANT_UTF8:
            length = reader.u2()
            constant_pool[index] = JVMUtf8Info(reader.read(length).decode("utf-8"))
        elif tag == CONSTANT_INTEGER:
            constant_pool[index] = JVMIntegerInfo(
                struct.unpack(">i", reader.read(4))[0]
            )
        elif tag == CONSTANT_LONG:
            constant_pool[index] = JVMLongInfo(
                struct.unpack(">q", reader.read(8))[0]
            )
            index += 1
        elif tag == CONSTANT_DOUBLE:
            constant_pool[index] = JVMDoubleInfo(
                struct.unpack(">d", reader.read(8))[0]
            )
            index += 1
        elif tag == CONSTANT_CLASS:
            constant_pool[index] = JVMClassInfo(name_index=reader.u2())
        elif tag == CONSTANT_STRING:
            constant_pool[index] = JVMStringInfo(string_index=reader.u2())
        elif tag == CONSTANT_NAME_AND_TYPE:
            constant_pool[index] = JVMNameAndTypeInfo(
                name_index=reader.u2(),
                descriptor_index=reader.u2(),
            )
        elif tag == CONSTANT_FIELDREF:
            constant_pool[index] = JVMFieldrefInfo(
                class_index=reader.u2(),
                name_and_type_index=reader.u2(),
            )
        elif tag == CONSTANT_METHODREF:
            constant_pool[index] = JVMMethodrefInfo(
                class_index=reader.u2(),
                name_and_type_index=reader.u2(),
            )
        else:
            msg = f"Unsupported constant-pool tag {tag} at index {index}"
            raise ClassFileFormatError(msg)
        index += 1

    access_flags = reader.u2()
    this_class = _resolve_class_name(constant_pool, reader.u2())
    super_class_index = reader.u2()
    super_class = (
        None
        if super_class_index == 0
        else _resolve_class_name(constant_pool, super_class_index)
    )

    interfaces_count = reader.u2()
    for _ in range(interfaces_count):
        reader.u2()

    fields_count = reader.u2()
    for _ in range(fields_count):
        _skip_member(reader)

    methods_count = reader.u2()
    methods = tuple(_parse_method(reader, constant_pool) for _ in range(methods_count))

    class_attributes_count = reader.u2()
    for _ in range(class_attributes_count):
        _parse_attribute(reader, constant_pool)

    if reader.remaining != 0:
        msg = f"Trailing bytes after class-file parse: {reader.remaining}"
        raise ClassFileFormatError(msg)

    return JVMClassFile(
        version=version,
        access_flags=access_flags,
        this_class_name=this_class,
        super_class_name=super_class,
        constant_pool=tuple(constant_pool),
        methods=methods,
    )


def build_minimal_class_file(
    *,
    class_name: str,
    method_name: str,
    descriptor: str,
    code: bytes,
    max_stack: int,
    max_locals: int,
    constants: tuple[int | str, ...] = (),
    major_version: int = 61,
    minor_version: int = 0,
    class_access_flags: int = ACC_PUBLIC | ACC_SUPER,
    method_access_flags: int = ACC_PUBLIC | ACC_STATIC,
    super_class_name: str = "java/lang/Object",
) -> bytes:
    entries: list[bytes] = []
    indices: dict[tuple[object, ...], int] = {}

    def add_entry(key: tuple[object, ...], payload: bytes) -> int:
        if key in indices:
            return indices[key]
        entries.append(payload)
        index = len(entries)
        indices[key] = index
        return index

    def add_utf8(value: str) -> int:
        encoded = value.encode("utf-8")
        return add_entry(
            ("Utf8", value),
            bytes([CONSTANT_UTF8]) + len(encoded).to_bytes(2, "big") + encoded,
        )

    def add_class(value: str) -> int:
        name_index = add_utf8(value)
        return add_entry(
            ("Class", value),
            bytes([CONSTANT_CLASS]) + name_index.to_bytes(2, "big"),
        )

    def add_constant(value: int | str) -> int:
        if isinstance(value, int):
            return add_entry(
                ("Integer", value),
                bytes([CONSTANT_INTEGER]) + struct.pack(">i", value),
            )
        string_index = add_utf8(value)
        return add_entry(
            ("String", value),
            bytes([CONSTANT_STRING]) + string_index.to_bytes(2, "big"),
        )

    this_class_index = add_class(class_name)
    super_class_index = add_class(super_class_name)
    method_name_index = add_utf8(method_name)
    descriptor_index = add_utf8(descriptor)
    code_name_index = add_utf8("Code")

    for constant in constants:
        add_constant(constant)

    code_attribute_body = b"".join(
        [
            max_stack.to_bytes(2, "big"),
            max_locals.to_bytes(2, "big"),
            len(code).to_bytes(4, "big"),
            code,
            (0).to_bytes(2, "big"),
            (0).to_bytes(2, "big"),
        ]
    )
    code_attribute = b"".join(
        [
            code_name_index.to_bytes(2, "big"),
            len(code_attribute_body).to_bytes(4, "big"),
            code_attribute_body,
        ]
    )

    method_info = b"".join(
        [
            method_access_flags.to_bytes(2, "big"),
            method_name_index.to_bytes(2, "big"),
            descriptor_index.to_bytes(2, "big"),
            (1).to_bytes(2, "big"),
            code_attribute,
        ]
    )

    return b"".join(
        [
            (0xCAFEBABE).to_bytes(4, "big"),
            minor_version.to_bytes(2, "big"),
            major_version.to_bytes(2, "big"),
            (len(entries) + 1).to_bytes(2, "big"),
            b"".join(entries),
            class_access_flags.to_bytes(2, "big"),
            this_class_index.to_bytes(2, "big"),
            super_class_index.to_bytes(2, "big"),
            (0).to_bytes(2, "big"),
            (0).to_bytes(2, "big"),
            (1).to_bytes(2, "big"),
            method_info,
            (0).to_bytes(2, "big"),
        ]
    )


def _resolve_class_name(
    constant_pool: list[JVMConstantPoolEntry],
    class_index: int,
) -> str:
    if class_index <= 0 or class_index >= len(constant_pool):
        msg = f"Class constant-pool index {class_index} is out of range"
        raise ClassFileFormatError(msg)
    entry = constant_pool[class_index]
    if not isinstance(entry, JVMClassInfo):
        msg = f"Constant pool entry {class_index} is not a Class entry"
        raise ClassFileFormatError(msg)
    name_entry = constant_pool[entry.name_index]
    if not isinstance(name_entry, JVMUtf8Info):
        msg = f"Class name entry {entry.name_index} is not UTF-8"
        raise ClassFileFormatError(msg)
    return name_entry.value


def _parse_method(
    reader: _ClassReader,
    constant_pool: list[JVMConstantPoolEntry],
) -> JVMMethodInfo:
    access_flags = reader.u2()
    name = _get_utf8(constant_pool, reader.u2())
    descriptor = _get_utf8(constant_pool, reader.u2())
    attributes_count = reader.u2()
    attributes = tuple(
        _parse_attribute(reader, constant_pool) for _ in range(attributes_count)
    )
    return JVMMethodInfo(
        access_flags=access_flags,
        name=name,
        descriptor=descriptor,
        attributes=attributes,
    )


def _parse_attribute(
    reader: _ClassReader,
    constant_pool: list[JVMConstantPoolEntry],
) -> JVMMethodAttribute:
    name = _get_utf8(constant_pool, reader.u2())
    length = reader.u4()
    payload = reader.read(length)

    if name != "Code":
        return JVMAttributeInfo(name=name, info=payload)

    code_reader = _ClassReader(payload)
    max_stack = code_reader.u2()
    max_locals = code_reader.u2()
    code_length = code_reader.u4()
    code = code_reader.read(code_length)

    exception_table_length = code_reader.u2()
    for _ in range(exception_table_length):
        code_reader.read(8)

    nested_attributes_count = code_reader.u2()
    nested_attributes = tuple(
        _parse_attribute(code_reader, constant_pool)
        for _ in range(nested_attributes_count)
    )

    if code_reader.remaining != 0:
        msg = "Code attribute contained trailing bytes after parsing"
        raise ClassFileFormatError(msg)

    raw_nested_attributes = tuple(
        attribute
        for attribute in nested_attributes
        if isinstance(attribute, JVMAttributeInfo)
    )

    return JVMCodeAttribute(
        name=name,
        max_stack=max_stack,
        max_locals=max_locals,
        code=code,
        nested_attributes=raw_nested_attributes,
    )


def _get_utf8(constant_pool: list[JVMConstantPoolEntry], index: int) -> str:
    if index <= 0 or index >= len(constant_pool):
        msg = f"UTF-8 constant-pool index {index} is out of range"
        raise ClassFileFormatError(msg)
    entry = constant_pool[index]
    if not isinstance(entry, JVMUtf8Info):
        msg = f"Constant pool entry {index} is not UTF-8"
        raise ClassFileFormatError(msg)
    return entry.value


def _skip_member(reader: _ClassReader) -> None:
    reader.u2()
    reader.u2()
    reader.u2()
    attributes_count = reader.u2()
    for _ in range(attributes_count):
        reader.u2()
        attribute_length = reader.u4()
        reader.read(attribute_length)
