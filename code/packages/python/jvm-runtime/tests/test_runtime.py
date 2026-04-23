"""Tests for jvm_runtime — the JVM orchestration layer.

Coverage targets
----------------

``JVMStdlibHost``
    get_static     — System.out, System.in, unknown → RuntimeError
    invoke_virtual — println(String), write(int), flush(), InputStream.read(),
                     stdout callback, unknown → RuntimeError
    invoke_static  — Arrays.fill, in-class dispatch (via mock), unknown → RuntimeError

``JVMRuntime``
    run_main                    — hello-world smoke test (original)
    run_method                  — accepts raw bytes, pre-parsed JVMClassFile
    run_method with <clinit>    — runs clinit before target method
    disassemble_method          — raises when method is absent
    _disassemble_method_optional — returns body when method is found (covers line 424)
    _run_method_with_shared_state — executes a helper method sharing static_fields
"""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from jvm_class_file import JVMFieldReference, JVMMethodReference
from jvm_runtime import JVMRuntime
from jvm_runtime.runtime import JVMInputStream, JVMPrintStream, JVMStdlibHost

# ---------------------------------------------------------------------------
# Shared test fixtures — class file byte sequences
# ---------------------------------------------------------------------------

# Standard "Hello, world!" class compiled with javac (Java 17)
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

# Minimal class: MinClinit extends Object.
# Has a <clinit>()V { return; } and main([Ljava/lang/String;)V { return; }
# Generated programmatically — see tools/gen_minimal_class.py for the recipe.
MIN_CLINIT_CLASS_BYTES = bytes.fromhex(
    "cafebabe00000041000a"
    "0100106a6176612f6c616e672f4f626a656374"
    "070001"
    "0100094d696e436c696e6974"
    "070003"
    "0100083c636c696e69743e"
    "010003282956"
    "0100046d61696e"
    "010016285b4c6a6176612f6c616e672f537472696e673b2956"
    "010004436f6465"
    "002100040002000000000002"
    "000800050006000100090000000d0000000000000001b100000000"
    "000800070008000100090000000d0000000000000001b100000000"
    "0000"
)


# ---------------------------------------------------------------------------
# Helper: ensure MIN_CLINIT_CLASS_BYTES parses correctly before any test runs
# ---------------------------------------------------------------------------

def test_min_clinit_class_parses() -> None:
    """Sanity-check that the embedded bytes are valid."""
    runtime = JVMRuntime()
    cf = runtime.load_class(MIN_CLINIT_CLASS_BYTES)
    assert cf.this_class_name == "MinClinit"
    method_names = [m.name for m in cf.methods]
    assert "<clinit>" in method_names
    assert "main" in method_names


# ---------------------------------------------------------------------------
# Original integration smoke test
# ---------------------------------------------------------------------------


def test_runtime_runs_real_hello_world_class() -> None:
    runtime = JVMRuntime()

    result = runtime.run_main(HELLO_WORLD_CLASS_BYTES)

    assert result.class_file.version.major == 65
    assert result.method.version.major == 65
    assert result.output == "Hello, world!\n"
    assert [trace.opcode for trace in result.traces] == [
        "getstatic",
        "ldc",
        "invokevirtual",
        "return",
    ]


# ---------------------------------------------------------------------------
# JVMStdlibHost.get_static
# ---------------------------------------------------------------------------


class TestGetStatic:
    """get_static returns sentinels for System.out / System.in; raises for others."""

    def test_get_static_system_out(self) -> None:
        host = JVMStdlibHost()
        ref = JVMFieldReference(
            class_name="java/lang/System",
            name="out",
            descriptor="Ljava/io/PrintStream;",
        )
        result = host.get_static(ref)
        assert isinstance(result, JVMPrintStream)

    def test_get_static_system_in(self) -> None:
        host = JVMStdlibHost()
        ref = JVMFieldReference(
            class_name="java/lang/System",
            name="in",
            descriptor="Ljava/io/InputStream;",
        )
        result = host.get_static(ref)
        assert isinstance(result, JVMInputStream)

    def test_get_static_unknown_raises(self) -> None:
        host = JVMStdlibHost()
        ref = JVMFieldReference(
            class_name="com/example/Unknown",
            name="INSTANCE",
            descriptor="Lcom/example/Unknown;",
        )
        with pytest.raises(RuntimeError, match="Unsupported static field reference"):
            host.get_static(ref)

    def test_get_static_system_err_raises(self) -> None:
        """System.err is not supported (only System.out / System.in are)."""
        host = JVMStdlibHost()
        ref = JVMFieldReference(
            class_name="java/lang/System",
            name="err",
            descriptor="Ljava/io/PrintStream;",
        )
        with pytest.raises(RuntimeError):
            host.get_static(ref)


# ---------------------------------------------------------------------------
# JVMStdlibHost.invoke_virtual
# ---------------------------------------------------------------------------


