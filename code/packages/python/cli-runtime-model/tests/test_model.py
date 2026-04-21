from __future__ import annotations

import pytest

from cli_runtime_model import (
    CLI_BOOLEAN,
    CLI_CHAR,
    CLI_INT32,
    CLI_INT64,
    CLI_NATIVE_INT,
    CLI_OBJECT,
    CLI_STRING,
    CLI_VOID,
    CliCallKind,
    CliEvaluationStack,
    CliExceptionClauseKind,
    CliExceptionHandler,
    CliFieldDescriptor,
    CliMethodDescriptor,
    CliMethodSignature,
    CliRuntimeModelError,
    CliTokenTable,
    CliType,
    CliTypeKind,
    CliValue,
    ClrFrame,
    ClrHeap,
    ClrHeapRef,
    ClrThreadState,
    MapCliTokenResolver,
    collect_call_arguments,
    decode_cli_token,
    find_exception_handler,
)


def test_type_and_default_value_helpers_cover_core_cli_shapes() -> None:
    custom_value = CliType.value_type("Point")
    int_array = CliType.szarray(CLI_INT32)

    assert custom_value.kind == CliTypeKind.VALUE_TYPE
    assert int_array.name == "int32[]"
    assert int_array.element_type == CLI_INT32
    assert CLI_OBJECT.is_assignable_from(CLI_STRING)
    assert CliValue.boolean(1) == CliValue(CLI_BOOLEAN, True)
    assert CliValue.default_for(CLI_BOOLEAN) == CliValue.boolean(False)
    assert CliValue.default_for(CLI_CHAR) == CliValue(CLI_CHAR, 0)
    assert CliValue.default_for(CLI_INT64) == CliValue(CLI_INT64, 0)
    assert CliValue.default_for(CLI_NATIVE_INT) == CliValue(CLI_NATIVE_INT, 0)
    assert CliValue.default_for(CLI_STRING).is_null
    assert CliValue.default_for(custom_value) == CliValue(custom_value, {})

    with pytest.raises(CliRuntimeModelError, match="void is not"):
        CliValue(CLI_VOID)
    with pytest.raises(CliRuntimeModelError, match="boxed values"):
        CliValue(CLI_INT32, 1, boxed_type=CLI_INT32)
    with pytest.raises(CliRuntimeModelError, match="cannot create"):
        CliValue.default_for(CLI_VOID)


def test_heap_reference_and_value_validation_errors_are_deterministic() -> None:
    with pytest.raises(CliRuntimeModelError, match="must be positive"):
        ClrHeapRef(0, CLI_OBJECT)
    with pytest.raises(CliRuntimeModelError, match="reference-like"):
        ClrHeapRef(1, CLI_INT32)
    with pytest.raises(CliRuntimeModelError, match="not a heap reference"):
        _ = CliValue.int32(1).heap_ref_value


def test_evaluation_stack_preserves_typed_values_and_bounds() -> None:
    stack = CliEvaluationStack(max_depth=2)
    stack.push(CliValue.int32(10))
    stack.push(CliValue.string("hello"))

    with pytest.raises(CliRuntimeModelError, match="overflow"):
        stack.push(CliValue.int32(30))

    assert stack.peek() == CliValue.string("hello")
    assert stack.pop(CLI_STRING) == CliValue.string("hello")
    assert stack.pop_many(0) == ()
    assert stack.snapshot() == (CliValue.int32(10),)
    assert stack.pop(CLI_INT32) == CliValue.int32(10)

    with pytest.raises(CliRuntimeModelError, match="underflow"):
        stack.pop()

    with pytest.raises(CliRuntimeModelError, match="non-negative"):
        CliEvaluationStack(max_depth=-1)

    stack.push(CliValue.int32(1))
    with pytest.raises(CliRuntimeModelError, match="count must"):
        stack.pop_many(-1)
    with pytest.raises(CliRuntimeModelError, match="cannot pop 2"):
        stack.pop_many(2)
    stack.clear()
    assert len(stack) == 0
    with pytest.raises(CliRuntimeModelError, match="empty"):
        stack.peek()


