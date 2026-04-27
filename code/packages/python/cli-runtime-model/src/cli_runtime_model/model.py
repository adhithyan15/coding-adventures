"""Reusable CLI runtime state for CLR execution engines."""

from __future__ import annotations

from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from enum import IntEnum, StrEnum
from typing import Protocol

_TOKEN_ROW_MASK = 0x00FF_FFFF


class CliRuntimeModelError(ValueError):
    """Raised when CLI runtime state cannot be created or mutated."""


class CliTypeKind(StrEnum):
    """Broad CLI type families used by the runtime model."""

    VOID = "void"
    PRIMITIVE = "primitive"
    REFERENCE = "reference"
    VALUE_TYPE = "value_type"
    ARRAY = "array"
    MANAGED_POINTER = "managed_pointer"
    NATIVE_INT = "native_int"


@dataclass(frozen=True)
class CliType:
    """A normalized CLI type identity."""

    name: str
    kind: CliTypeKind
    element_type: CliType | None = None

    @classmethod
    def primitive(cls, name: str) -> CliType:
        """Create a primitive CLI type."""
        return cls(name=name, kind=CliTypeKind.PRIMITIVE)

    @classmethod
    def reference(cls, name: str) -> CliType:
        """Create a reference type."""
        return cls(name=name, kind=CliTypeKind.REFERENCE)

    @classmethod
    def value_type(cls, name: str) -> CliType:
        """Create a value type."""
        return cls(name=name, kind=CliTypeKind.VALUE_TYPE)

    @classmethod
    def szarray(cls, element_type: CliType) -> CliType:
        """Create a single-dimensional zero-based array type."""
        return cls(
            name=f"{element_type.name}[]",
            kind=CliTypeKind.ARRAY,
            element_type=element_type,
        )

    @property
    def is_reference_like(self) -> bool:
        """Return whether values of this type can be represented by null/ref."""
        return self.kind in {
            CliTypeKind.REFERENCE,
            CliTypeKind.ARRAY,
            CliTypeKind.MANAGED_POINTER,
        }

    @property
    def is_void(self) -> bool:
        """Return whether this is the CLI void type."""
        return self.kind == CliTypeKind.VOID

    def is_assignable_from(self, source: CliType) -> bool:
        """Return whether a value of ``source`` can be assigned to this type."""
        if self == source:
            return True
        if self == CLI_OBJECT and source.is_reference_like:
            return True
        return self.kind == CliTypeKind.REFERENCE and self.name == "System.Object"


CLI_VOID = CliType(name="void", kind=CliTypeKind.VOID)
CLI_BOOLEAN = CliType.primitive("bool")
CLI_CHAR = CliType.primitive("char")
CLI_INT32 = CliType.primitive("int32")
CLI_INT64 = CliType.primitive("int64")
CLI_NATIVE_INT = CliType(name="native int", kind=CliTypeKind.NATIVE_INT)
CLI_STRING = CliType.reference("System.String")
CLI_OBJECT = CliType.reference("System.Object")


@dataclass(frozen=True)
class ClrHeapRef:
    """A stable managed heap reference."""

    address: int
    cli_type: CliType

    def __post_init__(self) -> None:
        if self.address <= 0:
            msg = "heap reference address must be positive"
            raise CliRuntimeModelError(msg)
        if not self.cli_type.is_reference_like:
            msg = f"heap reference type must be reference-like: {self.cli_type.name}"
            raise CliRuntimeModelError(msg)