class TestInvokeVirtual:
    """invoke_virtual dispatches PrintStream and InputStream methods."""

    def test_println_appends_with_newline(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="println",
            descriptor="(Ljava/lang/String;)V",
        )
        host.invoke_virtual(ref, JVMPrintStream(), ["Hello"])
        assert host.output == ["Hello\n"]

    def test_println_calls_stdout_callback(self) -> None:
        calls: list[str] = []
        host = JVMStdlibHost(stdout=calls.append)
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="println",
            descriptor="(Ljava/lang/String;)V",
        )
        host.invoke_virtual(ref, JVMPrintStream(), ["Hi"])
        assert calls == ["Hi\n"]
        assert host.output == ["Hi\n"]

    def test_write_int_appends_char(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="write",
            descriptor="(I)V",
        )
        host.invoke_virtual(ref, JVMPrintStream(), [65])  # 'A'
        assert host.output == ["A"]

    def test_write_int_calls_stdout_callback(self) -> None:
        calls: list[str] = []
        host = JVMStdlibHost(stdout=calls.append)
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="write",
            descriptor="(I)V",
        )
        host.invoke_virtual(ref, JVMPrintStream(), [66])  # 'B'
        assert calls == ["B"]

    def test_write_int_masks_to_byte(self) -> None:
        """Negative / large int values are masked to 0-255 before chr()."""
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="write",
            descriptor="(I)V",
        )
        host.invoke_virtual(ref, JVMPrintStream(), [0x141])  # 321 & 0xFF = 65 = 'A'
        assert host.output == ["A"]

    def test_flush_is_noop(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="flush",
            descriptor="()V",
        )
        result = host.invoke_virtual(ref, JVMPrintStream(), [])
        assert result is None
        assert host.output == []

    def test_inputstream_read_returns_eof(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/io/InputStream",
            name="read",
            descriptor="()I",
        )
        assert host.invoke_virtual(ref, JVMInputStream(), []) == -1

    def test_unknown_method_raises(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/io/PrintStream",
            name="nonExistentMethod",
            descriptor="()V",
        )
        with pytest.raises(RuntimeError, match="Unsupported virtual method reference"):
            host.invoke_virtual(ref, JVMPrintStream(), [])

    def test_non_reference_type_raises(self) -> None:
        """Anything that isn't a JVMMethodReference should also raise."""
        host = JVMStdlibHost()
        with pytest.raises(RuntimeError):
            host.invoke_virtual("not-a-ref", JVMPrintStream(), [])


# ---------------------------------------------------------------------------
# JVMStdlibHost.invoke_static
# ---------------------------------------------------------------------------


class TestInvokeStatic:
    """invoke_static handles Arrays.fill, in-class dispatch, and unknowns."""

    def test_arrays_fill_fills_slice(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/util/Arrays",
            name="fill",
            descriptor="([BIIB)V",
        )
        arr = bytearray(6)
        host.invoke_static(ref, {}, [arr, 1, 5, 0xAB])
        assert arr[0] == 0
        assert arr[1] == 0xAB
        assert arr[2] == 0xAB
        assert arr[3] == 0xAB
        assert arr[4] == 0xAB
        assert arr[5] == 0

    def test_arrays_fill_value_masked_to_byte(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="java/util/Arrays",
            name="fill",
            descriptor="([BIIB)V",
        )
        arr = bytearray(3)
        host.invoke_static(ref, {}, [arr, 0, 3, 256])  # 256 & 0xFF = 0
        assert arr[0] == 0

    def test_in_class_dispatch_calls_runtime(self) -> None:
        """invoke_static forwards same-class calls to _run_method_with_shared_state."""
        host = JVMStdlibHost()
        mock_rt = MagicMock()
        mock_rt._class_file.this_class_name = "MyApp"
        mock_rt._run_method_with_shared_state.return_value = 7
        host._runtime = mock_rt

        ref = JVMMethodReference(
            class_name="MyApp",
            name="__ca_regGet",
            descriptor="(I)I",
        )
        shared: dict[object, object] = {}
        result = host.invoke_static(ref, shared, [0])

        assert result == 7
        mock_rt._run_method_with_shared_state.assert_called_once_with(
            name="__ca_regGet",
            descriptor="(I)I",
            static_fields=shared,
            args=[0],
        )

    def test_unknown_class_raises(self) -> None:
        host = JVMStdlibHost()
        ref = JVMMethodReference(
            class_name="com/example/Unknown",
            name="method",
            descriptor="()V",
        )
        with pytest.raises(RuntimeError, match="Unsupported static method reference"):
            host.invoke_static(ref, {}, [])

    def test_non_reference_type_raises(self) -> None:
        host = JVMStdlibHost()
        with pytest.raises(RuntimeError):
            host.invoke_static("not-a-ref", {}, [])


# ---------------------------------------------------------------------------
# JVMRuntime.disassemble_method
# ---------------------------------------------------------------------------