def test_frame_and_thread_state_expose_immutable_snapshots() -> None:
    method = CliMethodDescriptor(
        token=0x06000001,
        declaring_type=CliType.reference("Program"),
        name="Main",
        signature=CliMethodSignature((CLI_INT32,), return_type=CLI_INT32),
    )
    frame = ClrFrame.create(
        method,
        [CliValue.int32(7)],
        local_types=[CLI_INT32, CLI_STRING],
        return_address=99,
    )

    assert frame.load_argument(0) == CliValue.int32(7)
    with pytest.raises(CliRuntimeModelError, match="uninitialized"):
        frame.load_local(0)

    frame.store_local(0, CliValue.int32(11))
    frame.evaluation_stack.push(CliValue.int32(18))
    frame.instruction_pointer = 12

    thread = ClrThreadState(managed_thread_id=42)
    thread.push_frame(frame)
    snapshot = thread.snapshot()

    assert snapshot.managed_thread_id == 42
    assert snapshot.frames[0].arguments == (CliValue.int32(7),)
    assert snapshot.frames[0].locals[0] == CliValue.int32(11)
    assert snapshot.frames[0].evaluation_stack == (CliValue.int32(18),)
    assert snapshot.frames[0].instruction_pointer == 12
    assert snapshot.frames[0].return_address == 99
    assert thread.frames == (frame,)
    assert thread.current_frame is frame
    assert thread.pop_frame() is frame

    with pytest.raises(CliRuntimeModelError, match="current frame"):
        _ = thread.current_frame
    with pytest.raises(CliRuntimeModelError, match="cannot pop"):
        thread.pop_frame()
    with pytest.raises(CliRuntimeModelError, match="positive"):
        ClrThreadState(managed_thread_id=0)
    with pytest.raises(CliRuntimeModelError, match="got 0"):
        ClrFrame.create(method)
    with pytest.raises(CliRuntimeModelError, match="out of range"):
        frame.load_argument(4)
    with pytest.raises(CliRuntimeModelError, match="out of range"):
        frame.store_argument(4, CliValue.int32(1))


def test_heap_allocates_objects_fields_and_boxed_values() -> None:
    heap = ClrHeap()
    point_type = CliType.reference("Point")
    point_ref = heap.allocate(point_type, fields={"x": CliValue.int32(1)})

    assert heap.get_field(point_ref, "x") == CliValue.int32(1)
    heap.set_field(point_ref, "x", CliValue.int32(5))
    assert heap.get_field(point_ref, "x") == CliValue.int32(5)

    boxed = heap.box(CliValue.int32(123))
    assert boxed.boxed_type == CLI_INT32
    assert heap.unbox(boxed.heap_ref_value, CLI_INT32) == CliValue.int32(123)

    with pytest.raises(CliRuntimeModelError, match="not int64"):
        heap.unbox(boxed.heap_ref_value, CliType.primitive("int64"))
    with pytest.raises(CliRuntimeModelError, match="non-reference"):
        heap.allocate(CLI_INT32)
    with pytest.raises(CliRuntimeModelError, match="unknown heap reference"):
        heap.get(ClrHeapRef(999, point_type))
    with pytest.raises(CliRuntimeModelError, match="unknown field"):
        heap.get_field(point_ref, "missing")
    with pytest.raises(CliRuntimeModelError, match="does not contain"):
        heap.unbox(point_ref, CLI_INT32)


