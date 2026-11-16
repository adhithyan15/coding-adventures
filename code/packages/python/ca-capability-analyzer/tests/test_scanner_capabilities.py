"""Tests for AST capability scanning across Python imports and function calls."""

import ast

import pytest

import coding_adventures_ca_capability_analyzer.scanner as sc
from coding_adventures_ca_capability_analyzer.scanner import (
    MAX_AST_NODES,
    scan_source,
)
from coding_adventures_ca_capability_analyzer.types import (
    CAP_DYNAMIC_EVAL,
    CAP_FFI_UNSAFE,
    CAP_FS_READ,
    CAP_FS_WRITE,
    CAP_NET_CLIENT,
    CAP_NET_SERVER,
    CAP_PROCESS_EXEC,
    CAP_REGEX_RE,
)


def test_scan_process_exec() -> None:
    src = """
import subprocess
import os

def run_cmd():
    subprocess.run(["ls", "-la"])
    os.system("echo hi")
    os.popen("date")
    os.spawnl(os.P_WAIT, "/bin/ls", "ls")
"""
    res = scan_source(src)
    assert CAP_PROCESS_EXEC in res.capabilities


def test_scan_process_exec_aliased() -> None:
    src = """
import subprocess as sp
from subprocess import Popen

def run_p():
    sp.call(["echo"])
    Popen(["cat"])
"""
    res = scan_source(src)
    assert CAP_PROCESS_EXEC in res.capabilities


def test_scan_net_capabilities() -> None:
    src = """
import socket
import urllib.request
import http.client

def do_net():
    s = socket.socket()
    s.connect(("127.0.0.1", 80))
    urllib.request.urlopen("https://example.com")
    http.client.HTTPConnection("localhost")
"""
    res = scan_source(src)
    assert CAP_NET_CLIENT in res.capabilities

    src_server = """
import socket

def do_server():
    s = socket.socket()
    s.bind(("0.0.0.0", 8080))
    s.listen(5)
"""
    res_server = scan_source(src_server)
    assert CAP_NET_SERVER in res_server.capabilities


def test_scan_ffi_capabilities() -> None:
    src = """
import ctypes
from ctypes import CDLL

libc = CDLL("libc.so.6")
"""
    res = scan_source(src)
    assert CAP_FFI_UNSAFE in res.capabilities


def test_scan_dynamic_eval() -> None:
    src = """
import pickle
import _pickle

def run_dynamic(code):
    eval(code)
    exec(code)
    compile(code, "fn", "exec")
    pickle.loads(b"")
    _pickle.load(None)
"""
    res = scan_source(src)
    assert CAP_DYNAMIC_EVAL in res.capabilities


def test_scan_regex_re() -> None:
    src = """
import re

pattern = re.compile(r"^test")
"""
    res = scan_source(src)
    assert CAP_REGEX_RE in res.capabilities


def test_scan_filesystem_read_and_write() -> None:
    src_read = """
def read_data():
    with open("data.txt", "r") as f:
        return f.read()
"""
    res_read = scan_source(src_read)
    assert CAP_FS_READ in res_read.capabilities
    assert CAP_FS_WRITE not in res_read.capabilities

    src_write = """
def write_data():
    with open("data.txt", "w") as f:
        f.write("hello")
"""
    res_write = scan_source(src_write)
    assert CAP_FS_WRITE in res_write.capabilities
    assert CAP_FS_READ not in res_write.capabilities

    src_rw = """
def rw_data():
    with open("data.txt", "w+") as f:
        pass
"""
    res_rw = scan_source(src_rw)
    assert CAP_FS_READ in res_rw.capabilities
    assert CAP_FS_WRITE in res_rw.capabilities


def test_scan_pathlib_operations() -> None:
    src = """
from pathlib import Path

def handle_path(p: Path):
    data = p.read_bytes()
    text = p.read_text()
    p.write_bytes(b"123")
    p.write_text("hello")
"""
    res = scan_source(src)
    assert CAP_FS_READ in res.capabilities
    assert CAP_FS_WRITE in res.capabilities


def test_scan_import_from_variations() -> None:
    src = """
from re import search
from ctypes import c_int
from subprocess import check_output
from socket import socket
from pickle import dumps
import socketserver

async def async_worker():
    s = socketserver.TCPServer()
    with open("f.txt", mode="r") as f1:
        pass
    with open("f2.txt", "w", encoding="utf-8") as f2:
        pass
    dynamic_mode = "r"
    with open("f3.txt", dynamic_mode) as f3:
        pass
    with open("f4.txt", mode=dynamic_mode) as f4:
        pass
"""
    res = scan_source(src)
    assert CAP_REGEX_RE in res.capabilities
    assert CAP_FFI_UNSAFE in res.capabilities
    assert CAP_PROCESS_EXEC in res.capabilities
    assert CAP_NET_CLIENT in res.capabilities
    assert CAP_NET_SERVER in res.capabilities
    assert CAP_DYNAMIC_EVAL in res.capabilities
    assert CAP_FS_READ in res.capabilities
    assert CAP_FS_WRITE in res.capabilities


def test_scan_node_limit_exceeded() -> None:
    # Build source with many nodes
    lines = ["x = 1\n"] * (MAX_AST_NODES + 10)
    src = "".join(lines)
    with pytest.raises(ValueError, match="exceeded MAX_AST_NODES"):
        scan_source(src)


def test_scan_source_cwe400_bounds_and_exceptions(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # MAX_FILE_BYTES exceeded
    monkeypatch.setattr(sc, "MAX_FILE_BYTES", 5)
    res_large = sc.scan_source("x = 1234567890")
    assert len(res_large.findings) == 1
    assert res_large.findings[0].cwe == "CWE-400"
    assert "exceeds MAX_FILE_BYTES" in res_large.findings[0].message

    monkeypatch.setattr(sc, "MAX_FILE_BYTES", 1000)

    # RecursionError
    def raise_recursion(*args: object, **kwargs: object) -> None:
        raise RecursionError("maximum recursion depth exceeded")

    monkeypatch.setattr(ast, "parse", raise_recursion)
    res_rec = sc.scan_source("x = 1")
    assert len(res_rec.findings) == 1
    assert res_rec.findings[0].cwe == "CWE-400"
    assert "RecursionError" in res_rec.findings[0].message

    # MemoryError
    def raise_mem(*args: object, **kwargs: object) -> None:
        raise MemoryError("out of memory")

    monkeypatch.setattr(ast, "parse", raise_mem)
    res_mem = sc.scan_source("x = 1")
    assert len(res_mem.findings) == 1
    assert res_mem.findings[0].cwe == "CWE-400"
    assert "MemoryError" in res_mem.findings[0].message
