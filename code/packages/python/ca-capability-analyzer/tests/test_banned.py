"""Tests for the banned construct detector.

These tests verify that the detector correctly flags dynamic execution
constructs that are banned outright — eval(), exec(), compile(),
__import__(), importlib.import_module(), pickle.loads(), etc.

The philosophy: if an attacker can't use dynamic execution, they must
use direct imports, which the capability analyzer catches trivially.
Banning dynamic execution closes the obfuscation escape hatch.
"""

import ast
import textwrap

import pytest

from ca_capability_analyzer.banned import (
    BannedConstructDetector,
    BannedConstructViolation,
    detect_banned_constructs,
)


def _detect(source: str) -> list[BannedConstructViolation]:
    """Helper: parse source and run the banned construct detector."""
    source = textwrap.dedent(source)
    tree = ast.parse(source, filename="<test>")
    detector = BannedConstructDetector("<test>")
    detector.visit(tree)
    return detector.violations


# ── Banned builtin calls ─────────────────────────────────────────────


class TestBannedBuiltins:
    """Tests for detecting banned builtin function calls."""

    def test_eval(self) -> None:
        violations = _detect('eval("1 + 2")')
        assert len(violations) == 1
        assert violations[0].construct == "eval"

    def test_exec(self) -> None:
        violations = _detect('exec("x = 1")')
        assert len(violations) == 1
        assert violations[0].construct == "exec"

    def test_compile(self) -> None:
        violations = _detect('compile("x = 1", "<string>", "exec")')
        assert len(violations) == 1
        assert violations[0].construct == "compile"

    def test_dunder_import(self) -> None:
        violations = _detect('__import__("os")')
        assert len(violations) == 1
        assert violations[0].construct == "__import__"

    def test_globals(self) -> None:
        violations = _detect("globals()")
        assert len(violations) == 1
        assert violations[0].construct == "globals"

    def test_locals(self) -> None:
        violations = _detect("locals()")
        assert len(violations) == 1
        assert violations[0].construct == "locals"

    def test_getattr_dynamic(self) -> None:
        """getattr with a non-literal second argument is banned."""
        violations = _detect("""\
            attr = "some_method"
            getattr(obj, attr)
        """)
        assert len(violations) == 1
        assert violations[0].construct == "getattr"

    def test_getattr_literal_is_ok(self) -> None:
        """getattr with a string literal second argument is allowed."""
        violations = _detect('getattr(obj, "method_name")')
        assert len(violations) == 0

    def test_multiple_banned_builtins(self) -> None:
        violations = _detect("""\
            eval("1")
            exec("2")
            compile("3", "", "exec")
        """)
        assert len(violations) == 3
        constructs = {v.construct for v in violations}
        assert constructs == {"eval", "exec", "compile"}


# ── Banned module calls ──────────────────────────────────────────────


class TestBannedModuleCalls:
    """Tests for detecting banned module-level function calls."""

    def test_importlib_import_module(self) -> None:
        violations = _detect('importlib.import_module("os")')
        assert len(violations) == 1
        assert violations[0].construct == "importlib.import_module"

    def test_pickle_loads(self) -> None:
        violations = _detect("pickle.loads(data)")
        assert len(violations) == 1
        assert violations[0].construct == "pickle.loads"

    def test_pickle_load(self) -> None:
        violations = _detect("pickle.load(f)")
        assert len(violations) == 1
        assert violations[0].construct == "pickle.load"

    def test_marshal_loads(self) -> None:
        violations = _detect("marshal.loads(data)")
        assert len(violations) == 1
        assert violations[0].construct == "marshal.loads"

    def test_marshal_load(self) -> None:
        violations = _detect("marshal.load(f)")
        assert len(violations) == 1
        assert violations[0].construct == "marshal.load"


# ── Banned imports ───────────────────────────────────────────────────


