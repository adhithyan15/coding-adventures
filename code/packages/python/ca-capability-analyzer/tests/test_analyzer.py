"""Tests for the capability analyzer AST walker.

These tests verify that the analyzer correctly detects OS capability
usage in Python source code. Each test provides a small Python snippet
and verifies that the analyzer identifies the correct capabilities.
"""

import ast
import textwrap

import pytest

from ca_capability_analyzer.analyzer import (
    CapabilityAnalyzer,
    DetectedCapability,
    analyze_file,
)


def _analyze(source: str) -> list[DetectedCapability]:
    """Helper: parse source and run the analyzer."""
    source = textwrap.dedent(source)
    tree = ast.parse(source, filename="<test>")
    analyzer = CapabilityAnalyzer("<test>")
    analyzer.visit(tree)
    return analyzer.detected


# ── Import detection ──────────────────────────────────────────────────


class TestImportDetection:
    """Tests for detecting capabilities from import statements."""

    def test_import_os(self) -> None:
        caps = _analyze("import os")
        assert len(caps) == 1
        assert caps[0].category == "fs"
        assert caps[0].action == "*"

    def test_import_socket(self) -> None:
        caps = _analyze("import socket")
        assert len(caps) == 1
        assert caps[0].category == "net"

    def test_import_subprocess(self) -> None:
        caps = _analyze("import subprocess")
        assert len(caps) == 1
        assert caps[0].category == "proc"
        assert caps[0].action == "exec"

    def test_import_ctypes(self) -> None:
        """ctypes is both an import capability AND a banned construct.
        The analyzer should detect it as an FFI capability."""
        caps = _analyze("import ctypes")
        assert len(caps) == 1
        assert caps[0].category == "ffi"

    def test_import_pathlib(self) -> None:
        caps = _analyze("import pathlib")
        assert len(caps) == 1
        assert caps[0].category == "fs"

    def test_import_shutil(self) -> None:
        caps = _analyze("import shutil")
        assert len(caps) == 1
        assert caps[0].category == "fs"

    def test_import_http_client(self) -> None:
        caps = _analyze("import http.client")
        assert len(caps) == 1
        assert caps[0].category == "net"
        assert caps[0].action == "connect"

    def test_import_urllib_request(self) -> None:
        caps = _analyze("import urllib.request")
        assert len(caps) == 1
        assert caps[0].category == "net"
        assert caps[0].action == "connect"

    def test_import_signal(self) -> None:
        caps = _analyze("import signal")
        assert len(caps) == 1
        assert caps[0].category == "proc"
        assert caps[0].action == "signal"

    def test_import_multiprocessing(self) -> None:
        caps = _analyze("import multiprocessing")
        assert len(caps) == 1
        assert caps[0].category == "proc"
        assert caps[0].action == "fork"

    def test_from_import(self) -> None:
        caps = _analyze("from os import listdir")
        assert len(caps) == 1
        assert caps[0].category == "fs"

    def test_from_socket_import(self) -> None:
        caps = _analyze("from socket import socket")
        assert len(caps) == 1
        assert caps[0].category == "net"

    def test_from_subprocess_import(self) -> None:
        caps = _analyze("from subprocess import run")
        assert len(caps) == 1
        assert caps[0].category == "proc"

    def test_import_aliased(self) -> None:
        caps = _analyze("import os as operating_system")
        assert len(caps) == 1
        assert caps[0].category == "fs"

    def test_no_capability_import(self) -> None:
        """Importing pure modules should not trigger detection."""
        caps = _analyze("""\
            import json
            import math
            import re
            import collections
            import typing
        """)
        assert len(caps) == 0

    def test_import_tempfile(self) -> None:
        caps = _analyze("import tempfile")
        assert len(caps) == 1
        assert caps[0].category == "fs"
        assert caps[0].action == "write"

    def test_import_glob(self) -> None:
        caps = _analyze("import glob")
        assert len(caps) == 1
        assert caps[0].category == "fs"
        assert caps[0].action == "list"


# ── open() call detection ─────────────────────────────────────────────


