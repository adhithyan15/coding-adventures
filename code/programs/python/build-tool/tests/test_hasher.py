"""Tests for the hasher module."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path

import pytest

import build_tool.hasher as hasher_module
from build_tool.discovery import Package, discover_packages
from build_tool.hasher import (
    GENERATED_DIRECTORY_COMPONENTS,
    _collect_source_files,
    _hash_file,
    _update_file_frame,
    combine_hashes,
    hash_deps,
    hash_package,
)
from build_tool.resolver import DirectedGraph, resolve_dependencies

FIXTURES = Path(__file__).parent / "fixtures"
REPO_ROOT = Path(__file__).resolve().parents[5]
SOURCE_COLLECTION_CASES = (
    REPO_ROOT
    / "code"
    / "specs"
    / "fixtures"
    / "build-tool-v1"
    / "cases"
    / "source-collection-extension.json",
    REPO_ROOT
    / "code"
    / "specs"
    / "fixtures"
    / "build-tool-v1"
    / "cases"
    / "source-collection-declared.json",
)
HASHING_CACHE_MISSING_CASE = (
    REPO_ROOT
    / "code"
    / "specs"
    / "fixtures"
    / "build-tool-v1"
    / "cases"
    / "hashing-cache-missing.json"
)
HASHING_CACHE_CASES = tuple(
    REPO_ROOT
    / "code"
    / "specs"
    / "fixtures"
    / "build-tool-v1"
    / "cases"
    / f"hashing-cache-{state}.json"
    for state in ("missing", "hit", "corrupt")
)


def _fixture_generated_components(case_path: Path) -> frozenset[str]:
    case = json.loads(case_path.read_text(encoding="utf-8"))
    return frozenset(
        candidate["path"].split("/")[1]
        for candidate in case["input"]["options"]["candidates"]
        if candidate["path"].startswith("excluded-")
    )


FIXTURE_GENERATED_COMPONENTS = tuple(
    _fixture_generated_components(case_path) for case_path in SOURCE_COLLECTION_CASES
)


def _expected_framed_package_hash(
    repository_root: Path, repository_relative_paths: tuple[str, ...]
) -> str:
    """Build the portable package-hash frame independently of production code."""
    package_hash = hashlib.sha256()
    for relative_path in sorted(repository_relative_paths):
        path_bytes = relative_path.encode("utf-8")
        content = (repository_root / relative_path).read_bytes()
        package_hash.update(len(path_bytes).to_bytes(8, "big"))
        package_hash.update(path_bytes)
        package_hash.update(len(content).to_bytes(8, "big"))
        package_hash.update(content)
    return package_hash.hexdigest()


def _expected_framed_dependency_hash(
    dependency_digests: dict[str, str],
) -> str:
    """Build the hashing-v1 dependency frame independently of production code."""
    dependency_hash = hashlib.sha256()
    for package_name in sorted(dependency_digests):
        name_bytes = package_name.encode("utf-8")
        digest_bytes = bytes.fromhex(dependency_digests[package_name])
        dependency_hash.update(len(name_bytes).to_bytes(8, "big"))
        dependency_hash.update(name_bytes)
        dependency_hash.update(len(digest_bytes).to_bytes(8, "big"))
        dependency_hash.update(digest_bytes)
    return dependency_hash.hexdigest()


def _expected_combined_hash(package_digest: str, dependencies_digest: str) -> str:
    """Combine raw package and dependency digest bytes independently."""
    return hashlib.sha256(
        bytes.fromhex(package_digest) + bytes.fromhex(dependencies_digest)
    ).hexdigest()


class TestCollectSourceFiles:
    """Tests for _collect_source_files."""

    def test_collects_python_files(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.py").write_text("print('hi')")
        (pkg_dir / "pyproject.toml").write_text("[project]")
        (pkg_dir / "README.md").write_text("# readme")  # should be excluded

        pkg = Package(
            name="python/test-pkg", path=pkg_dir, language="python"
        )
        files = _collect_source_files(pkg)
        names = [f.name for f in files]
        assert "BUILD" in names
        assert "main.py" in names
        assert "pyproject.toml" in names
        assert "README.md" not in names

    def test_collects_ruby_files(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "ruby" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.rb").write_text("puts 'hi'")
        (pkg_dir / "Gemfile").write_text("source 'rubygems'")
        (pkg_dir / "Rakefile").write_text("task :test")
        (pkg_dir / "test.gemspec").write_text("Gem::Specification.new")

        pkg = Package(
            name="ruby/test-pkg", path=pkg_dir, language="ruby"
        )
        files = _collect_source_files(pkg)
        names = [f.name for f in files]
        assert "BUILD" in names
        assert "main.rb" in names
        assert "Gemfile" in names
        assert "Rakefile" in names
        assert "test.gemspec" in names

    def test_collects_go_files(self, tmp_path):
        pkg_dir = tmp_path / "programs" / "go" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.go").write_text("package main")
        (pkg_dir / "go.mod").write_text("module test")
        (pkg_dir / "go.sum").write_text("hash")

        pkg = Package(
            name="go/test-pkg", path=pkg_dir, language="go"
        )
        files = _collect_source_files(pkg)
        names = [f.name for f in files]
        assert "BUILD" in names
        assert "main.go" in names
        assert "go.mod" in names
        assert "go.sum" in names

    @pytest.mark.parametrize(
        ("case_path", "is_starlark"),
        (
            (SOURCE_COLLECTION_CASES[0], False),
            (SOURCE_COLLECTION_CASES[1], True),
        ),
        ids=("extension", "declared-sources"),
    )
    def test_collects_neutral_ocaml_sources_and_metadata(
        self, tmp_path, case_path, is_starlark
    ):
        case = json.loads(case_path.read_text(encoding="utf-8"))
        options = case["input"]["options"]
        expected_paths = tuple(
            entry["path"] for entry in case["expected"]["result"]["files"]
        )
        pkg_dir = tmp_path / "packages" / "ocaml" / "test-pkg"

        # Materialize the fixture's expected portable inputs plus representative
        # non-source files. Generated and linked candidates are already covered
        # by the exact pruning tests below and remain inert fixture records here.
        for relative_path in (*expected_paths, "README.md"):
            source = pkg_dir / relative_path
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_bytes(b"source\n")

        pkg = Package(
            name="ocaml/test-pkg",
            path=pkg_dir,
            language="ocaml",
            is_starlark=is_starlark,
            declared_srcs=options["declared_srcs"],
        )
        relative_files = tuple(
            path.relative_to(pkg_dir).as_posix()
            for path in _collect_source_files(pkg)
        )

        assert relative_files == expected_paths

    def test_declared_ocaml_sources_always_include_opam_manifest(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "ocaml" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("ocaml_library(name='test')")
        (pkg_dir / "test-pkg.opam").write_text('opam-version: "2.0"')
        (pkg_dir / "main.ml").write_text("let answer = 42")
        nested_opam = pkg_dir / "nested" / "unrelated.opam"
        nested_opam.parent.mkdir()
        nested_opam.write_text('opam-version: "2.0"')

        pkg = Package(
            name="ocaml/test-pkg",
            path=pkg_dir,
            language="ocaml",
            is_starlark=True,
            declared_srcs=["**/*.ml"],
        )

        assert tuple(
            path.relative_to(pkg_dir).as_posix()
            for path in _collect_source_files(pkg)
        ) == ("BUILD", "main.ml", "test-pkg.opam")

    def test_sorted_lexicographically(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "z_file.py").write_text("z")
        (pkg_dir / "a_file.py").write_text("a")
        sub = pkg_dir / "sub"
        sub.mkdir()
        (sub / "middle.py").write_text("m")

        pkg = Package(name="python/test", path=pkg_dir, language="python")
        files = _collect_source_files(pkg)
        relative_names = [str(f.relative_to(pkg_dir)) for f in files]
        assert relative_names == sorted(relative_names)

    def test_empty_package(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "empty"
        pkg_dir.mkdir(parents=True)
        # No files at all
        pkg = Package(name="python/empty", path=pkg_dir, language="python")
        files = _collect_source_files(pkg)
        assert files == []

    def test_generated_registry_matches_both_neutral_fixture_modes(self):
        assert FIXTURE_GENERATED_COMPONENTS == (
            GENERATED_DIRECTORY_COMPONENTS,
            GENERATED_DIRECTORY_COMPONENTS,
        )

    @pytest.mark.parametrize(
        ("is_starlark", "declared_srcs"),
        ((False, []), (True, ["**/*.py"])),
        ids=("extension", "declared-sources"),
    )
    def test_prunes_exact_generated_directory_components(
        self, tmp_path, is_starlark, declared_srcs
    ):
        pkg_dir = tmp_path / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.py").write_text("print('source')")

        for index, component in enumerate(sorted(FIXTURE_GENERATED_COMPONENTS[0])):
            generated = pkg_dir / f"excluded-{index}" / component
            generated.mkdir(parents=True)
            (generated / "generated.py").write_text("print('generated')")

        retained_paths = (
            "retained-case/_Build/source.py",
            "retained-near/_build-example/source.py",
            "retained-cabal-case/Dist-newstyle/source.py",
            "retained-cabal-near/dist-newstyle-example/source.py",
        )
        for relative_path in retained_paths:
            source = pkg_dir / relative_path
            source.parent.mkdir(parents=True)
            source.write_text("print('retained')")

        pkg = Package(
            name="python/test-pkg",
            path=pkg_dir,
            language="python",
            is_starlark=is_starlark,
            declared_srcs=declared_srcs,
        )
        relative_files = {
            path.relative_to(pkg_dir).as_posix() for path in _collect_source_files(pkg)
        }

        assert relative_files == {"BUILD", "main.py", *retained_paths}

    @pytest.mark.parametrize(
        ("is_starlark", "declared_srcs"),
        ((False, []), (True, ["**/*.py"])),
        ids=("extension", "declared-sources"),
    )
    def test_prunes_directory_links_and_reparse_points(
        self, tmp_path, monkeypatch, is_starlark, declared_srcs
    ):
        pkg_dir = tmp_path / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        for directory in ("source", "linked-source", "reparse-source"):
            child = pkg_dir / directory / "main.py"
            child.parent.mkdir()
            child.write_text("print('source')")
        (pkg_dir / "linked.py").write_text("print('linked')")

        monkeypatch.setattr(
            os.path,
            "islink",
            lambda path: Path(path).name in {"linked-source", "linked.py"},
        )
        monkeypatch.setattr(
            os.path,
            "isjunction",
            lambda path: Path(path).name == "reparse-source",
            raising=False,
        )

        pkg = Package(
            name="python/test-pkg",
            path=pkg_dir,
            language="python",
            is_starlark=is_starlark,
            declared_srcs=declared_srcs,
        )
        relative_files = {
            path.relative_to(pkg_dir).as_posix() for path in _collect_source_files(pkg)
        }

        assert relative_files == {"BUILD", "source/main.py"}

    @pytest.mark.skipif(os.name == "nt", reason="POSIX symlink semantics")
    def test_prunes_real_posix_directory_and_file_symlinks(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.py").write_text("print('source')")
        external = tmp_path / "external"
        external.mkdir()
        external_source = external / "external.py"
        external_source.write_text("print('external')")
        try:
            (pkg_dir / "linked-directory").symlink_to(
                external, target_is_directory=True
            )
            (pkg_dir / "linked.py").symlink_to(external_source)
        except OSError as error:
            pytest.skip(f"symlink creation unavailable: {error}")

        pkg = Package(name="python/test-pkg", path=pkg_dir, language="python")
        relative_files = {
            path.relative_to(pkg_dir).as_posix() for path in _collect_source_files(pkg)
        }

        assert relative_files == {"BUILD", "main.py"}

    @pytest.mark.skipif(os.name != "nt", reason="Windows junction semantics")
    def test_prunes_real_windows_junction(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.py").write_text("print('source')")
        external = tmp_path / "external"
        external.mkdir()
        (external / "external.py").write_text("print('external')")
        junction = pkg_dir / "junction"
        created = subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(junction), str(external)],
            check=False,
            capture_output=True,
            text=True,
        )
        if created.returncode != 0:
            pytest.skip(f"junction creation unavailable: {created.stderr.strip()}")

        try:
            pkg = Package(name="python/test-pkg", path=pkg_dir, language="python")
            relative_files = {
                path.relative_to(pkg_dir).as_posix()
                for path in _collect_source_files(pkg)
            }
            assert relative_files == {"BUILD", "main.py"}
        finally:
            os.rmdir(junction)


class TestHashFile:
    """Tests for _hash_file."""

    def test_hash_known_content(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("hello world")
        expected = hashlib.sha256(b"hello world").hexdigest()
        assert _hash_file(f) == expected

    def test_hash_empty_file(self, tmp_path):
        f = tmp_path / "empty.txt"
        f.write_text("")
        expected = hashlib.sha256(b"").hexdigest()
        assert _hash_file(f) == expected

    def test_frame_rejects_path_swapped_after_open(self, tmp_path, monkeypatch):
        source = tmp_path / "source.py"
        replacement = tmp_path / "replacement.py"
        source.write_bytes(b"source")
        replacement.write_bytes(b"replacement")
        real_lstat = os.lstat

        def mismatched_lstat(path):
            if Path(path) == source:
                return real_lstat(replacement)
            return real_lstat(path)

        monkeypatch.setattr(os, "lstat", mismatched_lstat)

        with pytest.raises(
            OSError, match="source path changed or is not a regular file"
        ):
            _update_file_frame(
                hashlib.sha256(), "code/source.py", source, tmp_path
            )

    def test_frame_rejects_same_size_mutation_metadata(self, tmp_path, monkeypatch):
        source = tmp_path / "source.py"
        source.write_bytes(b"source")
        real_fstat = os.fstat
        calls = 0

        def changing_fstat(descriptor):
            nonlocal calls
            calls += 1
            source_stat = real_fstat(descriptor)
            if calls == 1:
                return source_stat
            fields = list(source_stat)
            fields[8] += 1
            return os.stat_result(fields)

        monkeypatch.setattr(os, "fstat", changing_fstat)

        with pytest.raises(OSError, match="source changed while hashing"):
            _update_file_frame(
                hashlib.sha256(), "code/source.py", source, tmp_path
            )

    @pytest.mark.skipif(os.name != "nt", reason="Windows junction semantics")
    def test_frame_rejects_ancestor_replaced_by_windows_junction(self, tmp_path):
        package_root = tmp_path / "package"
        nested = package_root / "nested"
        nested.mkdir(parents=True)
        collected_source = nested / "source.py"
        collected_source.write_bytes(b"inside")

        external = tmp_path / "external"
        external.mkdir()
        (external / "source.py").write_bytes(b"outside")
        collected_source.unlink()
        nested.rmdir()
        created = subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(nested), str(external)],
            check=False,
            capture_output=True,
            text=True,
        )
        if created.returncode != 0:
            pytest.skip(f"junction creation unavailable: {created.stderr.strip()}")

        try:
            with pytest.raises(
                OSError, match="source path contains a linked directory"
            ):
                _update_file_frame(
                    hashlib.sha256(),
                    "code/packages/python/test/source.py",
                    collected_source,
                    package_root,
                )
        finally:
            os.rmdir(nested)

    @pytest.mark.skipif(os.name != "nt", reason="Windows junction semantics")
    def test_frame_rejects_windows_junction_to_package_sibling(self, tmp_path):
        package_root = tmp_path / "package"
        nested = package_root / "nested"
        nested.mkdir(parents=True)
        collected_source = nested / "source.py"
        collected_source.write_bytes(b"inside")

        sibling = package_root / "sibling"
        sibling.mkdir()
        (sibling / "source.py").write_bytes(b"different")
        collected_source.unlink()
        nested.rmdir()
        created = subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(nested), str(sibling)],
            check=False,
            capture_output=True,
            text=True,
        )
        if created.returncode != 0:
            pytest.skip(f"junction creation unavailable: {created.stderr.strip()}")

        try:
            with pytest.raises(
                OSError, match="source path contains a linked directory"
            ):
                _update_file_frame(
                    hashlib.sha256(),
                    "code/packages/python/test/source.py",
                    collected_source,
                    package_root,
                )
        finally:
            os.rmdir(nested)

    @pytest.mark.skipif(os.name != "nt", reason="Windows junction semantics")
    def test_frame_rejects_package_root_replaced_by_windows_junction(
        self, tmp_path
    ):
        package_root = tmp_path / "package"
        package_root.mkdir()
        collected_source = package_root / "source.py"
        collected_source.write_bytes(b"inside")

        external = tmp_path / "external"
        external.mkdir()
        (external / "source.py").write_bytes(b"outside")
        collected_source.unlink()
        package_root.rmdir()
        created = subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(package_root), str(external)],
            check=False,
            capture_output=True,
            text=True,
        )
        if created.returncode != 0:
            pytest.skip(f"junction creation unavailable: {created.stderr.strip()}")

        try:
            with pytest.raises(
                OSError, match="source path contains a linked directory"
            ):
                _update_file_frame(
                    hashlib.sha256(),
                    "code/packages/python/test/source.py",
                    collected_source,
                    package_root,
                )
        finally:
            os.rmdir(package_root)

    @pytest.mark.skipif(os.name != "nt", reason="Windows reparse semantics")
    def test_frame_rejects_ancestor_mutated_after_directory_lock(
        self, tmp_path, monkeypatch
    ):
        import ctypes
        import struct
        from ctypes import wintypes

        package_root = tmp_path / "package"
        nested = package_root / "nested"
        nested.mkdir(parents=True)
        collected_source = nested / "source.py"
        sibling = package_root / "sibling"
        sibling.mkdir()
        (sibling / "source.py").write_bytes(b"different")

        real_lock = hasher_module._windows_lock_unlinked_directories

        def lock_then_mutate(directory):
            handles = real_lock(directory)
            kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
            create_file = kernel32.CreateFileW
            create_file.argtypes = (
                wintypes.LPCWSTR,
                wintypes.DWORD,
                wintypes.DWORD,
                wintypes.LPVOID,
                wintypes.DWORD,
                wintypes.DWORD,
                wintypes.HANDLE,
            )
            create_file.restype = wintypes.HANDLE
            device_io_control = kernel32.DeviceIoControl
            device_io_control.argtypes = (
                wintypes.HANDLE,
                wintypes.DWORD,
                wintypes.LPVOID,
                wintypes.DWORD,
                wintypes.LPVOID,
                wintypes.DWORD,
                ctypes.POINTER(wintypes.DWORD),
                wintypes.LPVOID,
            )
            device_io_control.restype = wintypes.BOOL

            write_handle = create_file(
                str(nested),
                0x40000000,
                0x00000001 | 0x00000002,
                None,
                3,
                0x02000000 | 0x00200000,
                None,
            )
            invalid_handle_value = ctypes.c_void_p(-1).value
            assert write_handle != invalid_handle_value
            try:
                substitute = ("\\??\\" + str(sibling)).encode("utf-16-le")
                print_name = str(sibling).encode("utf-16-le")
                paths = substitute + b"\0\0" + print_name + b"\0\0"
                payload = struct.pack(
                    "<IHHHHHH",
                    0xA0000003,
                    8 + len(paths),
                    0,
                    0,
                    len(substitute),
                    len(substitute) + 2,
                    len(print_name),
                ) + paths
                buffer = ctypes.create_string_buffer(payload)
                returned = wintypes.DWORD()
                assert device_io_control(
                    write_handle,
                    0x000900A4,
                    buffer,
                    len(payload),
                    None,
                    0,
                    ctypes.byref(returned),
                    None,
                )
            finally:
                kernel32.CloseHandle(write_handle)
            return handles

        monkeypatch.setattr(
            hasher_module, "_windows_lock_unlinked_directories", lock_then_mutate
        )
        try:
            with pytest.raises(
                OSError, match="opened source did not retain its lexical path"
            ):
                _update_file_frame(
                    hashlib.sha256(),
                    "code/packages/python/test/source.py",
                    collected_source,
                    package_root,
                )
        finally:
            os.rmdir(nested)

    @pytest.mark.skipif(os.name == "nt", reason="POSIX symlink semantics")
    def test_frame_rejects_ancestor_replaced_by_posix_symlink(self, tmp_path):
        package_root = tmp_path / "package"
        nested = package_root / "nested"
        nested.mkdir(parents=True)
        collected_source = nested / "source.py"
        collected_source.write_bytes(b"inside")

        external = tmp_path / "external"
        external.mkdir()
        (external / "source.py").write_bytes(b"outside")
        collected_source.unlink()
        nested.rmdir()
        nested.symlink_to(external, target_is_directory=True)

        with pytest.raises(OSError):
            _update_file_frame(
                hashlib.sha256(),
                "code/packages/python/test/source.py",
                collected_source,
                package_root,
            )

    @pytest.mark.skipif(os.name == "nt", reason="POSIX no-follow semantics")
    def test_frame_rejects_lexical_parent_escape(self, tmp_path):
        package_root = tmp_path / "package"
        package_root.mkdir()
        external = tmp_path / "external"
        external.mkdir()
        source = external / "source.py"
        source.write_bytes(b"outside")

        with pytest.raises(OSError, match="source path is outside its package"):
            _update_file_frame(
                hashlib.sha256(),
                "code/packages/python/test/source.py",
                package_root / ".." / "external" / "source.py",
                package_root,
            )

    @pytest.mark.skipif(os.name == "nt", reason="POSIX no-follow semantics")
    def test_frame_fails_closed_without_no_follow_support(
        self, tmp_path, monkeypatch
    ):
        source = tmp_path / "source.py"
        source.write_bytes(b"source")
        monkeypatch.delattr(os, "O_NOFOLLOW", raising=False)

        with pytest.raises(
            OSError, match="source no-follow support is unavailable"
        ):
            _update_file_frame(
                hashlib.sha256(),
                "code/packages/python/test/source.py",
                source,
                tmp_path,
            )


class TestHashPackage:
    """Tests for hash_package."""

    def test_deterministic(self):
        packages = discover_packages(FIXTURES / "simple")
        h1 = hash_package(packages[0])
        h2 = hash_package(packages[0])
        assert h1 == h2

    def test_changes_on_content_change(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        src = pkg_dir / "main.py"
        src.write_text("v1")

        pkg = Package(name="python/test", path=pkg_dir, language="python")
        h1 = hash_package(pkg)

        src.write_text("v2")
        h2 = hash_package(pkg)

        assert h1 != h2

    @pytest.mark.parametrize(
        ("is_starlark", "declared_srcs"),
        ((False, []), (True, ["**/*.py"])),
        ids=("extension", "declared-sources"),
    )
    def test_changes_when_same_content_moves_to_a_new_path(
        self, tmp_path, is_starlark, declared_srcs
    ):
        pkg_dir = tmp_path / "packages" / "python" / "test"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_bytes(b"echo build\n")
        source = pkg_dir / "source.py"
        source.write_bytes(b"same bytes\x00\xff")
        pkg = Package(
            name="python/test",
            path=pkg_dir,
            language="python",
            is_starlark=is_starlark,
            declared_srcs=declared_srcs,
        )
        original_hash = hash_package(pkg)

        moved = pkg_dir / "nested" / "renamed.py"
        moved.parent.mkdir()
        source.rename(moved)

        assert hash_package(pkg) != original_hash

    def test_matches_language_neutral_hashing_cache_oracle(self, tmp_path):
        case = json.loads(HASHING_CACHE_MISSING_CASE.read_text(encoding="utf-8"))
        include_paths = tuple(case["input"]["options"]["include_paths"])
        repository_root = tmp_path / "repository"
        for entry in case["workspace"]["files"]:
            source = repository_root / entry["path"]
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text(entry["content_utf8"], encoding="utf-8", newline="")

        pkg = Package(
            name=case["input"]["options"]["package"],
            path=repository_root / "code" / "packages" / "python" / "demo",
            language="python",
            is_starlark=True,
            declared_srcs=["src/data.bin"],
        )

        assert hash_package(pkg) == case["expected"]["result"]["package_digest"]
        assert hash_package(pkg) == _expected_framed_package_hash(
            repository_root, include_paths
        )

    def test_frames_normalized_utf8_paths_lengths_and_raw_content(self, tmp_path):
        repository_root = tmp_path / "repository"
        pkg_dir = repository_root / "code" / "packages" / "python" / "test"
        package_prefix = "code/packages/python/test"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_bytes(b"echo build\r\n")
        nested = pkg_dir / "nested"
        nested.mkdir()
        (nested / "alpha.py").write_bytes(b"alpha\x00\xff\r\n")
        (nested / "caf\N{LATIN SMALL LETTER E WITH ACUTE}.py").write_bytes(
            b"same-content"
        )
        pkg = Package(name="python/test", path=pkg_dir, language="python")

        expected = _expected_framed_package_hash(
            repository_root,
            (
                f"{package_prefix}/BUILD",
                f"{package_prefix}/nested/alpha.py",
                f"{package_prefix}/nested/caf\N{LATIN SMALL LETTER E WITH ACUTE}.py",
            ),
        )

        assert hash_package(pkg) == expected

    @pytest.mark.parametrize("package_basename", ("packages", "programs"))
    def test_repository_path_anchor_ignores_later_bucket_names(
        self, tmp_path, package_basename
    ):
        repository_root = tmp_path / "repository"
        pkg_dir = (
            repository_root
            / "code"
            / "packages"
            / "python"
            / package_basename
        )
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_bytes(b"echo build\n")
        pkg = Package(
            name=f"python/{package_basename}",
            path=pkg_dir,
            language="python",
        )

        assert hash_package(pkg) == _expected_framed_package_hash(
            repository_root,
            (f"code/packages/python/{package_basename}/BUILD",),
        )

    def test_empty_package_hash(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "empty"
        pkg_dir.mkdir(parents=True)
        pkg = Package(name="python/empty", path=pkg_dir, language="python")
        h = hash_package(pkg)
        assert h == hashlib.sha256(b"").hexdigest()


class TestHashDeps:
    """Tests for hash_deps."""

    def test_diamond_deps_hash(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        pkg_hashes = {p.name: hash_package(p) for p in packages}
        original_hashes = dict(pkg_hashes)
        original_nodes = set(graph.nodes())
        original_edges = set(graph.edges())

        # A depends on B, C, D transitively
        h_a = hash_deps("python/pkg-a", graph, pkg_hashes)
        # D has no deps
        h_d = hash_deps("python/pkg-d", graph, pkg_hashes)

        assert h_a == _expected_framed_dependency_hash(
            {
                name: pkg_hashes[name]
                for name in ("python/pkg-b", "python/pkg-c", "python/pkg-d")
            }
        )
        assert h_a != h_d
        # D should be hash of empty string (no deps)
        assert h_d == hashlib.sha256(b"").hexdigest()
        assert pkg_hashes == original_hashes
        assert set(graph.nodes()) == original_nodes
        assert set(graph.edges()) == original_edges

    def test_no_deps_returns_empty_hash(self):
        g = DirectedGraph()
        g.add_node("a")
        h = hash_deps("a", g, {"a": "abc123"})
        assert h == hashlib.sha256(b"").hexdigest()

    def test_missing_node_returns_empty_hash(self):
        g = DirectedGraph()
        h = hash_deps("nonexistent", g, {})
        assert h == hashlib.sha256(b"").hexdigest()

    def test_deterministic(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        pkg_hashes = {p.name: hash_package(p) for p in packages}

        h1 = hash_deps("python/pkg-a", graph, pkg_hashes)
        h2 = hash_deps("python/pkg-a", graph, pkg_hashes)
        assert h1 == h2

    @pytest.mark.parametrize("case_path", HASHING_CACHE_CASES)
    def test_matches_language_neutral_hashing_cache_oracle(self, case_path):
        case = json.loads(case_path.read_text(encoding="utf-8"))
        options = case["input"]["options"]
        expected = case["expected"]["result"]
        graph = DirectedGraph()
        graph.add_node(options["package"])
        dependency_digests = {
            entry["package"]: entry["digest"]
            for entry in options["dependency_digests"]
        }
        for dependency_name in reversed(tuple(dependency_digests)):
            graph.add_edge(dependency_name, options["package"])

        dependencies_digest = hash_deps(
            options["package"], graph, dependency_digests
        )

        assert dependencies_digest == expected["dependencies_digest"]
        assert dependencies_digest == _expected_framed_dependency_hash(
            dependency_digests
        )
        assert (
            combine_hashes(expected["package_digest"], dependencies_digest)
            == expected["combined_digest"]
        )
        assert expected["combined_digest"] == _expected_combined_hash(
            expected["package_digest"], dependencies_digest
        )

    def test_frames_sorted_package_names_and_raw_digest_bytes(self):
        graph = DirectedGraph()
        graph.add_edge("python/a", "python/app")
        graph.add_edge("python/bc", "python/app")
        digests = {
            "python/bc": "22" * 32,
            "python/a": "11" * 32,
        }
        original = dict(digests)

        actual = hash_deps("python/app", graph, digests)

        assert actual == _expected_framed_dependency_hash(digests)
        assert digests == original

    def test_package_identity_participates_in_dependency_hash(self):
        first = DirectedGraph()
        first.add_edge("python/a", "python/app")
        second = DirectedGraph()
        second.add_edge("python/b", "python/app")
        digest = "34" * 32

        assert hash_deps("python/app", first, {"python/a": digest}) != hash_deps(
            "python/app", second, {"python/b": digest}
        )

    def test_frames_prevent_dependency_boundary_ambiguity(self):
        first = DirectedGraph()
        first.add_edge("python/a", "python/app")
        first.add_edge("python/bc", "python/app")
        second = DirectedGraph()
        second.add_edge("python/ab", "python/app")
        second.add_edge("python/c", "python/app")
        first_digests = {"python/a": "11" * 32, "python/bc": "22" * 32}
        second_digests = {"python/ab": "11" * 32, "python/c": "22" * 32}

        assert hash_deps("python/app", first, first_digests) != hash_deps(
            "python/app", second, second_digests
        )

    @pytest.mark.parametrize(
        "digest",
        (
            "AB" * 32,
            "ab" * 31,
            "gg" * 32,
            "ab" * 32 + "00",
        ),
    )
    def test_rejects_invalid_dependency_digest_without_echoing_it(self, digest):
        graph = DirectedGraph()
        graph.add_edge("python/base", "python/app")

        with pytest.raises(
            ValueError, match="invalid SHA-256 dependency digest"
        ) as exc:
            hash_deps("python/app", graph, {"python/base": digest})

        assert digest not in str(exc.value)

    def test_rejects_missing_dependency_digest(self):
        graph = DirectedGraph()
        graph.add_edge("python/base", "python/app")

        with pytest.raises(ValueError, match="missing SHA-256 dependency digest"):
            hash_deps("python/app", graph, {})

    @pytest.mark.parametrize("invalid_role", ("package", "dependency"))
    def test_rejects_invalid_combined_digest_input(self, invalid_role):
        package_digest = "11" * 32
        dependencies_digest = "22" * 32
        invalid_digest = "not-a-digest"
        if invalid_role == "package":
            package_digest = invalid_digest
        else:
            dependencies_digest = invalid_digest

        with pytest.raises(ValueError, match=f"invalid SHA-256 {invalid_role} digest"):
            combine_hashes(package_digest, dependencies_digest)