@dataclass(frozen=True)
class CliValue:
    """A typed value carried by the CLI evaluation stack or slots."""

    cli_type: CliType
    value: object | None = None
    boxed_type: CliType | None = None

    def __post_init__(self) -> None:
        if self.cli_type.is_void:
            msg = "void is not a runtime value"
            raise CliRuntimeModelError(msg)
        if self.boxed_type is not None and not self.cli_type.is_reference_like:
            msg = "boxed values must be represented by a reference-like type"
            raise CliRuntimeModelError(msg)

    @classmethod
    def int32(cls, value: int) -> CliValue:
        """Create an int32 value."""
        return cls(cli_type=CLI_INT32, value=value)

    @classmethod
    def boolean(cls, value: bool) -> CliValue:
        """Create a CLI boolean value."""
        return cls(cli_type=CLI_BOOLEAN, value=bool(value))

    @classmethod
    def string(cls, value: str) -> CliValue:
        """Create a managed string value."""
        return cls(cli_type=CLI_STRING, value=value)

    @classmethod
    def null(cls, cli_type: CliType = CLI_OBJECT) -> CliValue:
        """Create a typed null reference."""
        if not cli_type.is_reference_like:
            msg = f"null requires a reference-like type: {cli_type.name}"
            raise CliRuntimeModelError(msg)
        return cls(cli_type=cli_type, value=None)

    @classmethod
    def heap_ref(cls, ref: ClrHeapRef) -> CliValue:
        """Create a value from a managed heap reference."""
        return cls(cli_type=ref.cli_type, value=ref)

    @property
    def is_null(self) -> bool:
        """Return whether this value is a null reference."""
        return self.value is None and self.cli_type.is_reference_like

    @property
    def heap_ref_value(self) -> ClrHeapRef:
        """Return the underlying heap reference or raise a model error."""
        if not isinstance(self.value, ClrHeapRef):
            msg = f"value is not a heap reference: {self.cli_type.name}"
            raise CliRuntimeModelError(msg)
        return self.value

    @staticmethod
    def default_for(cli_type: CliType) -> CliValue:
        """Return the CLI default value for ``cli_type``."""
        if cli_type == CLI_BOOLEAN:
            return CliValue.boolean(False)
        if cli_type in {CLI_INT32, CLI_INT64, CLI_NATIVE_INT, CLI_CHAR}:
            return CliValue(cli_type=cli_type, value=0)
        if cli_type.is_reference_like:
            return CliValue.null(cli_type)
        if cli_type.kind == CliTypeKind.VALUE_TYPE:
            return CliValue(cli_type=cli_type, value={})
        msg = f"cannot create a runtime default for {cli_type.name}"
        raise CliRuntimeModelError(msg)


@dataclass(frozen=True)
class ClrHeapObject:
    """A managed heap object or boxed value."""

    cli_type: CliType
    fields: dict[str, CliValue] = field(default_factory=dict)
    boxed_value: CliValue | None = None


class ClrHeap:
    """In-memory managed heap model used by execution engines."""

    def __init__(self) -> None:
        self._next_address = 1
        self._objects: dict[ClrHeapRef, ClrHeapObject] = {}

    def allocate(
        self,
        cli_type: CliType,
        *,
        fields: dict[str, CliValue] | None = None,
    ) -> ClrHeapRef:
        """Allocate an object and return its managed reference."""
        if not cli_type.is_reference_like:
            msg = f"cannot allocate non-reference type: {cli_type.name}"
            raise CliRuntimeModelError(msg)
        ref = ClrHeapRef(self._next_address, cli_type)
        self._next_address += 1
        self._objects[ref] = ClrHeapObject(cli_type=cli_type, fields=fields or {})
        return ref

    def box(self, value: CliValue) -> CliValue:
        """Box a value and return the object reference value."""
        ref = ClrHeapRef(
            self._next_address,
            CliType.reference(f"boxed<{value.cli_type.name}>"),
        )
        self._next_address += 1
        self._objects[ref] = ClrHeapObject(
            cli_type=ref.cli_type,
            boxed_value=value,
        )
        return CliValue(cli_type=ref.cli_type, value=ref, boxed_type=value.cli_type)

    def get(self, ref: ClrHeapRef) -> ClrHeapObject:
        """Return the heap object for ``ref``."""
        try:
            return self._objects[ref]
        except KeyError as exc:
            msg = f"unknown heap reference: {ref.address}"
            raise CliRuntimeModelError(msg) from exc

    def get_field(self, ref: ClrHeapRef, name: str) -> CliValue:
        """Read a field from a managed object."""
        obj = self.get(ref)
        try:
            return obj.fields[name]
        except KeyError as exc:
            msg = f"unknown field {name!r} on {obj.cli_type.name}"
            raise CliRuntimeModelError(msg) from exc

    def set_field(self, ref: ClrHeapRef, name: str, value: CliValue) -> None:
        """Write a field on a managed object."""
        self.get(ref).fields[name] = value

    def unbox(self, ref: ClrHeapRef, expected_type: CliType) -> CliValue:
        """Return the boxed value if it matches ``expected_type``."""
        boxed_value = self.get(ref).boxed_value
        if boxed_value is None:
            msg = f"heap reference {ref.address} does not contain a boxed value"
            raise CliRuntimeModelError(msg)
        if boxed_value.cli_type != expected_type:
            msg = (
                f"boxed value has type {boxed_value.cli_type.name}, "
                f"not {expected_type.name}"
            )
            raise CliRuntimeModelError(msg)
        return boxed_value


