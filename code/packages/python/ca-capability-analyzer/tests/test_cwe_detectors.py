"""Tests for static CWE detectors (CWE-1333, CWE-674, CWE-78, CWE-94, CWE-22)."""

from coding_adventures_ca_capability_analyzer.scanner import scan_source


def test_cwe_1333_regex_detection() -> None:
    src = """
import re

def parse(text):
    return re.findall(r"\\d+", text)
"""
    res = scan_source(src)
    cwe_1333 = [f for f in res.findings if f.cwe == "CWE-1333"]
    assert len(cwe_1333) >= 1
    assert any("re" in f.message for f in cwe_1333)


def test_cwe_674_direct_recursion() -> None:
    src = """
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

class Processor:
    def process(self, items):
        if not items:
            return 0
        return 1 + self.process(items[1:])
"""
    res = scan_source(src)
    cwe_674 = [f for f in res.findings if f.cwe == "CWE-674"]
    assert len(cwe_674) == 2
    messages = [f.message for f in cwe_674]
    assert any("factorial" in m for m in messages)
    assert any("process" in m for m in messages)


def test_cwe_674_non_recursive_ignored() -> None:
    src = """
def helper(x):
    return x + 1

def worker(x):
    return helper(x)
"""
    res = scan_source(src)
    cwe_674 = [f for f in res.findings if f.cwe == "CWE-674"]
    assert len(cwe_674) == 0


def test_cwe_78_command_injection_sinks() -> None:
    src = """
import os
import subprocess

def run_user_cmd(cmd):
    os.system(cmd)
    subprocess.call(cmd, shell=True)
"""
    res = scan_source(src)
    cwe_78 = [f for f in res.findings if f.cwe == "CWE-78"]
    assert len(cwe_78) >= 2
    assert all(f.severity == "error" for f in cwe_78)


def test_cwe_94_dynamic_eval_sinks() -> None:
    src = """
import pickle

def eval_code(user_input):
    eval(user_input)
    exec(user_input)
    pickle.loads(user_input)
"""
    res = scan_source(src)
    cwe_94 = [f for f in res.findings if f.cwe == "CWE-94"]
    assert len(cwe_94) >= 3
    assert all(f.severity == "error" for f in cwe_94)


def test_cwe_22_path_traversal() -> None:
    src = """
def read_secrets():
    # Explicit directory traversal
    open("../../../etc/passwd", "r")
    open("..\\\\windows\\\\system32", "r")

def dynamic_open(user_path):
    # Dynamic unvalidated path
    open("/var/log/" + user_path, "r")
"""
    res = scan_source(src)
    cwe_22 = [f for f in res.findings if f.cwe == "CWE-22"]
    assert len(cwe_22) >= 3
    # Literal '..' should be errors
    literal_traversal = [f for f in cwe_22 if f.severity == "error"]
    assert len(literal_traversal) >= 2
    # Dynamic open should be warning
    dynamic_warnings = [f for f in cwe_22 if f.severity == "warning"]
    assert len(dynamic_warnings) >= 1