class TestOpenDetection:
    """Tests for detecting filesystem capabilities from open() calls."""

    def test_open_read_literal(self) -> None:
        caps = _analyze('open("data.txt")')
        assert len(caps) == 1
        assert caps[0].category == "fs"
        assert caps[0].action == "read"
        assert caps[0].target == "data.txt"

    def test_open_write_literal(self) -> None:
        caps = _analyze('open("output.txt", "w")')
        assert len(caps) == 1
        assert caps[0].action == "write"
        assert caps[0].target == "output.txt"

    def test_open_append(self) -> None:
        caps = _analyze('open("log.txt", "a")')
        assert len(caps) == 1
        assert caps[0].action == "write"

    def test_open_exclusive_create(self) -> None:
        caps = _analyze('open("new.txt", "x")')
        assert len(caps) == 1
        assert caps[0].action == "write"

    def test_open_read_explicit(self) -> None:
        caps = _analyze('open("data.txt", "r")')
        assert len(caps) == 1
        assert caps[0].action == "read"

    def test_open_with_mode_kwarg(self) -> None:
        caps = _analyze('open("data.txt", mode="w")')
        assert len(caps) == 1
        assert caps[0].action == "write"

    def test_open_variable_path(self) -> None:
        """When the path is a variable, target should be '*'."""
        caps = _analyze("""\
            path = get_path()
            open(path)
        """)
        assert len(caps) == 1
        assert caps[0].target == "*"

    def test_open_in_with_statement(self) -> None:
        caps = _analyze("""\
            with open("config.json") as f:
                data = f.read()
        """)
        assert len(caps) == 1
        assert caps[0].target == "config.json"

    def test_open_read_binary(self) -> None:
        caps = _analyze('open("image.png", "rb")')
        assert len(caps) == 1
        assert caps[0].action == "read"


# ── Attribute call detection ──────────────────────────────────────────


class TestAttributeCallDetection:
    """Tests for detecting capabilities from module.function() calls."""

    def test_os_listdir(self) -> None:
        caps = _analyze("""\
            import os
            os.listdir(".")
        """)
        # import os → fs:*:*, os.listdir → fs:list:*
        assert len(caps) == 2
        list_caps = [c for c in caps if c.action == "list"]
        assert len(list_caps) == 1

    def test_os_system(self) -> None:
        caps = _analyze("""\
            import os
            os.system("ls")
        """)
        proc_caps = [c for c in caps if c.category == "proc"]
        assert len(proc_caps) == 1
        assert proc_caps[0].action == "exec"

    def test_subprocess_run(self) -> None:
        caps = _analyze("""\
            import subprocess
            subprocess.run(["ls", "-la"])
        """)
        proc_caps = [c for c in caps if c.action == "exec"]
        # import subprocess gives proc:exec:*, subprocess.run also gives proc:exec:*
        assert len(proc_caps) >= 1

    def test_os_getenv(self) -> None:
        caps = _analyze("""\
            import os
            os.getenv("HOME")
        """)
        env_caps = [c for c in caps if c.category == "env"]
        assert len(env_caps) == 1

    def test_shutil_copy(self) -> None:
        caps = _analyze("""\
            import shutil
            shutil.copy("src.txt", "dst.txt")
        """)
        write_caps = [c for c in caps if c.action == "write"]
        assert len(write_caps) == 1

    def test_shutil_rmtree(self) -> None:
        caps = _analyze("""\
            import shutil
            shutil.rmtree("/tmp/old")
        """)
        delete_caps = [c for c in caps if c.action == "delete"]
        assert len(delete_caps) == 1

    def test_os_makedirs(self) -> None:
        caps = _analyze("""\
            import os
            os.makedirs("/tmp/new")
        """)
        create_caps = [c for c in caps if c.action == "create"]
        assert len(create_caps) == 1

    def test_os_remove(self) -> None:
        caps = _analyze("""\
            import os
            os.remove("old.txt")
        """)
        delete_caps = [c for c in caps if c.action == "delete"]
        assert len(delete_caps) == 1

    def test_os_environ_subscript(self) -> None:
        caps = _analyze("""\
            import os
            val = os.environ["PATH"]
        """)
        env_caps = [c for c in caps if c.category == "env"]
        assert len(env_caps) == 1
        assert env_caps[0].target == "PATH"

    def test_os_environ_subscript_variable(self) -> None:
        caps = _analyze("""\
            import os
            key = "PATH"
            val = os.environ[key]
        """)
        env_caps = [c for c in caps if c.category == "env"]
        assert len(env_caps) == 1
        assert env_caps[0].target == "*"

    def test_os_fork(self) -> None:
        caps = _analyze("""\
            import os
            os.fork()
        """)
        fork_caps = [c for c in caps if c.action == "fork"]
        assert len(fork_caps) == 1

    def test_os_kill(self) -> None:
        caps = _analyze("""\
            import os
            os.kill(1234, 9)
        """)
        signal_caps = [c for c in caps if c.action == "signal"]
        assert len(signal_caps) == 1


# ── From-import direct call detection ─────────────────────────────────


class TestFromImportDirectCall:
    """Tests for detecting calls to directly imported functions."""

    def test_from_os_import_listdir(self) -> None:
        caps = _analyze("""\
            from os import listdir
            listdir(".")
        """)
        list_caps = [c for c in caps if c.action == "list"]
        assert len(list_caps) == 1

    def test_from_subprocess_import_run(self) -> None:
        caps = _analyze("""\
            from subprocess import run
            run(["ls"])
        """)
        exec_caps = [c for c in caps if c.action == "exec"]
        assert len(exec_caps) >= 1