class CliEvaluationStack:
    """Mutable CLI evaluation stack with typed values."""

    def __init__(self, *, max_depth: int | None = None) -> None:
        if max_depth is not None and max_depth < 0:
            msg = "max_depth must be non-negative"
            raise CliRuntimeModelError(msg)
        self.max_depth = max_depth
        self._values: list[CliValue] = []

    def push(self, value: CliValue) -> None:
        """Push ``value`` onto the stack."""
        if self.max_depth is not None and len(self._values) >= self.max_depth:
            msg = f"evaluation stack overflow at max depth {self.max_depth}"
            raise CliRuntimeModelError(msg)
        self._values.append(value)

    def pop(self, expected_type: CliType | None = None) -> CliValue:
        """Pop one value, optionally checking assignability."""
        if not self._values:
            msg = "evaluation stack underflow"
            raise CliRuntimeModelError(msg)
        value = self._values.pop()
        if expected_type is not None:
            _require_assignable(expected_type, value)
        return value

    def pop_many(self, count: int) -> tuple[CliValue, ...]:
        """Pop ``count`` values and return them in call-order."""
        if count < 0:
            msg = "count must be non-negative"
            raise CliRuntimeModelError(msg)
        if count > len(self._values):
            msg = f"cannot pop {count} values from stack depth {len(self._values)}"
            raise CliRuntimeModelError(msg)
        if count == 0:
            return ()
        values = self._values[-count:]
        del self._values[-count:]
        return tuple(values)

    def peek(self) -> CliValue:
        """Return the top stack value without popping it."""
        if not self._values:
            msg = "evaluation stack is empty"
            raise CliRuntimeModelError(msg)
        return self._values[-1]

    def clear(self) -> None:
        """Remove all values from the stack."""
        self._values.clear()

    def snapshot(self) -> tuple[CliValue, ...]:
        """Return an immutable bottom-to-top stack snapshot."""
        return tuple(self._values)

    def __len__(self) -> int:
        return len(self._values)


@dataclass
class CliSlot:
    """An argument or local variable slot."""

    declared_type: CliType
    value: CliValue | None = None

    @property
    def initialized(self) -> bool:
        """Return whether the slot has been assigned."""
        return self.value is not None

    def read(self) -> CliValue:
        """Read the slot value, failing if it is uninitialized."""
        if self.value is None:
            msg = f"uninitialized {self.declared_type.name} slot"
            raise CliRuntimeModelError(msg)
        return self.value

    def store(self, value: CliValue) -> None:
        """Assign a value to this slot."""
        _require_assignable(self.declared_type, value)
        self.value = value

    def snapshot(self) -> CliValue | None:
        """Return the current slot value."""
        return self.value