def test_map_token_resolver_validates_token_kinds() -> None:
    method = CliMethodDescriptor(
        token=0x0A000001,
        declaring_type=CliType.reference("System.Console"),
        name="WriteLine",
        signature=CliMethodSignature((CLI_STRING,), return_type=CLI_VOID),
    )
    field = CliFieldDescriptor(
        token=0x04000001,
        declaring_type=CliType.reference("Program"),
        name="counter",
        field_type=CLI_INT32,
        is_static=True,
    )
    resolver = MapCliTokenResolver(
        methods=[method],
        fields=[field],
        types={0x01000001: CLI_STRING},
        user_strings={0x70000001: "hello"},
    )

    token = decode_cli_token(0x0A000001)
    assert token.table == CliTokenTable.MEMBER_REF
    assert token.row == 1
    assert resolver.resolve_method(0x0A000001) is method
    assert resolver.resolve_field(0x04000001) is field
    assert resolver.resolve_type(0x01000001) == CLI_STRING
    assert resolver.resolve_user_string(0x70000001) == "hello"

    with pytest.raises(CliRuntimeModelError, match="not a method token"):
        resolver.resolve_method(0x70000001)

    resolver.add_method(method).add_field(field)
    resolver.add_type(0x02000001, CliType.reference("Program"))
    resolver.add_user_string(0x70000002, "world")
    assert resolver.resolve_type(0x02000001).name == "Program"
    assert resolver.resolve_user_string(0x70000002) == "world"

    for bad_token in (-1, 0x1_0000_0000, 0x99000001, 0x06000000):
        with pytest.raises(CliRuntimeModelError):
            decode_cli_token(bad_token)
    with pytest.raises(CliRuntimeModelError, match="not a type token"):
        resolver.add_type(0x06000001, CLI_OBJECT)
    with pytest.raises(CliRuntimeModelError, match="not a user string token"):
        resolver.add_user_string(0x06000001, "bad")
    with pytest.raises(CliRuntimeModelError, match="unknown method"):
        resolver.resolve_method(0x06000001)
    with pytest.raises(CliRuntimeModelError, match="not a field token"):
        resolver.resolve_field(0x06000001)
    with pytest.raises(CliRuntimeModelError, match="unknown field"):
        resolver.resolve_field(0x04000002)
    with pytest.raises(CliRuntimeModelError, match="not a type token"):
        resolver.resolve_type(0x04000001)
    with pytest.raises(CliRuntimeModelError, match="unknown type"):
        resolver.resolve_type(0x01000002)
    with pytest.raises(CliRuntimeModelError, match="not a user string token"):
        resolver.resolve_user_string(0x01000001)
    with pytest.raises(CliRuntimeModelError, match="unknown user string"):
        resolver.resolve_user_string(0x70000003)


def test_collect_call_arguments_models_call_and_callvirt() -> None:
    program_type = CliType.reference("Program")
    instance_method = CliMethodDescriptor(
        token=0x06000001,
        declaring_type=program_type,
        name="Add",
        signature=CliMethodSignature((CLI_INT32, CLI_INT32), return_type=CLI_INT32),
        is_static=False,
        is_virtual=True,
    )
    heap = ClrHeap()
    receiver = CliValue.heap_ref(heap.allocate(program_type))
    stack = CliEvaluationStack()
    stack.push(receiver)
    stack.push(CliValue.int32(2))
    stack.push(CliValue.int32(3))

    args = collect_call_arguments(
        stack,
        instance_method,
        kind=CliCallKind.CALLVIRT,
    )

    assert args.instance == receiver
    assert args.parameters == (CliValue.int32(2), CliValue.int32(3))
    assert stack.snapshot() == ()

    stack.push(CliValue.null(program_type))
    stack.push(CliValue.int32(2))
    stack.push(CliValue.int32(3))
    with pytest.raises(CliRuntimeModelError, match="target is null"):
        collect_call_arguments(stack, instance_method, kind=CliCallKind.CALLVIRT)

    static_method = CliMethodDescriptor(
        token=0x06000002,
        declaring_type=program_type,
        name="Static",
        signature=CliMethodSignature(()),
    )
    with pytest.raises(CliRuntimeModelError, match="static method"):
        collect_call_arguments(
            CliEvaluationStack(),
            static_method,
            kind=CliCallKind.CALLVIRT,
        )

    wrong_receiver = CliEvaluationStack()
    wrong_receiver.push(CliValue.string("not a Program"))
    wrong_receiver.push(CliValue.int32(2))
    wrong_receiver.push(CliValue.int32(3))
    with pytest.raises(CliRuntimeModelError, match="instance type"):
        collect_call_arguments(wrong_receiver, instance_method)

    wrong_parameter = CliEvaluationStack()
    wrong_parameter.push(receiver)
    wrong_parameter.push(CliValue.string("bad"))
    wrong_parameter.push(CliValue.int32(3))
    with pytest.raises(CliRuntimeModelError, match="not assignable"):
        collect_call_arguments(wrong_parameter, instance_method)