class TestBannedImports:
    """Tests for detecting banned module imports."""

    def test_import_ctypes(self) -> None:
        violations = _detect("import ctypes")
        assert len(violations) == 1
        assert violations[0].construct == "import ctypes"

    def test_import_cffi(self) -> None:
        violations = _detect("import cffi")
        assert len(violations) == 1
        assert violations[0].construct == "import cffi"

    def test_from_ctypes_import(self) -> None:
        violations = _detect("from ctypes import cdll")
        assert len(violations) == 1
        assert violations[0].construct == "import ctypes"

    def test_from_cffi_import(self) -> None:
        violations = _detect("from cffi import FFI")
        assert len(violations) == 1
        assert violations[0].construct == "import cffi"

    def test_from_importlib_import_import_module(self) -> None:
        violations = _detect("from importlib import import_module")
        assert len(violations) == 1
        assert violations[0].construct == "importlib.import_module"

    def test_from_pickle_import_loads(self) -> None:
        violations = _detect("from pickle import loads")
        assert len(violations) == 1
        assert violations[0].construct == "pickle.loads"

    def test_from_marshal_import_load(self) -> None:
        violations = _detect("from marshal import load")
        assert len(violations) == 1
        assert violations[0].construct == "marshal.load"


# ── Safe code (no violations) ────────────────────────────────────────


class TestSafeCode:
    """Tests verifying that normal code triggers no violations."""

    def test_normal_function_calls(self) -> None:
        violations = _detect("""\
            x = len([1, 2, 3])
            y = str(42)
            z = int("10")
        """)
        assert len(violations) == 0

    def test_safe_imports(self) -> None:
        violations = _detect("""\
            import json
            import math
            import os
            import pathlib
        """)
        assert len(violations) == 0

    def test_getattr_with_literal(self) -> None:
        violations = _detect('x = getattr(obj, "name")')
        assert len(violations) == 0

    def test_open_is_not_banned(self) -> None:
        """open() is a capability, not a banned construct."""
        violations = _detect('open("file.txt")')
        assert len(violations) == 0

    def test_ast_parse_is_not_banned(self) -> None:
        """ast.parse is not banned (it's used by the analyzer itself)."""
        violations = _detect('import ast; ast.parse("x = 1")')
        assert len(violations) == 0


# ── Line numbers and evidence ────────────────────────────────────────


class TestLineNumbers:
    """Tests verifying line numbers and evidence strings."""

    def test_line_number(self) -> None:
        violations = _detect("""\
            x = 1
            y = 2
            eval("x + y")
        """)
        assert violations[0].line == 3

    def test_evidence_string(self) -> None:
        violations = _detect('eval("hello")')
        assert "eval" in violations[0].evidence

    def test_str_representation(self) -> None:
        violations = _detect('eval("1")')
        s = str(violations[0])
        assert "BANNED" in s
        assert "eval" in s


# ── File analysis ────────────────────────────────────────────────────


class TestFileAnalysis:
    """Tests for analyzing actual files on disk."""

    def test_detect_banned_in_file(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        test_file = tmp / "evil.py"
        test_file.write_text('eval("1 + 2")\nexec("x = 1")\n')
        violations = detect_banned_constructs(test_file)
        assert len(violations) == 2

    def test_clean_file(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        test_file = tmp / "clean.py"
        test_file.write_text("x = 1 + 2\n")
        violations = detect_banned_constructs(test_file)
        assert len(violations) == 0

    def test_syntax_error_file(self, tmp_path: object) -> None:
        from pathlib import Path

        tmp = Path(str(tmp_path))
        test_file = tmp / "bad.py"
        test_file.write_text("def f(\n")
        with pytest.raises(SyntaxError):
            detect_banned_constructs(test_file)


# ── BannedConstructViolation dataclass ───────────────────────────────


class TestViolationDataclass:
    """Tests for the BannedConstructViolation dataclass."""

    def test_frozen(self) -> None:
        v = BannedConstructViolation(
            construct="eval",
            file="test.py",
            line=1,
            evidence="eval(...)",
        )
        with pytest.raises(AttributeError):
            v.construct = "exec"  # type: ignore[misc]