@dataclass(frozen=True)
class CliMethodSignature:
    """CLI method signature used by execution state and token resolution."""

    parameter_types: tuple[CliType, ...]
    return_type: CliType = CLI_VOID
    has_this: bool = False

    @property
    def parameter_count(self) -> int:
        """Return the number of explicit parameters."""
        return len(self.parameter_types)


@dataclass(frozen=True)
class CliMethodDescriptor:
    """Resolved method identity and call-shape metadata."""

    token: int
    declaring_type: CliType
    name: str
    signature: CliMethodSignature
    is_static: bool = True
    is_virtual: bool = False

    @property
    def full_name(self) -> str:
        """Return a display name for diagnostics."""
        return f"{self.declaring_type.name}.{self.name}"

    def requires_instance(self, kind: CliCallKind | None = None) -> bool:
        """Return whether a call to this method consumes an instance value."""
        call_kind = kind or CliCallKind.CALL
        return not self.is_static or call_kind == CliCallKind.CALLVIRT


@dataclass(frozen=True)
class CliFieldDescriptor:
    """Resolved field identity."""

    token: int
    declaring_type: CliType
    name: str
    field_type: CliType
    is_static: bool = False


class CliTokenTable(IntEnum):
    """High-byte metadata token tables used by the runtime model."""

    TYPE_REF = 0x01
    TYPE_DEF = 0x02
    FIELD = 0x04
    METHOD_DEF = 0x06
    MEMBER_REF = 0x0A
    STANDALONE_SIG = 0x11
    TYPE_SPEC = 0x1B
    ASSEMBLY_REF = 0x23
    USER_STRING = 0x70


@dataclass(frozen=True)
class CliToken:
    """Decoded CLI metadata or heap token."""

    raw: int
    table: CliTokenTable
    row: int


def decode_cli_token(raw: int) -> CliToken:
    """Decode a 32-bit CLI token into table and row/index components."""
    if raw < 0 or raw > 0xFFFF_FFFF:
        msg = f"CLI token must be uint32: {raw}"
        raise CliRuntimeModelError(msg)
    table_value = (raw >> 24) & 0xFF
    try:
        table = CliTokenTable(table_value)
    except ValueError as exc:
        msg = f"unsupported CLI token table: 0x{table_value:02x}"
        raise CliRuntimeModelError(msg) from exc
    row = raw & _TOKEN_ROW_MASK
    if row == 0 and table != CliTokenTable.USER_STRING:
        msg = f"metadata token row must be non-zero: 0x{raw:08x}"
        raise CliRuntimeModelError(msg)
    return CliToken(raw=raw, table=table, row=row)


class CliTokenResolver(Protocol):
    """Resolve CLI tokens into runtime descriptors and literals."""

    def resolve_method(self, token: int) -> CliMethodDescriptor:
        """Resolve a MethodDef or MemberRef token."""

    def resolve_field(self, token: int) -> CliFieldDescriptor:
        """Resolve a Field token."""

    def resolve_type(self, token: int) -> CliType:
        """Resolve a TypeDef, TypeRef, or TypeSpec token."""

    def resolve_user_string(self, token: int) -> str:
        """Resolve a UserString token."""