def test_exception_handler_lookup_matches_protected_regions_and_types() -> None:
    invalid_operation = CliType.reference("System.InvalidOperationException")
    handlers = (
        CliExceptionHandler(
            kind=CliExceptionClauseKind.CATCH,
            try_start=0,
            try_end=10,
            handler_start=20,
            handler_end=30,
            catch_type=invalid_operation,
        ),
        CliExceptionHandler(
            kind=CliExceptionClauseKind.FINALLY,
            try_start=10,
            try_end=20,
            handler_start=30,
            handler_end=35,
        ),
    )

    assert (
        find_exception_handler(
            handlers,
            offset=5,
            exception_type=invalid_operation,
        )
        == handlers[0]
    )
    assert (
        find_exception_handler(
            handlers,
            offset=15,
            exception_type=CliType.reference("AnyException"),
        )
        == handlers[1]
    )
    assert (
        find_exception_handler(
            handlers,
            offset=40,
            exception_type=invalid_operation,
        )
        is None
    )

    object_handler = CliExceptionHandler(
        kind=CliExceptionClauseKind.CATCH,
        try_start=0,
        try_end=10,
        handler_start=40,
        handler_end=50,
        catch_type=CLI_OBJECT,
    )
    assert (
        find_exception_handler(
            [object_handler],
            offset=2,
            exception_type=invalid_operation,
            is_assignable=lambda target, source: target == CLI_OBJECT
            and source == invalid_operation,
        )
        == object_handler
    )

    with pytest.raises(CliRuntimeModelError, match="invalid try"):
        CliExceptionHandler(
            kind=CliExceptionClauseKind.FINALLY,
            try_start=5,
            try_end=4,
            handler_start=0,
            handler_end=1,
        )
    with pytest.raises(CliRuntimeModelError, match="invalid handler"):
        CliExceptionHandler(
            kind=CliExceptionClauseKind.FINALLY,
            try_start=0,
            try_end=1,
            handler_start=5,
            handler_end=4,
        )
    with pytest.raises(CliRuntimeModelError, match="catch_type"):
        CliExceptionHandler(
            kind=CliExceptionClauseKind.CATCH,
            try_start=0,
            try_end=1,
            handler_start=2,
            handler_end=3,
        )
    with pytest.raises(CliRuntimeModelError, match="filter_start"):
        CliExceptionHandler(
            kind=CliExceptionClauseKind.FILTER,
            try_start=0,
            try_end=1,
            handler_start=2,
            handler_end=3,
        )


def test_assignability_rejects_non_reference_nulls_and_wrong_slot_types() -> None:
    with pytest.raises(CliRuntimeModelError, match="null requires"):
        CliValue.null(CLI_INT32)

    frame = ClrFrame.create(
        CliMethodDescriptor(
            token=0x06000002,
            declaring_type=CliType.reference("Program"),
            name="Store",
            signature=CliMethodSignature(()),
        ),
        local_types=[CLI_INT32],
    )
    with pytest.raises(CliRuntimeModelError, match="not assignable"):
        frame.store_local(0, CliValue.null(CLI_OBJECT))