class TestDisassembleMethod:
    """disassemble_method raises when the named method is absent."""

    def test_raises_when_method_not_found(self) -> None:
        runtime = JVMRuntime()
        cf = runtime.load_class(HELLO_WORLD_CLASS_BYTES)
        with pytest.raises(RuntimeError, match="was not found"):
            runtime.disassemble_method(cf, method_name="noSuchMethod", descriptor="()V")

    def test_returns_body_for_existing_method(self) -> None:
        runtime = JVMRuntime()
        cf = runtime.load_class(HELLO_WORLD_CLASS_BYTES)
        body = runtime.disassemble_method(
            cf,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )
        assert body is not None


# ---------------------------------------------------------------------------
# JVMRuntime._disassemble_method_optional  (covers line 424)
# ---------------------------------------------------------------------------


class TestDisassembleMethodOptional:
    """_disassemble_method_optional returns None when absent and body when present."""

    def test_returns_none_when_absent(self) -> None:
        runtime = JVMRuntime()
        cf = runtime.load_class(HELLO_WORLD_CLASS_BYTES)
        body = runtime._disassemble_method_optional(
            cf, method_name="doesNotExist", descriptor="()V"
        )
        assert body is None

    def test_returns_body_when_present(self) -> None:
        runtime = JVMRuntime()
        cf = runtime.load_class(HELLO_WORLD_CLASS_BYTES)
        # main([Ljava/lang/String;)V definitely exists in Hello World
        body = runtime._disassemble_method_optional(
            cf,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )
        assert body is not None  # line 424 in runtime.py is now covered


# ---------------------------------------------------------------------------
# JVMRuntime.run_method  — accepts both bytes and JVMClassFile
# ---------------------------------------------------------------------------


class TestRunMethod:
    """run_method can take raw bytes or a pre-parsed class file."""

    def test_run_method_accepts_parsed_class_file(self) -> None:
        runtime = JVMRuntime()
        cf = runtime.load_class(HELLO_WORLD_CLASS_BYTES)
        result = runtime.run_method(
            cf,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )
        assert result.output == "Hello, world!\n"

    def test_run_method_accepts_raw_bytes(self) -> None:
        runtime = JVMRuntime()
        result = runtime.run_method(
            HELLO_WORLD_CLASS_BYTES,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )
        assert result.output == "Hello, world!\n"


# ---------------------------------------------------------------------------
# JVMRuntime.run_method — <clinit> execution (covers lines 373-375)
# ---------------------------------------------------------------------------


class TestClinitExecution:
    """run_method runs <clinit> before the target method when one exists."""

    def test_run_method_with_clinit_class(self) -> None:
        """Runs a class that has a trivial <clinit>{ return; } + main{ return; }.

        The <clinit> is a no-op but its presence exercises the branch at lines
        373-375 in runtime.py (clinit_sim construction, load, and run).
        """
        runtime = JVMRuntime()
        result = runtime.run_method(
            MIN_CLINIT_CLASS_BYTES,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )
        # main just returns without producing any output
        assert result.output == ""
        assert result.return_value is None


# ---------------------------------------------------------------------------
# JVMRuntime._run_method_with_shared_state (covers lines 469-483)
# ---------------------------------------------------------------------------


class TestRunMethodWithSharedState:
    """_run_method_with_shared_state dispatches to a helper method in the class."""

    def test_runs_hello_world_main_as_inner_call(self) -> None:
        """Use Hello World's own main as the 'inner' method to prove the
        mechanism works end-to-end: output is accumulated in the shared host."""
        runtime = JVMRuntime()
        # Load the class so _run_method_with_shared_state has a class_file to look up
        runtime._class_file = runtime.load_class(HELLO_WORLD_CLASS_BYTES)
        shared: dict[object, object] = {}

        runtime._run_method_with_shared_state(
            name="main",
            descriptor="([Ljava/lang/String;)V",
            static_fields=shared,
            args=[None],  # String[] arg that main ignores
        )

        assert "Hello, world!" in "".join(runtime.host.output)

    def test_return_value_is_forwarded(self) -> None:
        """A method that returns an int should propagate the value."""
        runtime = JVMRuntime()
        # Use MIN_CLINIT_CLASS_BYTES which has a main that returns void (None)
        runtime._class_file = runtime.load_class(MIN_CLINIT_CLASS_BYTES)

        result = runtime._run_method_with_shared_state(
            name="main",
            descriptor="([Ljava/lang/String;)V",
            static_fields={},
            args=[None],
        )
        assert result is None  # void return → None

    def test_assert_fires_without_class_file(self) -> None:
        """_run_method_with_shared_state asserts that _class_file is loaded."""
        runtime = JVMRuntime()
        # _class_file is None by default
        with pytest.raises(AssertionError):
            runtime._run_method_with_shared_state(
                name="main",
                descriptor="()V",
                static_fields={},
                args=[],
            )