class MapCliTokenResolver:
    """Map-backed token resolver for tests, adapters, and small VMs."""

    def __init__(
        self,
        *,
        methods: Iterable[CliMethodDescriptor] = (),
        fields: Iterable[CliFieldDescriptor] = (),
        types: dict[int, CliType] | None = None,
        user_strings: dict[int, str] | None = None,
    ) -> None:
        self._methods = {method.token: method for method in methods}
        self._fields = {field.token: field for field in fields}
        self._types = dict(types or {})
        self._user_strings = dict(user_strings or {})

    def add_method(self, method: CliMethodDescriptor) -> MapCliTokenResolver:
        """Register ``method`` and return this resolver."""
        self._methods[method.token] = method
        return self

    def add_field(self, field: CliFieldDescriptor) -> MapCliTokenResolver:
        """Register ``field`` and return this resolver."""
        self._fields[field.token] = field
        return self

    def add_type(self, token: int, cli_type: CliType) -> MapCliTokenResolver:
        """Register ``cli_type`` and return this resolver."""
        decoded = decode_cli_token(token)
        if decoded.table not in {
            CliTokenTable.TYPE_DEF,
            CliTokenTable.TYPE_REF,
            CliTokenTable.TYPE_SPEC,
        }:
            msg = f"token is not a type token: 0x{token:08x}"
            raise CliRuntimeModelError(msg)
        self._types[token] = cli_type
        return self

    def add_user_string(self, token: int, value: str) -> MapCliTokenResolver:
        """Register a user string and return this resolver."""
        decoded = decode_cli_token(token)
        if decoded.table != CliTokenTable.USER_STRING:
            msg = f"token is not a user string token: 0x{token:08x}"
            raise CliRuntimeModelError(msg)
        self._user_strings[token] = value
        return self

    def resolve_method(self, token: int) -> CliMethodDescriptor:
        """Resolve a MethodDef or MemberRef token."""
        decoded = decode_cli_token(token)
        if decoded.table not in {CliTokenTable.METHOD_DEF, CliTokenTable.MEMBER_REF}:
            msg = f"token is not a method token: 0x{token:08x}"
            raise CliRuntimeModelError(msg)
        try:
            return self._methods[token]
        except KeyError as exc:
            msg = f"unknown method token: 0x{token:08x}"
            raise CliRuntimeModelError(msg) from exc

    def resolve_field(self, token: int) -> CliFieldDescriptor:
        """Resolve a Field token."""
        decoded = decode_cli_token(token)
        if decoded.table != CliTokenTable.FIELD:
            msg = f"token is not a field token: 0x{token:08x}"
            raise CliRuntimeModelError(msg)
        try:
            return self._fields[token]
        except KeyError as exc:
            msg = f"unknown field token: 0x{token:08x}"
            raise CliRuntimeModelError(msg) from exc

    def resolve_type(self, token: int) -> CliType:
        """Resolve a TypeDef, TypeRef, or TypeSpec token."""
        decoded = decode_cli_token(token)
        if decoded.table not in {
            CliTokenTable.TYPE_DEF,
            CliTokenTable.TYPE_REF,
            CliTokenTable.TYPE_SPEC,
        }:
            msg = f"token is not a type token: 0x{token:08x}"
            raise CliRuntimeModelError(msg)
        try:
            return self._types[token]
        except KeyError as exc:
            msg = f"unknown type token: 0x{token:08x}"
            raise CliRuntimeModelError(msg) from exc

    def resolve_user_string(self, token: int) -> str:
        """Resolve a UserString token."""
        decoded = decode_cli_token(token)
        if decoded.table != CliTokenTable.USER_STRING:
            msg = f"token is not a user string token: 0x{token:08x}"
            raise CliRuntimeModelError(msg)
        try:
            return self._user_strings[token]
        except KeyError as exc:
            msg = f"unknown user string token: 0x{token:08x}"
            raise CliRuntimeModelError(msg) from exc


class CliCallKind(StrEnum):
    """Supported call opcodes."""

    CALL = "call"
    CALLVIRT = "callvirt"


@dataclass(frozen=True)
class CliCallArguments:
    """Values consumed for a method call."""

    method: CliMethodDescriptor
    kind: CliCallKind
    instance: CliValue | None
    parameters: tuple[CliValue, ...]


