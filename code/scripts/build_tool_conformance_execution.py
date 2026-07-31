#!/usr/bin/env python3
"""Validate trusted-execution policy without executing fixture code.

This module is the process-free policy layer. It deliberately imports no
process API, creates no workspace, and has no host-execution fallback. Platform
backends land separately. The Linux OCI identity schema is checked here, but
its process-owning capability preflight is never imported. Until later
authority and execution tranches land, ``run-case`` can only return a stable
non-passing result after authority checks succeed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import struct
import sys
import unicodedata
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import build_tool_conformance as bootstrap
import build_tool_conformance_authority as authority

DEFAULT_FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT
DEFAULT_EXECUTION_CASE_ROOT = DEFAULT_FIXTURE_ROOT / "execution-cases"
MAX_EXECUTION_CASE_BYTES = bootstrap.MAX_DOCUMENT_BYTES
MAX_EXECUTION_CORPUS_MEMBERS = 256
MAX_EXECUTION_CORPUS_TOTAL_BYTES = 16 * 1024 * 1024
MAX_EXECUTION_DIRECTORY_ENTRIES = 4096
SHA256_PATTERN = "0123456789abcdef"
PLATFORM_BACKEND_KINDS = {
    "darwin": "macos_isolated",
    "linux": "linux_oci",
    "windows": "windows_appcontainer",
}
_SNAPSHOT_FACTORY_TOKEN = object()


@dataclass(frozen=True, init=False)
class ExecutionCaseMember:
    """One direct corpus member retained as immutable exact bytes."""

    relative_path: str
    raw: bytes

    def __init__(
        self,
        *,
        relative_path: str,
        raw: bytes,
        _factory_token: object | None = None,
    ) -> None:
        if _factory_token is not _SNAPSHOT_FACTORY_TOKEN:
            raise TypeError(
                "execution case members are created only by snapshot_from_entries"
            )
        _validate_execution_case_name(
            relative_path,
            code="EXECUTION_CORPUS_PATH_UNSAFE",
            label="execution corpus path",
        )
        if not isinstance(raw, bytes):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_BYTES_INVALID",
                f"execution corpus member is not immutable bytes: {relative_path}",
            )
        object.__setattr__(self, "relative_path", relative_path)
        object.__setattr__(self, "raw", raw)


@dataclass(frozen=True, init=False)
class ExecutionCaseSelection:
    """One exact member selected from a particular held corpus snapshot."""

    relative_path: str
    corpus_sha256: str
    raw: bytes

    def __init__(
        self,
        *,
        relative_path: str,
        corpus_sha256: str,
        raw: bytes,
        _factory_token: object | None = None,
    ) -> None:
        if _factory_token is not _SNAPSHOT_FACTORY_TOKEN:
            raise TypeError(
                "execution case selections are created only by a held snapshot"
            )
        _validate_execution_case_name(
            relative_path,
            code="EXECUTION_CASE_SELECTOR_UNSAFE",
            label="execution case selector",
        )
        if not _is_sha256(corpus_sha256) or not isinstance(raw, bytes):
            raise TypeError("execution case selection identity is invalid")
        object.__setattr__(self, "relative_path", relative_path)
        object.__setattr__(self, "corpus_sha256", corpus_sha256)
        object.__setattr__(self, "raw", raw)


@dataclass(frozen=True, init=False)
class ExecutionCaseSnapshot:
    """One immutable, digest-bound execution-corpus snapshot."""

    corpus_sha256: str
    members: tuple[ExecutionCaseMember, ...]

    def __init__(
        self,
        *,
        members: tuple[ExecutionCaseMember, ...],
        _factory_token: object | None = None,
    ) -> None:
        if _factory_token is not _SNAPSHOT_FACTORY_TOKEN:
            raise TypeError(
                "execution case snapshots are created only by snapshot_from_entries"
            )
        if not isinstance(members, tuple) or any(
            type(member) is not ExecutionCaseMember for member in members
        ):
            raise TypeError("execution case snapshot members are invalid")
        object.__setattr__(self, "members", members)
        object.__setattr__(self, "corpus_sha256", _digest_members(members))

    def select(self, relative_path: str) -> ExecutionCaseSelection:
        """Select one canonical direct member without reopening its pathname."""

        try:
            _validate_execution_case_name(
                relative_path,
                code="EXECUTION_CASE_SELECTOR_UNSAFE",
                label="execution case selector",
            )
        except bootstrap.ConformanceError as error:
            if isinstance(relative_path, str):
                normalized = unicodedata.normalize("NFC", relative_path)
                if normalized != relative_path:
                    try:
                        _validate_execution_case_name(
                            normalized,
                            code="EXECUTION_CASE_SELECTOR_UNSAFE",
                            label="execution case selector",
                        )
                    except bootstrap.ConformanceError:
                        pass
                    else:
                        normalized_identity = _execution_case_name_identity(normalized)
                        if any(
                            _execution_case_name_identity(member.relative_path)
                            == normalized_identity
                            for member in self.members
                        ):
                            raise bootstrap.ConformanceError(
                                "EXECUTION_CASE_SELECTOR_ALIAS",
                                "execution case selector is a Unicode normalization "
                                "alias of a retained member",
                            ) from error
            raise
        requested_identity = _execution_case_name_identity(relative_path)
        for member in self.members:
            if member.relative_path == relative_path:
                return ExecutionCaseSelection(
                    relative_path=member.relative_path,
                    corpus_sha256=self.corpus_sha256,
                    raw=member.raw,
                    _factory_token=_SNAPSHOT_FACTORY_TOKEN,
                )
            if (
                _execution_case_name_identity(member.relative_path)
                == requested_identity
            ):
                raise bootstrap.ConformanceError(
                    "EXECUTION_CASE_SELECTOR_ALIAS",
                    "execution case selector is a case or Unicode alias of "
                    f"{member.relative_path}",
                )
        raise bootstrap.ConformanceError(
            "EXECUTION_CASE_NOT_FOUND",
            f"execution case is not present in the held snapshot: {relative_path}",
        )


def _execution_case_name_identity(relative_path: str) -> str:
    return unicodedata.normalize("NFC", relative_path).casefold()


def _validate_execution_case_name(
    relative_path: Any,
    *,
    code: str,
    label: str,
) -> None:
    error = bootstrap.portable_path_error(relative_path)
    if error is not None or "/" in relative_path or not relative_path.endswith(".json"):
        detail = error or "name must be one direct lowercase-.json member"
        raise bootstrap.ConformanceError(
            code,
            f"unsafe {label} {relative_path!r}: {detail}",
        )
    try:
        relative_path.encode("utf-8", errors="strict")
    except UnicodeEncodeError as error:
        raise bootstrap.ConformanceError(
            code,
            f"unsafe {label} {relative_path!r}: name is not strict UTF-8",
        ) from error


def _validated_corpus_entries(
    entries: Iterable[tuple[str, bytes]],
) -> tuple[ExecutionCaseMember, ...]:
    members: list[ExecutionCaseMember] = []
    identities: dict[str, str] = {}
    total_bytes = 0
    for relative_path, raw in entries:
        _validate_execution_case_name(
            relative_path,
            code="EXECUTION_CORPUS_PATH_UNSAFE",
            label="execution corpus path",
        )
        if not isinstance(raw, bytes):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_BYTES_INVALID",
                f"execution corpus member is not immutable bytes: {relative_path}",
            )
        if len(raw) > MAX_EXECUTION_CASE_BYTES:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_FILE_TOO_LARGE",
                f"execution corpus member exceeds its byte ceiling: {relative_path}",
            )
        identity = _execution_case_name_identity(relative_path)
        previous = identities.get(identity)
        if previous is not None:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_PATH_DUPLICATE",
                f"execution corpus path collides with {previous}: {relative_path}",
            )
        identities[identity] = relative_path
        if len(members) >= MAX_EXECUTION_CORPUS_MEMBERS:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_MEMBER_LIMIT_EXCEEDED",
                "execution corpus exceeds its runner-owned member ceiling",
            )
        total_bytes += len(raw)
        if total_bytes > MAX_EXECUTION_CORPUS_TOTAL_BYTES:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_AGGREGATE_TOO_LARGE",
                "execution corpus exceeds its runner-owned aggregate byte ceiling",
            )
        members.append(
            ExecutionCaseMember(
                relative_path=relative_path,
                raw=raw,
                _factory_token=_SNAPSHOT_FACTORY_TOKEN,
            )
        )
    return tuple(sorted(members, key=lambda member: member.relative_path))


def _digest_members(members: Sequence[ExecutionCaseMember]) -> str:
    digest = hashlib.sha256()
    for member in members:
        path_bytes = member.relative_path.encode("utf-8")
        digest.update(struct.pack(">Q", len(path_bytes)))
        digest.update(path_bytes)
        digest.update(struct.pack(">Q", len(member.raw)))
        digest.update(member.raw)
    return digest.hexdigest()


def snapshot_from_entries(
    entries: Iterable[tuple[str, bytes]],
) -> ExecutionCaseSnapshot:
    """Create a typed immutable snapshot from already-retained exact bytes."""

    members = _validated_corpus_entries(entries)
    return ExecutionCaseSnapshot(
        members=members,
        _factory_token=_SNAPSHOT_FACTORY_TOKEN,
    )


def _snapshot_file_identity(value: os.stat_result) -> tuple[int, ...]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _is_reparse(value: os.stat_result) -> bool:
    return bool(
        getattr(value, "st_file_attributes", 0)
        & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    )


def _read_raw_regular_bound(
    path: Path,
    *,
    max_bytes: int = MAX_EXECUTION_CASE_BYTES,
) -> tuple[bytes, os.stat_result]:
    """Read one stable, singly linked regular file without final-link follow."""

    try:
        with bootstrap._open_regular_no_follow(path) as source:
            before = os.fstat(source.fileno())
            if (
                not stat.S_ISREG(before.st_mode)
                or _is_reparse(before)
                or before.st_nlink != 1
            ):
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_FILE_INVALID",
                    "execution corpus member is not one regular, non-reparse, "
                    f"singly linked file: {path.name}",
                )
            raw = source.read(max_bytes + 1)
            after = os.fstat(source.fileno())
    except bootstrap.ConformanceError as error:
        if error.code.startswith("DOCUMENT_"):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_FILE_INVALID",
                f"execution corpus member could not be opened safely: {path.name}",
            ) from error
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_READ_FAILED",
            f"could not read execution corpus member: {path.name}",
        ) from error
    if len(raw) > max_bytes:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_FILE_TOO_LARGE",
            f"execution corpus member exceeds {max_bytes} bytes: {path.name}",
        )
    if _snapshot_file_identity(before) != _snapshot_file_identity(
        after
    ) or before.st_size != len(raw):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_FILE_CHANGED",
            f"execution corpus member changed while it was read: {path.name}",
        )
    return raw, before


def _read_raw_regular(
    path: Path,
    *,
    max_bytes: int = MAX_EXECUTION_CASE_BYTES,
) -> bytes:
    """Read exact bytes from one stable singly linked regular file."""

    raw, _ = _read_raw_regular_bound(path, max_bytes=max_bytes)
    return raw


def framed_corpus_digest(entries: Iterable[tuple[str, bytes]]) -> str:
    """Hash sorted portable paths and exact bytes with length framing."""

    return _digest_members(_validated_corpus_entries(entries))


def _validated_execution_case_names(names: Iterable[str]) -> tuple[str, ...]:
    candidates: list[str] = []
    for entry_count, name in enumerate(names, start=1):
        if entry_count > MAX_EXECUTION_DIRECTORY_ENTRIES:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_ENUMERATION_LIMIT_EXCEEDED",
                "execution corpus directory exceeds its runner-owned entry ceiling",
            )
        if name.casefold().endswith(".json"):
            candidates.append(name)
            if len(candidates) > MAX_EXECUTION_CORPUS_MEMBERS:
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_MEMBER_LIMIT_EXCEEDED",
                    "execution corpus exceeds its runner-owned member ceiling",
                )
    candidates.sort()
    identities: dict[str, str] = {}
    for name in candidates:
        _validate_execution_case_name(
            name,
            code="EXECUTION_CORPUS_PATH_UNSAFE",
            label="execution corpus path",
        )
        identity = _execution_case_name_identity(name)
        previous = identities.get(identity)
        if previous is not None:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_PATH_DUPLICATE",
                f"execution corpus path collides with {previous}: {name}",
            )
        identities[identity] = name
    return tuple(candidates)


def _scan_posix_execution_case_names(root_descriptor: int) -> tuple[str, ...]:
    try:
        with os.scandir(root_descriptor) as iterator:
            return _validated_execution_case_names(entry.name for entry in iterator)
    except bootstrap.ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_READ_FAILED",
            "execution corpus could not be enumerated from its retained root",
        ) from error


def _snapshot_directory_identity(value: os.stat_result) -> tuple[int, ...]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _lexical_absolute_case_root(case_root: Path) -> Path:
    if any(part in {".", ".."} for part in case_root.parts):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            "execution corpus root contains a dot segment",
        )
    if case_root.is_absolute():
        return case_root
    return Path.cwd() / case_root


def _open_posix_case_root(case_root: Path) -> int:
    if (
        os.name != "posix"
        or not hasattr(os, "O_NOFOLLOW")
        or os.open not in os.supports_dir_fd
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            "retained-root execution corpus capture is unavailable",
        )
    absolute = _lexical_absolute_case_root(case_root)
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_DIRECTORY", 0)
        | os.O_NOFOLLOW
    )
    descriptor: int | None = None
    try:
        descriptor = os.open("/", flags)
        for part in absolute.parts[1:]:
            child = os.open(part, flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        root_status = os.fstat(descriptor)
        if not stat.S_ISDIR(root_status.st_mode):
            raise OSError("execution corpus root is not a directory")
        return descriptor
    except FileNotFoundError as error:
        if descriptor is not None:
            os.close(descriptor)
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_MISSING",
            "execution corpus directory does not exist",
        ) from error
    except (OSError, ValueError) as error:
        if descriptor is not None:
            os.close(descriptor)
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            "execution corpus root could not be retained without following links",
        ) from error
    except BaseException:
        if descriptor is not None:
            os.close(descriptor)
        raise


def _read_posix_snapshot_member(
    root_descriptor: int,
    relative_path: str,
    *,
    max_bytes: int,
) -> tuple[bytes, tuple[int, int]]:
    descriptor: int | None = None
    try:
        descriptor = os.open(
            relative_path,
            os.O_RDONLY
            | getattr(os, "O_CLOEXEC", 0)
            | getattr(os, "O_NOFOLLOW", 0)
            | getattr(os, "O_NONBLOCK", 0),
            dir_fd=root_descriptor,
        )
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_FILE_INVALID",
                "execution corpus member must be one regular, singly linked file: "
                f"{relative_path}",
            )
        chunks: list[bytes] = []
        total = 0
        while True:
            chunk = os.read(
                descriptor,
                min(1_048_576, max_bytes + 1 - total),
            )
            if not chunk:
                break
            chunks.append(chunk)
            total += len(chunk)
            if total > max_bytes:
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_FILE_TOO_LARGE",
                    f"execution corpus member exceeds {max_bytes} bytes: "
                    f"{relative_path}",
                )
        after = os.fstat(descriptor)
        if (
            _snapshot_file_identity(before) != _snapshot_file_identity(after)
            or before.st_size != total
        ):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_FILE_CHANGED",
                f"execution corpus member changed while it was read: {relative_path}",
            )
        return b"".join(chunks), (before.st_dev, before.st_ino)
    except bootstrap.ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_FILE_INVALID",
            "execution corpus member could not be opened relative to the retained "
            f"root: {relative_path}",
        ) from error
    finally:
        if descriptor is not None:
            os.close(descriptor)


def _capture_posix_execution_entries(
    case_root: Path,
    *,
    max_bytes: int,
) -> list[tuple[str, bytes]]:
    root_descriptor = _open_posix_case_root(case_root)
    try:
        before = os.fstat(root_descriptor)
        first_names = _scan_posix_execution_case_names(root_descriptor)
        entries: list[tuple[str, bytes]] = []
        file_identities: set[tuple[int, int]] = set()
        total_bytes = 0
        for name in first_names:
            raw, identity = _read_posix_snapshot_member(
                root_descriptor,
                name,
                max_bytes=max_bytes,
            )
            if identity in file_identities:
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_FILE_INVALID",
                    f"execution corpus members alias one file identity: {name}",
                )
            file_identities.add(identity)
            total_bytes += len(raw)
            if total_bytes > MAX_EXECUTION_CORPUS_TOTAL_BYTES:
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_AGGREGATE_TOO_LARGE",
                    "execution corpus exceeds its runner-owned aggregate byte ceiling",
                )
            entries.append((name, raw))
        second_names = _scan_posix_execution_case_names(root_descriptor)
        after = os.fstat(root_descriptor)
        if first_names != second_names or _snapshot_directory_identity(
            before
        ) != _snapshot_directory_identity(after):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_CHANGED",
                "execution corpus membership changed during snapshot capture",
            )
        return entries
    except bootstrap.ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_READ_FAILED",
            "execution corpus could not be enumerated from its retained root",
        ) from error
    finally:
        os.close(root_descriptor)


def _capture_windows_execution_entries(
    case_root: Path,
    *,
    max_bytes: int,
) -> list[tuple[str, bytes]]:
    import ctypes
    from ctypes import wintypes

    absolute = _lexical_absolute_case_root(case_root)
    path_text = str(absolute)
    if not absolute.drive or path_text.startswith("\\\\"):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            "Windows execution corpus root must be one local drive path",
        )

    class FileAttributeTagInfo(ctypes.Structure):
        _fields_ = [
            ("file_attributes", wintypes.DWORD),
            ("reparse_tag", wintypes.DWORD),
        ]

    class FileBasicInfo(ctypes.Structure):
        _fields_ = [
            ("creation_time", ctypes.c_longlong),
            ("last_access_time", ctypes.c_longlong),
            ("last_write_time", ctypes.c_longlong),
            ("change_time", ctypes.c_longlong),
            ("file_attributes", wintypes.DWORD),
        ]

    class FileIdBothDirectoryInfo(ctypes.Structure):
        _fields_ = [
            ("next_entry_offset", wintypes.DWORD),
            ("file_index", wintypes.DWORD),
            ("creation_time", ctypes.c_longlong),
            ("last_access_time", ctypes.c_longlong),
            ("last_write_time", ctypes.c_longlong),
            ("change_time", ctypes.c_longlong),
            ("end_of_file", ctypes.c_longlong),
            ("allocation_size", ctypes.c_longlong),
            ("file_attributes", wintypes.DWORD),
            ("file_name_length", wintypes.DWORD),
            ("ea_size", wintypes.DWORD),
            ("short_name_length", ctypes.c_byte),
            ("short_name", wintypes.WCHAR * 12),
            ("file_id", ctypes.c_ulonglong),
        ]

    class ByHandleFileInformation(ctypes.Structure):
        _fields_ = [
            ("file_attributes", wintypes.DWORD),
            ("creation_time_low", wintypes.DWORD),
            ("creation_time_high", wintypes.DWORD),
            ("last_access_time_low", wintypes.DWORD),
            ("last_access_time_high", wintypes.DWORD),
            ("last_write_time_low", wintypes.DWORD),
            ("last_write_time_high", wintypes.DWORD),
            ("volume_serial_number", wintypes.DWORD),
            ("file_size_high", wintypes.DWORD),
            ("file_size_low", wintypes.DWORD),
            ("number_of_links", wintypes.DWORD),
            ("file_index_high", wintypes.DWORD),
            ("file_index_low", wintypes.DWORD),
        ]

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
    get_info = kernel32.GetFileInformationByHandleEx
    get_info.argtypes = (
        wintypes.HANDLE,
        ctypes.c_int,
        wintypes.LPVOID,
        wintypes.DWORD,
    )
    get_info.restype = wintypes.BOOL
    close_handle = kernel32.CloseHandle
    close_handle.argtypes = (wintypes.HANDLE,)
    close_handle.restype = wintypes.BOOL
    get_handle_info = kernel32.GetFileInformationByHandle
    get_handle_info.argtypes = (
        wintypes.HANDLE,
        ctypes.POINTER(ByHandleFileInformation),
    )
    get_handle_info.restype = wintypes.BOOL
    get_drive_type = kernel32.GetDriveTypeW
    get_drive_type.argtypes = (wintypes.LPCWSTR,)
    get_drive_type.restype = wintypes.UINT
    query_dos_device = kernel32.QueryDosDeviceW
    query_dos_device.argtypes = (
        wintypes.LPCWSTR,
        wintypes.LPWSTR,
        wintypes.DWORD,
    )
    query_dos_device.restype = wintypes.DWORD

    file_list_directory = 0x0001
    file_read_attributes = 0x0080
    file_share_read = 0x00000001
    file_share_write = 0x00000002
    open_existing = 3
    file_flag_backup_semantics = 0x02000000
    file_flag_open_reparse_point = 0x00200000
    file_attribute_directory = 0x00000010
    file_attribute_reparse_point = 0x00000400
    drive_fixed = 3
    invalid_handle = ctypes.c_void_p(-1).value

    drive_name = absolute.drive
    drive_root = f"{drive_name}\\"
    device_target_buffer = ctypes.create_unicode_buffer(1024)
    if (
        get_drive_type(drive_root) != drive_fixed
        or query_dos_device(
            drive_name,
            device_target_buffer,
            len(device_target_buffer),
        )
        == 0
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            "Windows execution corpus root must use one fixed local volume",
        )
    device_target = device_target_buffer.value
    harddisk_volume_prefix = "\\Device\\HarddiskVolume"
    if (
        not device_target.startswith(harddisk_volume_prefix)
        or not device_target[len(harddisk_volume_prefix) :].isdecimal()
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            "Windows execution corpus root uses a remappable device namespace",
        )

    def open_directory(path: Path, *, final: bool) -> int:
        handle = create_file(
            str(path),
            file_read_attributes | (file_list_directory if final else 0),
            file_share_read if final else file_share_read | file_share_write,
            None,
            open_existing,
            file_flag_backup_semantics | file_flag_open_reparse_point,
            None,
        )
        if handle == invalid_handle:
            error_code = ctypes.get_last_error()
            code = (
                "EXECUTION_CORPUS_DIRECTORY_MISSING"
                if error_code in {2, 3}
                else "EXECUTION_CORPUS_DIRECTORY_INVALID"
            )
            raise bootstrap.ConformanceError(
                code,
                "execution corpus directory could not be retained",
            )
        info = FileAttributeTagInfo()
        if not get_info(handle, 9, ctypes.byref(info), ctypes.sizeof(info)):
            error_code = ctypes.get_last_error()
            close_handle(handle)
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
                f"execution corpus directory attributes are unavailable: {error_code}",
            )
        if (
            not info.file_attributes & file_attribute_directory
            or info.file_attributes & file_attribute_reparse_point
        ):
            close_handle(handle)
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
                "execution corpus path is linked, reparse, or non-directory",
            )
        return handle

    def basic_identity(handle: int) -> tuple[int, ...]:
        info = FileBasicInfo()
        if not get_info(handle, 0, ctypes.byref(info), ctypes.sizeof(info)):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
                "execution corpus directory identity is unavailable",
            )
        return (
            info.creation_time,
            info.last_write_time,
            info.change_time,
            info.file_attributes,
        )

    def volume_serial(handle: int) -> int:
        info = ByHandleFileInformation()
        if not get_handle_info(handle, ctypes.byref(info)):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
                "execution corpus root volume identity is unavailable",
            )
        return int(info.volume_serial_number)

    def enumerate_directory(handle: int) -> tuple[tuple[Any, ...], ...]:
        records: list[tuple[Any, ...]] = []
        entry_count = 0
        while True:
            buffer = ctypes.create_string_buffer(65_536)
            if not get_info(handle, 10, buffer, len(buffer)):
                error_code = ctypes.get_last_error()
                if error_code == 18:
                    break
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_READ_FAILED",
                    f"execution corpus handle enumeration failed: {error_code}",
                )
            offset = 0
            while True:
                record = FileIdBothDirectoryInfo.from_buffer_copy(buffer, offset)
                name = ctypes.wstring_at(
                    ctypes.addressof(buffer)
                    + offset
                    + ctypes.sizeof(FileIdBothDirectoryInfo),
                    record.file_name_length // 2,
                )
                if name not in {".", ".."}:
                    entry_count += 1
                    if entry_count > MAX_EXECUTION_DIRECTORY_ENTRIES:
                        raise bootstrap.ConformanceError(
                            "EXECUTION_CORPUS_ENUMERATION_LIMIT_EXCEEDED",
                            "execution corpus directory exceeds its runner-owned "
                            "entry ceiling",
                        )
                    records.append(
                        (
                            name,
                            int(record.file_id),
                            int(record.file_attributes),
                            int(record.end_of_file),
                            int(record.last_write_time),
                            int(record.change_time),
                        )
                    )
                if record.next_entry_offset == 0:
                    break
                offset += record.next_entry_offset
        return tuple(records)

    chain_paths = [Path(absolute.anchor)]
    current = Path(absolute.anchor)
    for part in absolute.parts[1:]:
        current /= part
        chain_paths.append(current)

    handles: list[int] = []
    try:
        for index, path in enumerate(chain_paths):
            handles.append(open_directory(path, final=index == len(chain_paths) - 1))
        root_handle = handles[-1]
        before = basic_identity(root_handle)
        root_volume_serial = volume_serial(root_handle)
        first_records = enumerate_directory(root_handle)
        record_by_name = {str(record[0]): record for record in first_records}
        names = _validated_execution_case_names(record_by_name)
        entries: list[tuple[str, bytes]] = []
        identities: set[tuple[int, int]] = set()
        total_bytes = 0
        for name in names:
            record = record_by_name[name]
            attributes = int(record[2])
            if (
                attributes & file_attribute_directory
                or attributes & file_attribute_reparse_point
            ):
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_FILE_INVALID",
                    f"execution corpus member is linked or non-regular: {name}",
                )
            member_path = absolute / name
            raw, status = _read_raw_regular_bound(
                member_path,
                max_bytes=max_bytes,
            )
            identity = (status.st_dev, status.st_ino)
            if (
                status.st_dev != root_volume_serial
                or status.st_ino != int(record[1])
                or identity in identities
            ):
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_FILE_INVALID",
                    f"execution corpus member identity is aliased or changed: {name}",
                )
            identities.add(identity)
            total_bytes += len(raw)
            if total_bytes > MAX_EXECUTION_CORPUS_TOTAL_BYTES:
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_AGGREGATE_TOO_LARGE",
                    "execution corpus exceeds its runner-owned aggregate byte ceiling",
                )
            entries.append((name, raw))

        second_handle = open_directory(absolute, final=True)
        try:
            second_records = enumerate_directory(second_handle)
        finally:
            close_handle(second_handle)
        after = basic_identity(root_handle)
        first_cases = tuple(record_by_name[name] for name in names)
        second_by_name = {str(record[0]): record for record in second_records}
        second_cases = tuple(second_by_name.get(name) for name in names)
        second_names = _validated_execution_case_names(second_by_name)
        if before != after or names != second_names or first_cases != second_cases:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_CHANGED",
                "execution corpus membership changed during snapshot capture",
            )
        return entries
    finally:
        for handle in reversed(handles):
            close_handle(handle)


def capture_execution_case_snapshot(
    case_root: Path,
    *,
    max_bytes: int = MAX_EXECUTION_CASE_BYTES,
) -> ExecutionCaseSnapshot:
    """Capture one retained-root, exact-byte, immutable corpus snapshot."""

    if (
        not isinstance(max_bytes, int)
        or isinstance(max_bytes, bool)
        or not 0 <= max_bytes <= MAX_EXECUTION_CASE_BYTES
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_LIMIT_INVALID",
            "execution corpus member byte ceiling must be a non-negative integer",
        )
    entries = (
        _capture_windows_execution_entries(case_root, max_bytes=max_bytes)
        if os.name == "nt"
        else _capture_posix_execution_entries(case_root, max_bytes=max_bytes)
    )
    return snapshot_from_entries(entries)


def _execution_corpus_entries(case_root: Path) -> list[tuple[str, bytes]]:
    """Compatibility view over one immutable corpus snapshot."""

    snapshot = capture_execution_case_snapshot(case_root)
    return [(member.relative_path, member.raw) for member in snapshot.members]


def execution_corpus_digest(case_root: Path) -> str:
    """Compute the reviewed execution-corpus digest without JSON decoding."""

    return capture_execution_case_snapshot(case_root).corpus_sha256


def _is_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in SHA256_PATTERN for character in value)
    )


def validate_policy_semantics(policy: dict[str, Any]) -> dict[str, int]:
    """Validate cross-field identities that JSON Schema cannot express."""

    backends = policy["backends"]
    platforms = [item["platform"] for item in backends]
    if platforms != sorted(PLATFORM_BACKEND_KINDS):
        raise bootstrap.ConformanceError(
            "EXECUTION_BACKENDS_NOT_CANONICAL",
            "backends must contain darwin, linux, and windows in sorted order",
        )
    for backend in backends:
        expected_kind = PLATFORM_BACKEND_KINDS[backend["platform"]]
        if backend["kind"] != expected_kind:
            raise bootstrap.ConformanceError(
                "EXECUTION_BACKEND_KIND_MISMATCH",
                f"{backend['platform']} requires backend kind {expected_kind}",
            )

    adapter_keys: set[tuple[str, str]] = set()
    for adapter in policy["adapters"]:
        key = (adapter["language"], adapter["platform"])
        if key in adapter_keys:
            raise bootstrap.ConformanceError(
                "EXECUTION_ADAPTER_DUPLICATE",
                f"duplicate execution adapter identity: {key[0]}/{key[1]}",
            )
        adapter_keys.add(key)
        if error := bootstrap.portable_path_error(adapter["executable"]):
            raise bootstrap.ConformanceError(
                "EXECUTION_ADAPTER_PATH_UNSAFE",
                f"unsafe adapter path {adapter['executable']!r}: {error}",
            )
    return {
        "ready_backend_count": sum(item["status"] == "ready" for item in backends),
        "adapter_count": len(policy["adapters"]),
    }


def _execution_options(case: dict[str, Any]) -> dict[str, Any]:
    input_value = case.get("input")
    if not isinstance(input_value, dict):
        return {}
    options = input_value.get("options")
    return options if isinstance(options, dict) else {}


def _validate_execution_graph(
    package_names: set[str],
    edges: list[list[str]],
) -> None:
    outgoing = {name: [] for name in package_names}
    indegree = {name: 0 for name in package_names}
    for prerequisite, dependent in edges:
        if prerequisite not in package_names or dependent not in package_names:
            raise bootstrap.ConformanceError(
                "EXECUTION_EDGE_UNKNOWN",
                "execution dependency edge references an unknown package",
            )
        outgoing[prerequisite].append(dependent)
        indegree[dependent] += 1
    ready = sorted(name for name, count in indegree.items() if count == 0)
    visited = 0
    while ready:
        current = ready.pop(0)
        visited += 1
        for dependent in sorted(outgoing[current]):
            indegree[dependent] -= 1
            if indegree[dependent] == 0:
                ready.append(dependent)
                ready.sort()
    if visited != len(package_names):
        raise bootstrap.ConformanceError(
            "EXECUTION_GRAPH_CYCLE",
            "execution dependency graph contains a cycle",
        )


def _validate_package_result_state(package_result: dict[str, Any]) -> None:
    """Enforce the fail-stop state machine independently of JSON Schema.

    Schema validation closes each local record before this function runs, but
    the semantic validator repeats the security-relevant scalar checks and adds
    sequence constraints that JSON Schema cannot express. In particular, a
    failed command divides one package's command list into a succeeded prefix
    and a not-run suffix.
    """

    package_name = package_result["name"]
    package_status = package_result["status"]
    return_code = package_result["return_code"]
    commands = package_result["commands"]
    command_statuses: list[str] = []

    for command in commands:
        command_status = command["status"]
        exit_code = command["exit_code"]
        command_statuses.append(command_status)
        valid_exit = (
            (
                command_status == "succeeded"
                and exit_code == 0
                and not isinstance(exit_code, bool)
            )
            or (
                command_status == "failed"
                and isinstance(exit_code, int)
                and not isinstance(exit_code, bool)
                and exit_code != 0
            )
            or (command_status == "not-run" and exit_code is None)
        )
        if not valid_exit:
            raise bootstrap.ConformanceError(
                "EXECUTION_COMMAND_EXIT_CODE_MISMATCH",
                f"command status and exit code disagree for {package_name}",
            )

    if package_status == "built":
        valid_package = (
            return_code == 0
            and not isinstance(return_code, bool)
            and all(status == "succeeded" for status in command_statuses)
        )
    elif package_status == "failed":
        failed_indices = [
            index for index, status in enumerate(command_statuses) if status == "failed"
        ]
        valid_package = (
            isinstance(return_code, int)
            and not isinstance(return_code, bool)
            and return_code != 0
            and len(failed_indices) == 1
        )
        if not valid_package:
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_STATE_MISMATCH",
                f"failed package state is incomplete for {package_name}",
            )
        failed_index = failed_indices[0]
        if not (
            all(status == "succeeded" for status in command_statuses[:failed_index])
            and all(
                status == "not-run" for status in command_statuses[failed_index + 1 :]
            )
        ):
            raise bootstrap.ConformanceError(
                "EXECUTION_COMMAND_STATE_ORDER_INVALID",
                f"commands do not stop at the first failure for {package_name}",
            )
        if return_code != commands[failed_index]["exit_code"]:
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_RETURN_CODE_MISMATCH",
                f"package return code does not equal its failed command for {package_name}",
            )
        return
    elif package_status in {"dep-skipped", "would-build"}:
        valid_package = return_code is None and all(
            status == "not-run" for status in command_statuses
        )
    else:
        valid_package = False

    if not valid_package:
        raise bootstrap.ConformanceError(
            "EXECUTION_PACKAGE_STATE_MISMATCH",
            f"package status, return code, and commands disagree for {package_name}",
        )


def _validate_execution_outcome_state(
    *,
    dry_run: bool,
    outcome: str,
    result_packages: list[dict[str, Any]],
) -> None:
    """Tie the case mode and overall outcome to every package state."""

    statuses = [package["status"] for package in result_packages]
    if dry_run:
        valid = outcome == "ok" and all(status == "would-build" for status in statuses)
    elif outcome == "ok":
        valid = all(status == "built" for status in statuses)
    elif outcome == "error":
        valid = "failed" in statuses and all(
            status in {"built", "failed", "dep-skipped"} for status in statuses
        )
    else:
        valid = False
    if not valid:
        raise bootstrap.ConformanceError(
            "EXECUTION_OUTCOME_STATE_MISMATCH",
            "dry-run mode, overall outcome, and package statuses disagree",
        )


def _validate_dependency_result_states(
    *,
    package_names: set[str],
    edges: list[list[str]],
    result_by_name: dict[str, dict[str, Any]],
) -> None:
    """Require dependency skips exactly where failed prerequisites demand them."""

    prerequisites = {name: [] for name in package_names}
    for prerequisite, dependent in edges:
        prerequisites[dependent].append(prerequisite)

    for package_name in sorted(package_names):
        blocked = any(
            result_by_name[prerequisite]["status"] in {"failed", "dep-skipped"}
            for prerequisite in prerequisites[package_name]
        )
        skipped = result_by_name[package_name]["status"] == "dep-skipped"
        if blocked != skipped:
            raise bootstrap.ConformanceError(
                "EXECUTION_DEPENDENCY_STATE_MISMATCH",
                f"dependency status does not justify the result for {package_name}",
            )


def validate_execution_semantics(case: dict[str, Any]) -> None:
    """Validate execution case identities and deterministic result ordering."""

    if case.get("domain") != "execution":
        raise bootstrap.ConformanceError(
            "EXECUTION_DOMAIN_INVALID",
            "execution cases require domain=execution",
        )
    input_value = case.get("input")
    if not isinstance(input_value, dict) or input_value.get("operation") != "execution":
        raise bootstrap.ConformanceError(
            "EXECUTION_OPERATION_INVALID",
            "execution cases require input.operation=execution",
        )
    capabilities = case.get("capabilities")
    if not isinstance(capabilities, list) or not {
        "execution",
        "trusted_execution",
    }.issubset(capabilities):
        raise bootstrap.ConformanceError(
            "EXECUTION_CAPABILITY_MISSING",
            "execution cases require execution and trusted_execution",
        )
    expected = case.get("expected")
    if (
        not isinstance(expected, dict)
        or expected.get("case_id") != case.get("id")
        or expected.get("domain") != "execution"
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_IDENTITY_MISMATCH",
            "execution case and expected result identities must match",
        )

    options = _execution_options(case)
    platform_name = options.get("platform")
    platforms = case.get("platforms")
    if not isinstance(platforms, list) or platform_name not in platforms:
        raise bootstrap.ConformanceError(
            "EXECUTION_PLATFORM_MISMATCH",
            "execution options.platform must be listed in top-level platforms",
        )
    limits = case.get("limits")
    process_limit = limits.get("process_count") if isinstance(limits, dict) else None
    jobs = options.get("jobs")
    if (
        not isinstance(jobs, int)
        or not isinstance(process_limit, int)
        or jobs > process_limit
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_JOB_LIMIT",
            "execution jobs cannot exceed the requested process_count limit",
        )

    packages = options.get("packages")
    if not isinstance(packages, list):
        raise bootstrap.ConformanceError(
            "EXECUTION_PACKAGES_INVALID",
            "execution packages must be an array",
        )
    package_names: set[str] = set()
    normalized_paths: set[str] = set()
    for package in packages:
        name = package["name"]
        if name in package_names:
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_DUPLICATE",
                f"duplicate execution package: {name}",
            )
        package_names.add(name)
        rel_path = package["rel_path"]
        if error := bootstrap.portable_path_error(rel_path):
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_PATH_UNSAFE",
                f"unsafe execution package path {rel_path!r}: {error}",
            )
        normalized = unicodedata.normalize("NFC", rel_path).casefold()
        if normalized in normalized_paths:
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_PATH_DUPLICATE",
                f"duplicate normalized execution package path: {rel_path}",
            )
        normalized_paths.add(normalized)
        if package["resource_locks"] != sorted(package["resource_locks"]):
            raise bootstrap.ConformanceError(
                "EXECUTION_LOCKS_NOT_CANONICAL",
                f"resource locks are not sorted for {name}",
            )

    edges = options.get("dependency_edges")
    if not isinstance(edges, list):
        raise bootstrap.ConformanceError(
            "EXECUTION_EDGES_INVALID",
            "execution dependency_edges must be an array",
        )
    _validate_execution_graph(package_names, edges)

    outcome = expected["outcome"]
    if outcome in {"ok", "error"}:
        result_packages = expected["result"]["packages"]
        result_names = [package["name"] for package in result_packages]
        if len(result_names) != len(set(result_names)):
            raise bootstrap.ConformanceError(
                "EXECUTION_RESULT_PACKAGE_DUPLICATE",
                "execution result package names must be unique",
            )
        if result_names != sorted(result_names):
            raise bootstrap.ConformanceError(
                "EXECUTION_RESULT_NOT_CANONICAL",
                "execution result packages must be sorted by name",
            )
        if set(result_names) != package_names:
            raise bootstrap.ConformanceError(
                "EXECUTION_RESULT_PACKAGE_MISMATCH",
                "execution result must classify every input package exactly once",
            )
        package_by_name = {package["name"]: package for package in packages}
        for package_result in result_packages:
            command_results = package_result["commands"]
            indices = [command["index"] for command in command_results]
            if indices != list(range(len(command_results))):
                raise bootstrap.ConformanceError(
                    "EXECUTION_COMMAND_INDEX_INVALID",
                    f"command indices are not canonical for {package_result['name']}",
                )
            command_count = len(package_by_name[package_result["name"]]["commands"])
            if len(command_results) != command_count:
                raise bootstrap.ConformanceError(
                    "EXECUTION_COMMAND_COUNT_MISMATCH",
                    f"command result count differs for {package_result['name']}",
                )
            _validate_package_result_state(package_result)

        _validate_execution_outcome_state(
            dry_run=options["dry_run"],
            outcome=outcome,
            result_packages=result_packages,
        )
        result_by_name = {
            package_result["name"]: package_result for package_result in result_packages
        }
        _validate_dependency_result_states(
            package_names=package_names,
            edges=edges,
            result_by_name=result_by_name,
        )


def _load_contract_documents(
    fixture_root: Path,
) -> tuple[
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
]:
    return (
        bootstrap.load_document(fixture_root / "schema.json"),
        bootstrap.load_document(fixture_root / "result.schema.json"),
        bootstrap.load_document(fixture_root / "execution.schema.json"),
        bootstrap.load_document(fixture_root / "execution-policy.schema.json"),
        bootstrap.load_document(fixture_root / "execution-policy.json"),
        bootstrap.load_document(fixture_root / "execution-authority.schema.json"),
    )


def validate_contract(
    fixture_root: Path = DEFAULT_FIXTURE_ROOT,
) -> dict[str, Any]:
    """Validate schemas, policy, digest, and inert execution cases."""

    # Keep the caller's lexical path so the snapshot capture can reject linked
    # ancestors instead of silently replacing them with their targets.
    fixture_root = Path(os.path.abspath(os.fspath(fixture_root)))
    (
        case_schema,
        result_schema,
        execution_schema,
        policy_schema,
        policy,
        authority_schema,
    ) = _load_contract_documents(fixture_root)
    linux_oci_schema = bootstrap.load_document(
        fixture_root / "linux-oci-backend.schema.json"
    )
    for schema in (
        case_schema,
        result_schema,
        execution_schema,
        policy_schema,
        linux_oci_schema,
        authority_schema,
    ):
        bootstrap._schema_errors({}, schema)
    bootstrap._validate_schema(
        policy,
        policy_schema,
        "EXECUTION_POLICY_SCHEMA_INVALID",
    )
    summary = validate_policy_semantics(policy)
    case_root = fixture_root / "execution-cases"
    corpus_snapshot = capture_execution_case_snapshot(case_root)
    digest = corpus_snapshot.corpus_sha256
    if digest != policy["execution_corpus_sha256"]:
        raise bootstrap.ConformanceError(
            "EXECUTION_POLICY_CORPUS_MISMATCH",
            "checked-in execution policy does not match the execution corpus",
        )

    case_ids: set[str] = set()
    for member in corpus_snapshot.members:
        case = bootstrap.strict_load_bytes(member.raw)
        bootstrap._validate_schema(
            case,
            case_schema,
            "EXECUTION_CASE_SCHEMA_INVALID",
        )
        expected = case.get("expected")
        if not isinstance(expected, dict):
            raise bootstrap.ConformanceError(
                "EXECUTION_EXPECTED_INVALID",
                "execution case expected result is missing",
            )
        bootstrap._validate_schema(
            expected,
            result_schema,
            "EXECUTION_RESULT_SCHEMA_INVALID",
        )
        projection = {
            "domain": case.get("domain"),
            "outcome": expected.get("outcome"),
            "input": case.get("input"),
            "result": expected.get("result"),
        }
        bootstrap._validate_schema(
            projection,
            execution_schema,
            "EXECUTION_PROJECTION_SCHEMA_INVALID",
        )
        validate_execution_semantics(case)
        case_id = case["id"]
        if case_id in case_ids:
            raise bootstrap.ConformanceError(
                "EXECUTION_CASE_ID_DUPLICATE",
                f"duplicate execution case id in {member.relative_path}: {case_id}",
            )
        case_ids.add(case_id)

    return {
        "schema_version": 1,
        "execution_case_count": len(corpus_snapshot.members),
        "execution_corpus_sha256": digest,
        "ready_backend_count": summary["ready_backend_count"],
        "adapter_count": summary["adapter_count"],
        "status": "valid",
        "conformance_status": "not-run",
    }


def _platform_name() -> str:
    if sys.platform.startswith("linux"):
        return "linux"
    if sys.platform == "darwin":
        return "darwin"
    if os.name == "nt":
        return "windows"
    return "unsupported"


def _nonpassing_skip(code: str, message: str) -> dict[str, Any]:
    return {
        "status": "skipped",
        "outcome": "skipped",
        "conformance_status": "non-passing",
        "diagnostics": [
            {
                "code": code,
                "severity": "error",
                "message": message,
            }
        ],
    }


def run_case(
    case_path: Path,
    *,
    language: str,
    authority_bundle: Path,
    approved_authority_digest: str,
    expected_commit_oid: str,
    expected_tree_oid: str,
    allow_trusted_execution: bool,
    repository_root: Path = bootstrap.REPO_ROOT,
) -> dict[str, Any]:
    """Reject preflight-only authority before decoding an execution case."""

    del case_path, language  # This tranche never decodes executable case data.
    if not allow_trusted_execution:
        raise bootstrap.ConformanceError(
            "EXECUTION_AUTHORIZATION_REQUIRED",
            "trusted execution requires --allow-trusted-execution",
        )
    if not _is_sha256(approved_authority_digest):
        raise bootstrap.ConformanceError(
            "AUTHORITY_DIGEST_INVALID",
            "approved authority SHA-256 must be 64 lowercase hexadecimal digits",
        )
    authority.authorize_preflight(
        authority_bundle,
        approved_digest=approved_authority_digest,
        expected_commit_oid=expected_commit_oid,
        expected_tree_oid=expected_tree_oid,
        repository_root=repository_root,
    )
    return _nonpassing_skip(
        "EXECUTION_AUTHORITY_SCOPE_UNAVAILABLE",
        "capability-preflight authority cannot authorize an execution case",
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate trusted-execution policy without executing code."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser(
        "validate-contract",
        help="Validate execution schemas, policy, digest, and inert cases.",
    )
    validate_parser.add_argument(
        "--fixture-root",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
    )

    run_parser = subparsers.add_parser(
        "run-case",
        help="Check execution authority and return a fail-closed result.",
    )
    run_parser.add_argument("--case", type=Path, required=True)
    run_parser.add_argument("--language", required=True)
    run_parser.add_argument("--authority-bundle", type=Path, required=True)
    run_parser.add_argument("--approved-authority-sha256", required=True)
    run_parser.add_argument("--source-commit", required=True)
    run_parser.add_argument("--source-tree", required=True)
    run_parser.add_argument("--allow-trusted-execution", action="store_true")
    run_parser.add_argument(
        "--repository-root",
        type=Path,
        default=bootstrap.REPO_ROOT,
    )
    return parser


def _public_failure_message(command: str) -> str:
    """Return one host-independent CLI error message; the code carries detail."""

    if command == "validate-contract":
        return "trusted-execution contract validation failed"
    return "trusted-execution request failed"


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    try:
        arguments = parser.parse_args(argv)
    except SystemExit as error:
        return int(error.code)
    try:
        if arguments.command == "validate-contract":
            output = validate_contract(arguments.fixture_root)
            exit_code = 0
        else:
            output = run_case(
                arguments.case,
                language=arguments.language,
                authority_bundle=arguments.authority_bundle,
                approved_authority_digest=arguments.approved_authority_sha256,
                expected_commit_oid=arguments.source_commit,
                expected_tree_oid=arguments.source_tree,
                allow_trusted_execution=arguments.allow_trusted_execution,
                repository_root=arguments.repository_root,
            )
            exit_code = 1
    except bootstrap.ConformanceError as error:
        print(
            json.dumps(
                {
                    "code": error.code,
                    "message": _public_failure_message(arguments.command),
                    "status": "error",
                },
                sort_keys=True,
            ),
            file=sys.stderr,
        )
        return 2
    print(json.dumps(output, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