# ── Pure code detection (no capabilities) ─────────────────────────────


class TestPureCode:
    """Tests verifying that pure computation code triggers no detection."""

    def test_math_operations(self) -> None:
        caps = _analyze("""\
            x = 1 + 2
            y = x * 3
            z = max(x, y)
        """)
        assert len(caps) == 0

    def test_string_operations(self) -> None:
        caps = _analyze("""\
            s = "hello"
            t = s.upper()
            u = f"{s} world"
        """)
        assert len(caps) == 0

    def test_data_structures(self) -> None:
        caps = _analyze("""\
            items = [1, 2, 3]
            mapping = {"a": 1, "b": 2}
            unique = {1, 2, 3}
        """)
        assert len(caps) == 0

    def test_class_definition(self) -> None:
        caps = _analyze("""\
            class Gate:
                def __init__(self, name):
                    self.name = name
                def evaluate(self, a, b):
                    return a & b
        """)
        assert len(caps) == 0

    def test_safe_imports(self) -> None:
        caps = _analyze("""\
            import json
            import ast
            import math
            import re
            import collections
            import functools
            import itertools
            import typing
            from dataclasses import dataclass
            from enum import Enum
        """)
        assert len(caps) == 0

    def test_json_operations(self) -> None:
        caps = _analyze("""\
            import json
            data = json.loads('{"key": "value"}')
            text = json.dumps(data)
        """)
        assert len(caps) == 0


# ── Line number accuracy ─────────────────────────────────────────────


class TestLineNumbers:
    """Tests verifying that line numbers are accurate."""

    def test_line_number_import(self) -> None:
        caps = _analyze("""\
            x = 1
            y = 2
            import socket
        """)
        assert caps[0].line == 3

    def test_line_number_open(self) -> None:
        caps = _analyze("""\
            x = 1
            y = 2
            z = 3
            open("file.txt")
        """)
        assert caps[0].line == 4


# ── Evidence strings ──────────────────────────────────────────────────


class TestEvidence:
    """Tests verifying that evidence strings are informative."""

    def test_import_evidence(self) -> None:
        caps = _analyze("import socket")
        assert "import socket" in caps[0].evidence

    def test_open_evidence_with_path(self) -> None:
        caps = _analyze('open("data.txt")')
        assert "data.txt" in caps[0].evidence

    def test_attribute_call_evidence(self) -> None:
        caps = _analyze("""\
            import os
            os.listdir(".")
        """)
        list_caps = [c for c in caps if c.action == "list"]
        assert "os.listdir" in list_caps[0].evidence


# ── File analysis ─────────────────────────────────────────────────────


class TestFileAnalysis:
    """Tests for analyzing actual files on disk."""

    def test_analyze_file(self, tmp_path: object) -> None:
        """Test analyzing a temporary file."""
        from pathlib import Path

        tmp = Path(str(tmp_path))
        test_file = tmp / "test_module.py"
        test_file.write_text('import socket\nopen("data.txt")\n')
        caps = analyze_file(test_file)
        assert len(caps) == 2

    def test_analyze_file_pure(self, tmp_path: object) -> None:
        """Test that a pure file produces zero capabilities."""
        from pathlib import Path

        tmp = Path(str(tmp_path))
        test_file = tmp / "pure.py"
        test_file.write_text("x = 1 + 2\n")
        caps = analyze_file(test_file)
        assert len(caps) == 0

    def test_analyze_file_syntax_error(self, tmp_path: object) -> None:
        """Test that a file with syntax errors raises SyntaxError."""
        from pathlib import Path

        tmp = Path(str(tmp_path))
        test_file = tmp / "bad.py"
        test_file.write_text("def f(\n")
        with pytest.raises(SyntaxError):
            analyze_file(test_file)


# ── DetectedCapability dataclass ──────────────────────────────────────


class TestDetectedCapability:
    """Tests for the DetectedCapability dataclass."""

    def test_str_representation(self) -> None:
        cap = DetectedCapability(
            category="fs",
            action="read",
            target="data.txt",
            file="test.py",
            line=1,
            evidence="open('data.txt')",
        )
        assert str(cap) == "fs:read:data.txt"

    def test_as_dict(self) -> None:
        cap = DetectedCapability(
            category="net",
            action="connect",
            target="*",
            file="test.py",
            line=5,
            evidence="import socket",
        )
        d = cap.as_dict()
        assert d["category"] == "net"
        assert d["action"] == "connect"
        assert d["target"] == "*"
        assert d["file"] == "test.py"
        assert d["line"] == 5

    def test_frozen(self) -> None:
        """DetectedCapability should be immutable."""
        cap = DetectedCapability(
            category="fs",
            action="read",
            target="*",
            file="test.py",
            line=1,
            evidence="import os",
        )
        with pytest.raises(AttributeError):
            cap.category = "net"  # type: ignore[misc]