def collect_call_arguments(
    stack: CliEvaluationStack,
    method: CliMethodDescriptor,
    *,
    kind: CliCallKind = CliCallKind.CALL,
) -> CliCallArguments:
    """Pop call arguments from ``stack`` according to CLI call conventions."""
    if kind == CliCallKind.CALLVIRT and method.is_static:
        msg = f"callvirt cannot target static method {method.full_name}"
        raise CliRuntimeModelError(msg)

    parameters = stack.pop_many(method.signature.parameter_count)
    for expected_type, value in zip(
        method.signature.parameter_types,
        parameters,
        strict=True,
    ):
        _require_assignable(expected_type, value)

    instance = None
    if method.requires_instance(kind):
        instance = stack.pop()
        if kind == CliCallKind.CALLVIRT and instance.is_null:
            msg = f"callvirt target is null for {method.full_name}"
            raise CliRuntimeModelError(msg)
        if not method.declaring_type.is_assignable_from(instance.cli_type):
            msg = (
                f"instance type {instance.cli_type.name} is not assignable to "
                f"{method.declaring_type.name}"
            )
            raise CliRuntimeModelError(msg)

    return CliCallArguments(
        method=method,
        kind=kind,
        instance=instance,
        parameters=parameters,
    )


@dataclass(frozen=True)
class ClrFrameSnapshot:
    """Immutable snapshot of a CLR frame."""

    method: CliMethodDescriptor
    arguments: tuple[CliValue | None, ...]
    locals: tuple[CliValue | None, ...]
    evaluation_stack: tuple[CliValue, ...]
    instruction_pointer: int
    return_address: int | None


@dataclass
class ClrFrame:
    """A mutable CLR stack frame."""

    method: CliMethodDescriptor
    arguments: list[CliSlot]
    locals: list[CliSlot]
    evaluation_stack: CliEvaluationStack
    instruction_pointer: int = 0
    return_address: int | None = None

    @classmethod
    def create(
        cls,
        method: CliMethodDescriptor,
        arguments: Iterable[CliValue] = (),
        *,
        local_types: Iterable[CliType] = (),
        return_address: int | None = None,
    ) -> ClrFrame:
        """Create a frame with initialized arguments and empty locals."""
        argument_values = tuple(arguments)
        if len(argument_values) != method.signature.parameter_count:
            msg = (
                f"{method.full_name} expects {method.signature.parameter_count} "
                f"arguments, got {len(argument_values)}"
            )
            raise CliRuntimeModelError(msg)
        argument_slots = [
            CliSlot(declared_type=declared_type, value=value)
            for declared_type, value in zip(
                method.signature.parameter_types,
                argument_values,
                strict=True,
            )
        ]
        for slot in argument_slots:
            if slot.value is not None:
                _require_assignable(slot.declared_type, slot.value)
        return cls(
            method=method,
            arguments=argument_slots,
            locals=[CliSlot(declared_type=local_type) for local_type in local_types],
            evaluation_stack=CliEvaluationStack(),
            return_address=return_address,
        )

    def load_argument(self, index: int) -> CliValue:
        """Read argument slot ``index``."""
        return self._slot(self.arguments, index, "argument").read()

    def store_argument(self, index: int, value: CliValue) -> None:
        """Write argument slot ``index``."""
        self._slot(self.arguments, index, "argument").store(value)

    def load_local(self, index: int) -> CliValue:
        """Read local slot ``index``."""
        return self._slot(self.locals, index, "local").read()

    def store_local(self, index: int, value: CliValue) -> None:
        """Write local slot ``index``."""
        self._slot(self.locals, index, "local").store(value)

    def snapshot(self) -> ClrFrameSnapshot:
        """Return an immutable snapshot of this frame."""
        return ClrFrameSnapshot(
            method=self.method,
            arguments=tuple(slot.snapshot() for slot in self.arguments),
            locals=tuple(slot.snapshot() for slot in self.locals),
            evaluation_stack=self.evaluation_stack.snapshot(),
            instruction_pointer=self.instruction_pointer,
            return_address=self.return_address,
        )

    @staticmethod
    def _slot(slots: list[CliSlot], index: int, name: str) -> CliSlot:
        if index < 0 or index >= len(slots):
            msg = f"{name} slot index out of range: {index}"
            raise CliRuntimeModelError(msg)
        return slots[index]


@dataclass(frozen=True)
class ClrThreadSnapshot:
    """Immutable snapshot of a managed thread."""

    managed_thread_id: int
    frames: tuple[ClrFrameSnapshot, ...]
    current_exception: CliValue | None


class ClrThreadState:
    """Managed thread stack with CLR frames."""

    def __init__(self, *, managed_thread_id: int = 1) -> None:
        if managed_thread_id <= 0:
            msg = "managed_thread_id must be positive"
            raise CliRuntimeModelError(msg)
        self.managed_thread_id = managed_thread_id
        self.current_exception: CliValue | None = None
        self._frames: list[ClrFrame] = []

    @property
    def frames(self) -> tuple[ClrFrame, ...]:
        """Return current frames from bottom to top."""
        return tuple(self._frames)

    @property
    def current_frame(self) -> ClrFrame:
        """Return the top frame."""
        if not self._frames:
            msg = "thread has no current frame"
            raise CliRuntimeModelError(msg)
        return self._frames[-1]

    def push_frame(self, frame: ClrFrame) -> None:
        """Push a frame onto this thread."""
        self._frames.append(frame)

    def pop_frame(self) -> ClrFrame:
        """Pop the current frame."""
        if not self._frames:
            msg = "cannot pop from an empty thread stack"
            raise CliRuntimeModelError(msg)
        return self._frames.pop()

    def snapshot(self) -> ClrThreadSnapshot:
        """Return an immutable thread snapshot."""
        return ClrThreadSnapshot(
            managed_thread_id=self.managed_thread_id,
            frames=tuple(frame.snapshot() for frame in self._frames),
            current_exception=self.current_exception,
        )


class CliExceptionClauseKind(StrEnum):
    """CLI exception clause kinds."""

    CATCH = "catch"
    FILTER = "filter"
    FINALLY = "finally"
    FAULT = "fault"


@dataclass(frozen=True)
class CliExceptionHandler:
    """Decoded exception handling region."""

    kind: CliExceptionClauseKind
    try_start: int
    try_end: int
    handler_start: int
    handler_end: int
    catch_type: CliType | None = None
    filter_start: int | None = None

    def __post_init__(self) -> None:
        if self.try_start < 0 or self.try_end < self.try_start:
            msg = "invalid try region"
            raise CliRuntimeModelError(msg)
        if self.handler_start < 0 or self.handler_end < self.handler_start:
            msg = "invalid handler region"
            raise CliRuntimeModelError(msg)
        if self.kind == CliExceptionClauseKind.CATCH and self.catch_type is None:
            msg = "catch handlers require catch_type"
            raise CliRuntimeModelError(msg)
        if self.kind == CliExceptionClauseKind.FILTER and self.filter_start is None:
            msg = "filter handlers require filter_start"
            raise CliRuntimeModelError(msg)

    def covers(self, offset: int) -> bool:
        """Return whether ``offset`` lies in the protected try region."""
        return self.try_start <= offset < self.try_end


def find_exception_handler(
    handlers: Iterable[CliExceptionHandler],
    *,
    offset: int,
    exception_type: CliType,
    is_assignable: Callable[[CliType, CliType], bool] | None = None,
) -> CliExceptionHandler | None:
    """Find the first handler that can catch ``exception_type`` at ``offset``."""
    assignable = is_assignable or _default_exception_assignable
    for handler in handlers:
        if not handler.covers(offset):
            continue
        if handler.kind != CliExceptionClauseKind.CATCH:
            return handler
        if handler.catch_type is not None and assignable(
            handler.catch_type,
            exception_type,
        ):
            return handler
    return None


def _default_exception_assignable(target: CliType, source: CliType) -> bool:
    return target.is_assignable_from(source)


def _require_assignable(target: CliType, value: CliValue) -> None:
    if value.is_null and target.is_reference_like:
        return
    if not target.is_assignable_from(value.cli_type):
        msg = f"value type {value.cli_type.name} is not assignable to {target.name}"
        raise CliRuntimeModelError(msg)
