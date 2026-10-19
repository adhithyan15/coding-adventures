"""Safe Python boundary for HL21's canonical JSON shard directories.

The TypeScript validators remain the authoritative schema gates. These helpers
keep older Python authoring/report scripts usable after aggregate ledgers were
removed while refusing path escapes, symlinks, and ambiguous shard identities.

The caller supplies a trusted ledger root. Descendants may be replaced by an
untrusted concurrent process. On Windows, each child is opened relative to a
validated directory handle; writes and deletions remain attached to that
original directory object even if its pathname is later renamed and replaced
with a junction. A moved original directory may move its own contents, but a
replacement pathname cannot redirect an operation to an unrelated outside
directory. On Unix, dir_fd and O_NOFOLLOW provide the corresponding boundary.
"""

import json
import os
import re
import secrets
import stat
from contextlib import contextmanager

if os.name == "nt":
    import ctypes
    import msvcrt
    from ctypes import wintypes

GROUPED_KEYS = (
    "referenceAppendices",
    "glossaries",
    "answerKeys",
    "indexes",
    "targets",
    "handwritten",
)
BOOK_GENERATION_SECTIONS = {
    "referenceAppendices": "reference-appendices.d",
    "glossaries": "glossaries.d",
    "answerKeys": "answer-keys.d",
    "indexes": "indexes.d",
    "targets": "targets.d",
    "handwritten": "handwritten.d",
}
BOOK_GENERATION_META_KEYS = ("version", "sourceBaseUrl")
DANGEROUS_KEYS = frozenset(("__proto__", "constructor", "prototype"))
SAFE_TRACK = re.compile(r"^[a-z][a-z0-9-]*$")
SAFE_SCRIPT = re.compile(r"^[a-z][a-z0-9-]*$")
SAFE_CURRICULUM_ID = re.compile(r"^[A-Z][A-Z0-9-]*$")
SECTION_NAME = re.compile(r"^(\d+)-([A-Z][A-Z0-9-]*)\.json$")
CHAPTER_NAME = re.compile(r"^(\d{4})\.json$")
SCRIPT_ENTRY_NAME = re.compile(
    r"^(\d{4})-(U-[0-9A-F]+(?:-U-[0-9A-F]+)*)\.json$"
)
BOOK_CHAPTER_OWNER = re.compile(r"^([a-z][a-z0-9-]*)-(\d{4})\.json$")
BOOK_SCRIPT_SET_OWNER = re.compile(r"^(\d{4})-([a-z][a-z0-9-]*)\.json$")
BOOK_OUTPUT_OWNER = re.compile(r"^([a-z][a-z0-9-]*)-([a-z0-9][a-z0-9-]*)\.json$")
BOOK_CHAPTER_IDENTITY = re.compile(r"^([a-z][a-z0-9-]*)/(\d{4})$")
SAFE_BOOK_SLUG = re.compile(r"^[a-z0-9][a-z0-9-]*$")
WINDOWS_RESERVED = re.compile(r"^(con|prn|aux|nul|com[1-9]|lpt[1-9])$", re.IGNORECASE)
NOFOLLOW = getattr(os, "O_NOFOLLOW", 0)
DIRECTORY = getattr(os, "O_DIRECTORY", 0)
REPARSE_POINT = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)


def _is_reparse(info):
    return bool(getattr(info, "st_file_attributes", 0) & REPARSE_POINT)


if os.name == "nt":
    class _WindowsFileInformation(ctypes.Structure):
        _fields_ = [
            ("attributes", wintypes.DWORD),
            ("creation_time", wintypes.FILETIME),
            ("access_time", wintypes.FILETIME),
            ("write_time", wintypes.FILETIME),
            ("volume_serial", wintypes.DWORD),
            ("size_high", wintypes.DWORD),
            ("size_low", wintypes.DWORD),
            ("links", wintypes.DWORD),
            ("index_high", wintypes.DWORD),
            ("index_low", wintypes.DWORD),
        ]

    _kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _create_file = _kernel32.CreateFileW
    _create_file.argtypes = (
        wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.LPVOID,
        wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE,
    )
    _create_file.restype = wintypes.HANDLE
    _file_information = _kernel32.GetFileInformationByHandle
    _file_information.argtypes = (wintypes.HANDLE, ctypes.POINTER(_WindowsFileInformation))
    _file_information.restype = wintypes.BOOL
    _close_handle = _kernel32.CloseHandle
    _close_handle.argtypes = (wintypes.HANDLE,)
    _close_handle.restype = wintypes.BOOL
    _INVALID_HANDLE = ctypes.c_void_p(-1).value
    _READ_ATTRIBUTES = 0x80
    _READ_DATA = 0x01
    _SHARE_READ_WRITE = 0x03  # Omit FILE_SHARE_DELETE: pin namespace components.
    _OPEN_EXISTING = 3
    _OPEN_REPARSE_POINT = 0x00200000
    _BACKUP_SEMANTICS = 0x02000000
    _DIRECTORY_ATTRIBUTE = 0x10
    _SYNCHRONIZE = 0x00100000
    _DELETE = 0x00010000
    _READ_CONTROL = 0x00020000
    _SE_FILE_OBJECT = 1
    _DACL_SECURITY_INFORMATION = 0x00000004
    _FILE_OPEN = 1
    _FILE_CREATE = 2
    _FILE_DIRECTORY_FILE = 0x00000001
    _FILE_NON_DIRECTORY_FILE = 0x00000040
    _FILE_SYNCHRONOUS_IO_NONALERT = 0x00000020
    _FILE_OPEN_REPARSE_POINT = 0x00200000


    class _UnicodeString(ctypes.Structure):
        _fields_ = [
            ("Length", wintypes.USHORT),
            ("MaximumLength", wintypes.USHORT),
            ("Buffer", ctypes.c_void_p),
        ]


    class _ObjectAttributes(ctypes.Structure):
        _fields_ = [
            ("Length", wintypes.ULONG),
            ("RootDirectory", wintypes.HANDLE),
            ("ObjectName", ctypes.POINTER(_UnicodeString)),
            ("Attributes", wintypes.ULONG),
            ("SecurityDescriptor", ctypes.c_void_p),
            ("SecurityQualityOfService", ctypes.c_void_p),
        ]


    class _IoStatusBlock(ctypes.Structure):
        _fields_ = [("Status", ctypes.c_void_p), ("Information", ctypes.c_void_p)]


    class _FileDirectoryInformationHeader(ctypes.Structure):
        """Fixed 64-byte prefix of Windows FILE_DIRECTORY_INFORMATION."""
        _fields_ = [
            ("NextEntryOffset", wintypes.ULONG),
            ("FileIndex", wintypes.ULONG),
            ("CreationTime", ctypes.c_int64),
            ("LastAccessTime", ctypes.c_int64),
            ("LastWriteTime", ctypes.c_int64),
            ("ChangeTime", ctypes.c_int64),
            ("EndOfFile", ctypes.c_int64),
            ("AllocationSize", ctypes.c_int64),
            ("FileAttributes", wintypes.ULONG),
            ("FileNameLength", wintypes.ULONG),
        ]


    _ntdll = ctypes.WinDLL("ntdll", use_last_error=True)
    _nt_create_file = _ntdll.NtCreateFile
    _nt_create_file.argtypes = (
        ctypes.POINTER(wintypes.HANDLE), wintypes.DWORD,
        ctypes.POINTER(_ObjectAttributes), ctypes.POINTER(_IoStatusBlock),
        ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD,
        wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD,
    )
    _nt_create_file.restype = ctypes.c_long
    _nt_to_win_error = _ntdll.RtlNtStatusToDosError
    _nt_to_win_error.argtypes = (ctypes.c_long,)
    _nt_to_win_error.restype = wintypes.ULONG
    _nt_set_information = _ntdll.NtSetInformationFile
    _nt_set_information.argtypes = (
        wintypes.HANDLE, ctypes.POINTER(_IoStatusBlock), ctypes.c_void_p,
        wintypes.ULONG, wintypes.ULONG,
    )
    _nt_set_information.restype = ctypes.c_long
    _nt_query_directory_file = _ntdll.NtQueryDirectoryFile
    _nt_query_directory_file.argtypes = (
        wintypes.HANDLE, wintypes.HANDLE, ctypes.c_void_p, ctypes.c_void_p,
        ctypes.POINTER(_IoStatusBlock), ctypes.c_void_p, wintypes.ULONG,
        wintypes.ULONG, wintypes.BOOLEAN, ctypes.c_void_p, wintypes.BOOLEAN,
    )
    _nt_query_directory_file.restype = ctypes.c_long
    _write_file = _kernel32.WriteFile
    _write_file.argtypes = (
        wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD,
        ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p,
    )
    _write_file.restype = wintypes.BOOL
    _flush_file = _kernel32.FlushFileBuffers
    _flush_file.argtypes = (wintypes.HANDLE,)
    _flush_file.restype = wintypes.BOOL
    _advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    _get_security_info = _advapi32.GetSecurityInfo
    _get_security_info.argtypes = (
        wintypes.HANDLE, wintypes.DWORD, wintypes.DWORD,
        ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p),
        ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p),
    )
    _get_security_info.restype = wintypes.DWORD
    _local_free = _kernel32.LocalFree
    _local_free.argtypes = (ctypes.c_void_p,)
    _local_free.restype = ctypes.c_void_p


    class _FileRenameInformation(ctypes.Structure):
        _fields_ = [
            ("ReplaceIfExists", wintypes.BOOLEAN),
            ("RootDirectory", wintypes.HANDLE),
            ("FileNameLength", wintypes.ULONG),
            ("FileName", wintypes.WCHAR * 1),
        ]


    class _FileDispositionInformation(ctypes.Structure):
        _fields_ = [("DeleteFile", wintypes.BOOLEAN)]


    def _windows_set_information(handle, buffer, size, kind):
        io_status = _IoStatusBlock()
        status = _nt_set_information(handle, ctypes.byref(io_status), buffer, size, kind)
        if status < 0:
            raise ctypes.WinError(_nt_to_win_error(status))


    def _windows_rename_relative(handle, directory_handle, name):
        encoded_name = name.encode("utf-16-le")
        buffer = ctypes.create_string_buffer(ctypes.sizeof(_FileRenameInformation) + len(encoded_name))
        info = ctypes.cast(buffer, ctypes.POINTER(_FileRenameInformation)).contents
        info.ReplaceIfExists = True
        info.RootDirectory = directory_handle
        info.FileNameLength = len(encoded_name)
        ctypes.memmove(ctypes.addressof(buffer) + _FileRenameInformation.FileName.offset,
                       encoded_name, len(encoded_name))
        _windows_set_information(handle, buffer, len(buffer), 10)  # FileRenameInformation


    def _windows_dispose(handle):
        info = _FileDispositionInformation(True)
        _windows_set_information(handle, ctypes.byref(info), ctypes.sizeof(info), 13)


    def _windows_write_handle(handle, body):
        data = body.encode("utf-8")
        buffer = ctypes.create_string_buffer(data)
        written = wintypes.DWORD()
        if not _write_file(handle, buffer, len(data), ctypes.byref(written), None):
            raise ctypes.WinError(ctypes.get_last_error())
        if written.value != len(data):
            raise OSError(f"short Windows ledger write: {written.value} of {len(data)} bytes")
        if not _flush_file(handle):
            raise ctypes.WinError(ctypes.get_last_error())


    def _windows_security_descriptor(source):
        """Get the existing shard's DACL for secure replacement creation."""
        dacl = ctypes.c_void_p()
        descriptor = ctypes.c_void_p()
        error = _get_security_info(
            source, _SE_FILE_OBJECT, _DACL_SECURITY_INFORMATION,
            None, None, ctypes.byref(dacl), None, ctypes.byref(descriptor),
        )
        if error:
            raise ctypes.WinError(error)
        return descriptor


    def _windows_open_relative(
        parent, name, *, directory, create=False, access=None, security_descriptor=None,
    ):
        """Resolve exactly one child relative to an already-open directory handle."""
        if not name or name in (".", "..") or "\\" in name or "/" in name:
            raise ValueError(f"unsafe relative ledger name: {name!r}")
        name_buffer = ctypes.create_unicode_buffer(name)
        size = len(name.encode("utf-16-le"))
        unicode_name = _UnicodeString(size, size, ctypes.cast(name_buffer, ctypes.c_void_p))
        attributes = _ObjectAttributes(
            ctypes.sizeof(_ObjectAttributes), parent, ctypes.pointer(unicode_name),
            0x40, security_descriptor, None,  # OBJ_CASE_INSENSITIVE
        )
        result = wintypes.HANDLE()
        io_status = _IoStatusBlock()
        desired_access = access if access is not None else _READ_ATTRIBUTES | _SYNCHRONIZE
        options = (_FILE_DIRECTORY_FILE if directory else _FILE_NON_DIRECTORY_FILE)
        options |= _FILE_SYNCHRONOUS_IO_NONALERT | _FILE_OPEN_REPARSE_POINT
        status = _nt_create_file(
            ctypes.byref(result), desired_access, ctypes.byref(attributes),
            ctypes.byref(io_status), None, 0x80, _SHARE_READ_WRITE,
            _FILE_CREATE if create else _FILE_OPEN, options, None, 0,
        )
        if status < 0:
            raise ctypes.WinError(_nt_to_win_error(status))
        handle = result.value
        try:
            info = _WindowsFileInformation()
            if not _file_information(handle, ctypes.byref(info)):
                raise ctypes.WinError(ctypes.get_last_error())
            if info.attributes & REPARSE_POINT or bool(info.attributes & _DIRECTORY_ATTRIBUTE) != directory:
                raise ValueError(f"refusing reparse or wrong-kind ledger name: {name}")
            return handle
        except BaseException:
            _close_handle(handle)
            raise


    def _windows_open_checked(path, *, directory, read_data=False):
        """Open the named object itself and reject junctions and other reparse points."""
        access = _READ_ATTRIBUTES | (_READ_DATA if read_data else 0)
        flags = _OPEN_REPARSE_POINT | (_BACKUP_SEMANTICS if directory else 0)
        handle = _create_file(path, access, _SHARE_READ_WRITE, None, _OPEN_EXISTING, flags, None)
        if handle == _INVALID_HANDLE:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            info = _WindowsFileInformation()
            if not _file_information(handle, ctypes.byref(info)):
                raise ctypes.WinError(ctypes.get_last_error())
            if info.attributes & REPARSE_POINT or bool(info.attributes & _DIRECTORY_ATTRIBUTE) != directory:
                raise ValueError(f"refusing reparse or wrong-kind ledger path: {path}")
            return handle
        except BaseException:
            _close_handle(handle)
            raise


    @contextmanager
    def _windows_directory(root, *parts, list_entries=False):
        """Anchor each untrusted component to its already-open parent handle."""
        handles = []
        try:
            handles.append(_windows_open_checked(
                _root(root), directory=True, read_data=list_entries and not parts,
            ))
            for index, part in enumerate(parts):
                access = _READ_ATTRIBUTES | _SYNCHRONIZE
                if list_entries and index == len(parts) - 1:
                    access |= _READ_DATA  # FILE_LIST_DIRECTORY on the final handle.
                handles.append(_windows_open_relative(
                    handles[-1], part, directory=True, access=access,
                ))
            yield handles[-1]
        finally:
            for handle in reversed(handles):
                _close_handle(handle)


    def _windows_read_regular(directory_handle, name):
        handle = _windows_open_relative(
            directory_handle, name, directory=False,
            access=_READ_DATA | _READ_ATTRIBUTES | _SYNCHRONIZE,
        )
        try:
            descriptor = msvcrt.open_osfhandle(handle, os.O_RDONLY)
        except BaseException:
            _close_handle(handle)
            raise
        with os.fdopen(descriptor, encoding="utf-8") as stream:
            return stream.read()


    def _windows_query_directory(directory_handle):
        """Enumerate the opened directory object, never its mutable pathname.

        NtQueryDirectoryFile returns batches of variable-length, 8-byte-aligned
        records. Validate every offset and length before decoding names: an
        incomplete batch must fail closed rather than select partial owners.
        """
        header_size = ctypes.sizeof(_FileDirectoryInformationHeader)
        buffer = ctypes.create_string_buffer(65536)
        entries = []
        while True:
            io_status = _IoStatusBlock()
            status = _nt_query_directory_file(
                directory_handle, None, None, None, ctypes.byref(io_status),
                buffer, len(buffer), 1, False, None, False,
            )
            if status & 0xFFFFFFFF == 0x80000006:  # STATUS_NO_MORE_FILES
                return entries
            if status != 0:
                raise ctypes.WinError(_nt_to_win_error(status))
            length = io_status.Information or 0
            if length < header_size or length > len(buffer):
                raise OSError("invalid Windows ledger directory batch length")
            offset = 0
            while True:
                if offset + header_size > length:
                    raise OSError("truncated Windows ledger directory entry")
                info = _FileDirectoryInformationHeader.from_buffer_copy(buffer, offset)
                next_offset = info.NextEntryOffset
                if next_offset and (
                    next_offset < header_size or next_offset % 8
                    or offset + next_offset > length
                ):
                    raise OSError("invalid Windows ledger directory entry offset")
                end = offset + next_offset if next_offset else length
                if info.FileNameLength % 2 or offset + header_size + info.FileNameLength > end:
                    raise OSError("invalid Windows ledger directory entry name length")
                raw_name = ctypes.string_at(
                    ctypes.addressof(buffer) + offset + header_size,
                    info.FileNameLength,
                )
                name = raw_name.decode("utf-16-le")
                if name not in (".", ".."):
                    if not name or "\\" in name or "/" in name or "\x00" in name:
                        raise ValueError(f"unsafe Windows ledger directory entry: {name!r}")
                    entries.append((name, info.FileAttributes))
                if not next_offset:
                    break
                offset = end


    def _windows_checked_entry(directory_handle, name, *, directory):
        """Re-open one enumerated name beneath the pinned parent, no-follow."""
        handle = _windows_open_relative(directory_handle, name, directory=directory)
        _close_handle(handle)


def _safe_track(track):
    if not isinstance(track, str) or not SAFE_TRACK.fullmatch(track):
        raise ValueError(f"unsafe track id: {track!r}")
    return track


def _safe_book_language(language):
    language = _safe_track(language)
    if WINDOWS_RESERVED.fullmatch(language):
        raise ValueError(f"unsafe book-generation language: {language!r}")
    return language


def _safe_script(script):
    if not isinstance(script, str) or not SAFE_SCRIPT.fullmatch(script):
        raise ValueError(f"unsafe script id: {script!r}")
    return script


def _script_entry_id(glyph):
    if not isinstance(glyph, str) or not glyph:
        raise ValueError(f"script shard entry has no non-empty glyph: {glyph!r}")
    return "-".join(f"U-{ord(character):X}" for character in glyph)


def _root(root):
    real_root = os.path.realpath(root)
    if not os.path.isdir(real_root):
        raise ValueError(f"ledger root is not a directory: {root}")
    return real_root


@contextmanager
def _unix_directory(root, *parts):
    """Pin every descendant to its already-open parent, rejecting symlinks."""
    if (
        not NOFOLLOW or not DIRECTORY
        or os.open not in os.supports_dir_fd
        or os.listdir not in os.supports_fd
        or os.stat not in os.supports_dir_fd
        or os.stat not in os.supports_follow_symlinks
    ):
        raise OSError("descriptor-relative no-follow ledger reads are unavailable")
    current = _root(root)
    descriptor = os.open(current, os.O_RDONLY | DIRECTORY | NOFOLLOW)
    try:
        for part in parts:
            if not isinstance(part, str) or part in ("", ".", "..") or os.sep in part:
                raise ValueError(f"unsafe ledger path component: {part!r}")
            child_path = os.path.join(current, part)
            try:
                child = os.open(part, os.O_RDONLY | DIRECTORY | NOFOLLOW, dir_fd=descriptor)
            except FileNotFoundError as cause:
                raise ValueError(f"missing ledger directory: {child_path}") from cause
            except OSError as cause:
                raise ValueError(f"refusing non-directory ledger ancestor: {child_path}") from cause
            previous = descriptor
            descriptor = child
            os.close(previous)
            current = child_path
        yield descriptor
    finally:
        os.close(descriptor)


def _directory(root, *parts):
    """Return a real directory below root, refusing every symlink component."""
    real_root = _root(root)
    current = real_root
    for part in parts:
        if not isinstance(part, str) or part in ("", ".", "..") or os.sep in part:
            raise ValueError(f"unsafe ledger path component: {part!r}")
        current = os.path.join(current, part)
        try:
            info = os.lstat(current)
        except FileNotFoundError as cause:
            raise ValueError(f"missing ledger directory: {current}") from cause
        if _is_reparse(info) or stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
            raise ValueError(f"refusing non-directory ledger ancestor: {current}")

    relative = os.path.relpath(os.path.realpath(current), real_root)
    if relative == os.pardir or relative.startswith(os.pardir + os.sep) or os.path.isabs(relative):
        raise ValueError(f"ledger path escapes root: {current}")
    return current


def _optional_directory(root, *parts):
    """True when an optional section directory exists (and is a real one).

    An absent final component means no section; anything present must pass
    every guard of ``_directory`` (no symlink, a real directory, inside root).
    """
    parent = _directory(root, *parts[:-1])
    name = parts[-1]
    if not isinstance(name, str) or name in ("", ".", "..") or os.sep in name:
        raise ValueError(f"unsafe ledger path component: {name!r}")
    try:
        os.lstat(os.path.join(parent, name))
    except FileNotFoundError:
        return False
    _directory(root, *parts)
    return True


def _file(root, *parts, must_exist=True):
    directory = _directory(root, *parts[:-1])
    name = parts[-1]
    if not isinstance(name, str) or name in ("", ".", "..") or os.sep in name:
        raise ValueError(f"unsafe ledger filename: {name!r}")
    path = os.path.join(directory, name)
    if not os.path.lexists(path):
        if must_exist:
            raise ValueError(f"missing ledger shard: {path}")
        return path
    info = os.lstat(path)
    if _is_reparse(info) or stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
        raise ValueError(f"refusing non-file ledger shard: {path}")
    return path


def _read(root, *parts):
    path = _file(root, *parts)
    if os.name == "nt":
        with _windows_directory(root, *parts[:-1]) as directory_handle:
            return json.loads(_windows_read_regular(directory_handle, parts[-1]))
    with _unix_directory(root, *parts[:-1]) as directory_fd:
        try:
            descriptor = os.open(parts[-1], os.O_RDONLY | NOFOLLOW, dir_fd=directory_fd)
        except OSError as cause:
            raise ValueError(f"refusing non-file ledger shard: {path}") from cause
        with os.fdopen(descriptor, encoding="utf-8") as handle:
            if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
                raise ValueError(f"refusing non-file ledger shard: {path}")
            return json.load(handle)


if os.name == "nt":
    def _windows_write_if_changed(root, parts, body):
        _file(root, *parts, must_exist=False)
        with _windows_directory(root, *parts[:-1]) as directory_handle:
            previous_handle = None
            try:
                previous_handle = _windows_open_relative(
                    directory_handle, parts[-1], directory=False,
                    access=_READ_DATA | _READ_ATTRIBUTES | _READ_CONTROL | _SYNCHRONIZE,
                )
            except FileNotFoundError:
                previous = None
            try:
                if previous_handle is not None:
                    previous = _windows_read_regular(directory_handle, parts[-1])
                if previous == body:
                    return
                temporary = f".{parts[-1]}.{os.getpid()}.{secrets.token_hex(8)}.tmp"
                descriptor = (
                    _windows_security_descriptor(previous_handle)
                    if previous_handle is not None else None
                )
                try:
                    handle = _windows_open_relative(
                        directory_handle, temporary, directory=False, create=True,
                        access=_READ_ATTRIBUTES | _SYNCHRONIZE | _DELETE | 0x02,
                        security_descriptor=descriptor,
                    )
                finally:
                    if descriptor is not None:
                        _local_free(descriptor)
                try:
                    if previous_handle is not None:
                        _close_handle(previous_handle)
                        previous_handle = None
                    _windows_write_handle(handle, body)
                    _windows_rename_relative(handle, directory_handle, parts[-1])
                except BaseException as cause:
                    try:
                        _windows_dispose(handle)
                    except OSError as cleanup_error:
                        raise cleanup_error from cause
                    raise
                finally:
                    _close_handle(handle)
            finally:
                if previous_handle is not None:
                    _close_handle(previous_handle)


    def _windows_remove(root, parts):
        _file(root, *parts)
        with _windows_directory(root, *parts[:-1]) as directory_handle:
            handle = _windows_open_relative(
                directory_handle, parts[-1], directory=False,
                access=_READ_ATTRIBUTES | _SYNCHRONIZE | _DELETE,
            )
            try:
                _windows_dispose(handle)
            finally:
                _close_handle(handle)


def _write_if_changed(root, *parts, value):
    """Atomically replace one regular shard without ever following its name."""
    body = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
    if os.name == "nt":
        return _windows_write_if_changed(root, parts, body)
    path = _file(root, *parts, must_exist=False)
    directory = os.path.dirname(path)
    name = os.path.basename(path)
    directory_fd = os.open(directory, os.O_RDONLY | DIRECTORY | NOFOLLOW)
    temporary = f".{name}.{os.getpid()}.{secrets.token_hex(8)}.tmp"
    try:
        mode = 0o644
        if os.path.lexists(path):
            info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            if stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
                raise ValueError(f"refusing non-file ledger shard: {path}")
            mode = stat.S_IMODE(info.st_mode)
            descriptor = os.open(name, os.O_RDONLY | NOFOLLOW, dir_fd=directory_fd)
            with os.fdopen(descriptor, encoding="utf-8") as handle:
                if handle.read() == body:
                    return

        descriptor = os.open(
            temporary,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | NOFOLLOW,
            mode,
            dir_fd=directory_fd,
        )
        try:
            with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
                handle.write(body)
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
        except BaseException:
            try:
                os.unlink(temporary, dir_fd=directory_fd)
            except FileNotFoundError:
                pass
            raise
    finally:
        os.close(directory_fd)


def _remove(root, *parts):
    if os.name == "nt":
        return _windows_remove(root, parts)
    path = _file(root, *parts)
    directory_fd = os.open(os.path.dirname(path), os.O_RDONLY | DIRECTORY | NOFOLLOW)
    try:
        info = os.stat(os.path.basename(path), dir_fd=directory_fd, follow_symlinks=False)
        if stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
            raise ValueError(f"refusing non-file ledger shard: {path}")
        os.unlink(os.path.basename(path), dir_fd=directory_fd)
    finally:
        os.close(directory_fd)


def _files(root, *parts):
    directory = _directory(root, *parts)
    names = []
    if os.name == "nt":
        with _windows_directory(root, *parts, list_entries=True) as directory_handle:
            for name, attributes in _windows_query_directory(directory_handle):
                if not name.endswith(".json"):
                    continue
                if attributes & REPARSE_POINT or attributes & _DIRECTORY_ATTRIBUTE:
                    raise ValueError(f"refusing non-file ledger shard: {directory}/{name}")
                _windows_checked_entry(directory_handle, name, directory=False)
                names.append(name)
        return sorted(names)
    with _unix_directory(root, *parts) as directory_fd:
        for name in os.listdir(directory_fd):
            if not name.endswith(".json"):
                continue
            info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            if _is_reparse(info) or stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
                raise ValueError(f"refusing non-file ledger shard: {directory}/{name}")
            names.append(name)
    return sorted(names)


def _script_entries(root, script, section, glyph_key, glyph_owners):
    entries = []
    ordinals = set()
    directory_parts = ("data", "scripts", f"{script}.d", section)
    for name in _files(root, *directory_parts):
        match = SCRIPT_ENTRY_NAME.fullmatch(name)
        if match is None:
            raise ValueError(f"malformed {section} script shard name: {name}")
        ordinal, entry_id = match.groups()
        if ordinal in ordinals:
            raise ValueError(f"duplicate {section} ordinal: {ordinal}")
        ordinals.add(ordinal)
        value = _read(root, *directory_parts, name)
        glyph = value.get(glyph_key) if isinstance(value, dict) else None
        expected = _script_entry_id(glyph)
        if entry_id != expected:
            raise ValueError(f"{section} identity does not match shard name: {name}")
        if glyph in glyph_owners:
            raise ValueError(
                f"script glyph {glyph!r} is owned by both {glyph_owners[glyph]} and {name}"
            )
        glyph_owners[glyph] = name
        entries.append(value)
    return entries


def load_script_inventory(root, script):
    """Load one canonical script inventory without a compatibility aggregate."""
    script = _safe_script(script)
    meta = _read(root, "data", "scripts", f"{script}.d", "_meta.json")
    if not isinstance(meta, dict) or meta.get("script") != script:
        raise ValueError(f"{script}: script metadata id mismatch")
    if "letters" in meta or "marks" in meta or "digits" in meta:
        raise ValueError(
            f"{script}: script metadata must not carry letters, marks or digits"
        )
    glyph_owners = {}
    inventory = {
        **meta,
        "letters": _script_entries(
            root, script, "letters", "glyph", glyph_owners
        ),
        "marks": _script_entries(root, script, "marks", "mark", glyph_owners),
    }
    # `digits/` is optional (HL25): a script with its own numerals keeps one
    # file per digit row, identified by its glyph like a letter. A script with
    # no such directory gets no `digits` key, exactly as before the section
    # existed, matching the TypeScript reader.
    digits = _optional_directory(root, "data", "scripts", f"{script}.d", "digits")
    if digits:
        entries = _script_entries(root, script, "digits", "glyph", glyph_owners)
        if entries:
            inventory["digits"] = entries
    return inventory


def load_script(root, script):
    """Load a monolithic or shard-native canonical script inventory.

    The logical provenance remains ``<script>.json`` for callers and reports;
    the sibling ``<script>.d`` directory merely changes the storage boundary.
    If the shard path exists, it must be a real directory and becomes the only
    source of truth rather than falling back to a possibly stale aggregate.
    """
    script = _safe_script(script)
    scripts = _directory(root, "data", "scripts")
    shard_directory = os.path.join(scripts, f"{script}.d")
    try:
        info = os.lstat(shard_directory)
    except FileNotFoundError:
        return _read(root, "data", "scripts", f"{script}.json")
    if _is_reparse(info) or stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
        raise ValueError(f"refusing non-directory ledger ancestor: {shard_directory}")
    return load_script_inventory(root, script)


def _chapter_entries(root, track):
    entries = []
    seen = set()
    for name in _files(root, track, "chapters.d"):
        if name == "_meta.json":
            continue
        match = CHAPTER_NAME.fullmatch(name)
        if match is None:
            raise ValueError(f"malformed chapter shard name: {name}")
        number = int(match.group(1))
        value = _read(root, track, "chapters.d", name)
        if not isinstance(value, dict) or value.get("chapter") != number:
            raise ValueError(f"chapter identity does not match shard name: {name}")
        if number in seen:
            raise ValueError(f"duplicate chapter identity: {number}")
        seen.add(number)
        entries.append((name, number, value))
    return entries


def load_chapters(root, track):
    track = _safe_track(track)
    meta = _read(root, track, "chapters.d", "_meta.json")
    if not isinstance(meta, dict) or meta.get("language") != track:
        raise ValueError(f"{track}: chapter metadata language mismatch")
    meta["chapters"] = [value for _, _, value in _chapter_entries(root, track)]
    return meta


def write_chapters(root, track, document):
    track = _safe_track(track)
    if not isinstance(document, dict) or document.get("language") != track:
        raise ValueError(f"{track}: chapter document language mismatch")
    chapters = document.get("chapters")
    if not isinstance(chapters, list):
        raise ValueError(f"{track}: chapters must be a list")

    planned = []
    expected = set()
    for chapter in chapters:
        number = chapter.get("chapter") if isinstance(chapter, dict) else None
        if not isinstance(number, int) or isinstance(number, bool) or number < 0 or number > 9999:
            raise ValueError(f"unsafe chapter number: {number!r}")
        name = f"{number:04d}.json"
        if name in expected:
            raise ValueError(f"duplicate chapter identity: {number}")
        expected.add(name)
        planned.append((name, chapter))

    existing = _chapter_entries(root, track)
    for name, chapter in planned:
        _write_if_changed(root, track, "chapters.d", name, value=chapter)
    for name, _, _ in existing:
        if name not in expected:
            _remove(root, track, "chapters.d", name)


def _section_entries(root, track, section):
    out = []
    seen = set()
    for name in _files(root, track, "curriculum.d", section):
        match = SECTION_NAME.fullmatch(name)
        if match is None:
            raise ValueError(f"malformed {section} shard name: {name}")
        entry_id = match.group(2)
        if entry_id in seen:
            raise ValueError(f"duplicate {section} shard identity: {entry_id}")
        seen.add(entry_id)
        value = _read(root, track, "curriculum.d", section, name)
        if section != "spine" and (not isinstance(value, dict) or value.get("id") != entry_id):
            raise ValueError(f"{section} identity does not match shard name: {name}")
        out.append((name, entry_id, value))
    return out


def load_curriculum(root, track):
    track = _safe_track(track)
    meta = _read(root, track, "curriculum.d", "_meta.json")
    if not isinstance(meta, dict) or meta.get("language") != track:
        raise ValueError(f"{track}: curriculum metadata language mismatch")
    key_order = meta.pop("_keys", None)
    sections = {
        "path": [value for _, _, value in _section_entries(root, track, "path")],
        "spine": {
            entry_id: value for _, entry_id, value in _section_entries(root, track, "spine")
        },
        "extensions": [
            value for _, _, value in _section_entries(root, track, "extensions")
        ],
    }
    if key_order is None:
        return {**meta, **sections}
    values = {**meta, **sections}
    return {key: values[key] for key in key_order}


def write_curriculum(root, track, document):
    track = _safe_track(track)
    if not isinstance(document, dict) or document.get("language") != track:
        raise ValueError(f"{track}: curriculum document language mismatch")

    plans = {}
    for section in ("path", "extensions"):
        values = document.get(section)
        if not isinstance(values, list):
            raise ValueError(f"{track}: curriculum {section} must be a list")
        existing_entries = _section_entries(root, track, section)
        existing = {entry_id: name for name, entry_id, _ in existing_entries}
        next_ordinal = max(
            (int(name.split("-", 1)[0]) for name in existing.values()), default=0
        ) + 10
        seen = set()
        planned = []
        for value in values:
            entry_id = value.get("id") if isinstance(value, dict) else None
            if not isinstance(entry_id, str) or not SAFE_CURRICULUM_ID.fullmatch(entry_id):
                raise ValueError(f"unsafe curriculum id: {entry_id!r}")
            if entry_id in seen:
                raise ValueError(f"duplicate curriculum {section} id: {entry_id}")
            seen.add(entry_id)
            name = existing.get(entry_id)
            if name is None:
                name = f"{next_ordinal:04d}-{entry_id}.json"
                next_ordinal += 10
            planned.append((name, value))
        plans[section] = planned

    spine = document.get("spine")
    if not isinstance(spine, dict):
        raise ValueError(f"{track}: curriculum spine must be an object")
    existing_spine = {
        entry_id: name for name, entry_id, _ in _section_entries(root, track, "spine")
    }
    spine_plan = []
    for entry_id, value in spine.items():
        if not isinstance(entry_id, str) or not SAFE_CURRICULUM_ID.fullmatch(entry_id):
            raise ValueError(f"unsafe curriculum spine id: {entry_id!r}")
        name = existing_spine.get(entry_id)
        if name is None:
            raise ValueError(f"{track}: cannot invent shared spine node {entry_id!r}")
        spine_plan.append((name, value))

    for section, planned in plans.items():
        for name, value in planned:
            _write_if_changed(root, track, "curriculum.d", section, name, value=value)
    for name, value in spine_plan:
        _write_if_changed(root, track, "curriculum.d", "spine", name, value=value)


def _reject_dangerous_keys(value, label):
    """Refuse JavaScript prototype keys before Python hands JSON to TS tools."""
    if isinstance(value, dict):
        for key, child in value.items():
            if key in DANGEROUS_KEYS:
                raise ValueError(f"{label}: dangerous key {key!r}")
            _reject_dangerous_keys(child, f"{label}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _reject_dangerous_keys(child, f"{label}[{index}]")


def _strict_directory_entries(root, *parts):
    """Enumerate every direct child, refusing symlinks and special files."""
    directory = _directory(root, *parts)
    entries = []
    if os.name == "nt":
        with _windows_directory(root, *parts, list_entries=True) as directory_handle:
            for name, attributes in _windows_query_directory(directory_handle):
                if attributes & REPARSE_POINT:
                    raise ValueError(
                        f"refusing link or reparse book-generation owner: {directory}/{name}"
                    )
                if attributes & 0x40:  # FILE_ATTRIBUTE_DEVICE is not a regular owner.
                    raise ValueError(
                        f"refusing special book-generation owner: {directory}/{name}"
                    )
                is_directory = bool(attributes & _DIRECTORY_ATTRIBUTE)
                _windows_checked_entry(directory_handle, name, directory=is_directory)
                entries.append((name, is_directory))
        return sorted(entries)
    with _unix_directory(root, *parts) as directory_fd:
        for name in os.listdir(directory_fd):
            info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            if _is_reparse(info) or stat.S_ISLNK(info.st_mode):
                raise ValueError(
                    f"refusing link or reparse book-generation owner: {directory}/{name}"
                )
            if not (stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)):
                raise ValueError(
                    f"refusing special book-generation owner: {directory}/{name}"
                )
            entries.append((name, stat.S_ISDIR(info.st_mode)))
    return sorted(entries)


def _validate_book_generation_tree(root):
    legacy = os.path.join(_directory(root, "core"), "book-generation.json")
    if os.path.lexists(legacy):
        raise ValueError(
            "resurrected book-generation monolith beside canonical owner directory"
        )
    expected_directories = set(BOOK_GENERATION_SECTIONS.values()) | {"script-sets.d"}
    seen_directories = set()
    seen_meta = False
    for name, is_directory in _strict_directory_entries(
        root, "core", "book-generation.d"
    ):
        if name == "_meta.json" and not is_directory:
            seen_meta = True
        elif name in expected_directories and is_directory:
            seen_directories.add(name)
        else:
            kind = "directory" if is_directory else "file"
            raise ValueError(
                f"unexpected book-generation {kind}: core/book-generation.d/{name}"
            )
    if not seen_meta:
        raise ValueError(
            "missing book-generation metadata: core/book-generation.d/_meta.json"
        )
    missing = expected_directories - seen_directories
    if missing:
        raise ValueError(
            "missing book-generation owner directories: " + ", ".join(sorted(missing))
        )


# A section directory MUST exist and MAY be empty, and those are two different
# claims. `handwritten.d` reached zero entries when the last hand-written chapter
# in the corpus was generated; git cannot track an empty directory, so a tracked
# `.gitkeep` holds it open and both readers filter the placeholder out.
#
# This mirrors `readStrictDirectory` in book-generation-shards.ts EXACTLY. The two
# are independent implementations of one contract, and the TypeScript side was
# fixed first -- which is how this reader came to reject a tree the other reader
# had just accepted. If one of them changes, the other has to change with it.
DIRECTORY_PLACEHOLDER = ".gitkeep"


def _strict_owner_names(root, directory):
    names = []
    for name, is_directory in _strict_directory_entries(
        root, "core", "book-generation.d", directory
    ):
        if is_directory:
            raise ValueError(
                f"unexpected nested book-generation owner directory: {directory}/{name}"
            )
        if name == DIRECTORY_PLACEHOLDER:
            continue
        if not name.endswith(".json"):
            raise ValueError(
                f"malformed book-generation owner name: {directory}/{name}"
            )
        names.append(name)
    return names


def _chapter_identity(language, chapter, label):
    language = _safe_book_language(language)
    if (
        not isinstance(chapter, int)
        or isinstance(chapter, bool)
        or chapter < 1
        or chapter > 9999
    ):
        raise ValueError(f"{label}: chapter must be an integer from 1 through 9999")
    return f"{language}/{chapter:04d}"


def _normalise_expected_identities(identities, label):
    if identities is None:
        return None
    if not isinstance(identities, (set, frozenset)):
        raise ValueError(f"{label} must be an independently supplied set")
    normalised = set()
    for identity in identities:
        if (
            not isinstance(identity, str)
            or BOOK_CHAPTER_IDENTITY.fullmatch(identity) is None
        ):
            raise ValueError(f"{label} contains malformed identity: {identity!r}")
        language, ordinal = identity.rsplit("/", 1)
        _safe_book_language(language)
        if int(ordinal) < 1:
            raise ValueError(f"{label} contains invalid chapter identity: {identity!r}")
        normalised.add(f"{language}/{int(ordinal):04d}")
    return normalised


def _assert_exact_identities(actual, expected, label):
    if expected is None:
        return
    missing = expected - actual
    unexpected = actual - expected
    if missing or unexpected:
        details = []
        if missing:
            details.append("missing " + ", ".join(sorted(missing)))
        if unexpected:
            details.append("unexpected " + ", ".join(sorted(unexpected)))
        raise ValueError(
            f"book-generation {label} owner set mismatch: {'; '.join(details)}"
        )


def _output_basename(entry, label):
    if not isinstance(entry, dict):
        raise ValueError(f"{label}: owner value must be a JSON object")
    _reject_dangerous_keys(entry, label)
    language = _safe_book_language(entry.get("language"))
    output = entry.get("output")
    if (
        not isinstance(output, str)
        or not output
        or "\\" in output
        or output.startswith("/")
    ):
        raise ValueError(f"{label}: unsafe output path: {output!r}")
    parts = output.split("/")
    if any(part in ("", ".", "..") for part in parts) or parts[0] != language:
        raise ValueError(f"{label}: unsafe output path: {output!r}")
    filename = parts[-1]
    if not filename.endswith(".tex"):
        raise ValueError(f"{label}: output path must end in .tex: {output!r}")
    basename = filename[: -len(".tex")]
    if SAFE_BOOK_SLUG.fullmatch(basename) is None or WINDOWS_RESERVED.fullmatch(
        basename
    ):
        raise ValueError(f"{label}: unsafe output basename: {basename!r}")
    return language, basename


def _load_script_sets(root):
    script_sets = {}
    previous_ordinal = 0
    for name in _strict_owner_names(root, "script-sets.d"):
        match = BOOK_SCRIPT_SET_OWNER.fullmatch(name)
        if match is None:
            raise ValueError(f"malformed script-set owner name: {name}")
        ordinal, entry_id = match.groups()
        if int(ordinal) <= previous_ordinal:
            raise ValueError(
                f"script-set owner ordinal is out of canonical order: {ordinal}"
            )
        if WINDOWS_RESERVED.fullmatch(entry_id):
            raise ValueError(f"unsafe script-set identity: {entry_id}")
        if entry_id in script_sets:
            raise ValueError(f"duplicate script-set identity: {entry_id}")
        value = _read(root, "core", "book-generation.d", "script-sets.d", name)
        _reject_dangerous_keys(value, f"script-sets.d/{name}")
        if not isinstance(value, list):
            raise ValueError(f"script-sets.d/{name}: owner value must be a JSON array")
        script_sets[entry_id] = value
        previous_ordinal = int(ordinal)
    return script_sets


def _load_chapter_book_section(root, key):
    directory = BOOK_GENERATION_SECTIONS[key]
    records = []
    identities = set()
    for name in _strict_owner_names(root, directory):
        match = BOOK_CHAPTER_OWNER.fullmatch(name)
        if match is None:
            raise ValueError(f"malformed {key} owner name: {name}")
        owner_language, owner_ordinal = match.groups()
        value = _read(root, "core", "book-generation.d", directory, name)
        if not isinstance(value, dict):
            raise ValueError(f"{directory}/{name}: owner value must be a JSON object")
        _reject_dangerous_keys(value, f"{directory}/{name}")
        identity = _chapter_identity(
            value.get("language"), value.get("chapter"), f"{directory}/{name}"
        )
        expected_name = f"{value['language']}-{value['chapter']:04d}.json"
        if (
            name != expected_name
            or owner_language != value["language"]
            or int(owner_ordinal) != value["chapter"]
        ):
            raise ValueError(f"{key} identity does not match owner name: {name}")
        if identity in identities:
            raise ValueError(f"duplicate {key} identity: {identity}")
        identities.add(identity)
        records.append(value)
    return records, identities


def _load_output_book_section(root, key):
    directory = BOOK_GENERATION_SECTIONS[key]
    records = []
    identities = set()
    for name in _strict_owner_names(root, directory):
        if BOOK_OUTPUT_OWNER.fullmatch(name) is None:
            raise ValueError(f"malformed {key} owner name: {name}")
        value = _read(root, "core", "book-generation.d", directory, name)
        language, basename = _output_basename(value, f"{directory}/{name}")
        expected_name = f"{language}-{basename}.json"
        if name != expected_name:
            raise ValueError(f"{key} identity does not match owner name: {name}")
        identity = (language, basename)
        if identity in identities:
            raise ValueError(f"duplicate {key} identity: {language}/{basename}")
        identities.add(identity)
        records.append(value)
    return records


def load_book_generation(
    root,
    *,
    expected_target_identities=None,
    expected_handwritten_identities=None,
):
    """Reconstruct the chapter-owned book ledger from strict direct owners.

    Expected identities are deliberately external to the directory being read.
    Supplying them turns a clean deletion (otherwise indistinguishable from an
    intentionally absent owner) into a deterministic validation failure.
    """
    expected_targets = _normalise_expected_identities(
        expected_target_identities, "expected_target_identities"
    )
    expected_handwritten = _normalise_expected_identities(
        expected_handwritten_identities, "expected_handwritten_identities"
    )
    _validate_book_generation_tree(root)
    meta = _read(root, "core", "book-generation.d", "_meta.json")
    if not isinstance(meta, dict) or tuple(meta) != BOOK_GENERATION_META_KEYS:
        raise ValueError(
            "book-generation metadata must contain exactly version then sourceBaseUrl"
        )
    _reject_dangerous_keys(meta, "book-generation metadata")
    if meta.get("version") != 1 or isinstance(meta.get("version"), bool):
        raise ValueError("book-generation metadata version must be 1")
    source = meta.get("sourceBaseUrl")
    if not isinstance(source, str) or not source.startswith(("https://", "http://")):
        raise ValueError("book-generation metadata sourceBaseUrl must be HTTP(S)")

    script_sets = _load_script_sets(root)
    sections = {}
    actual_targets = set()
    actual_handwritten = set()
    for key in GROUPED_KEYS:
        if key in ("targets", "handwritten"):
            values, identities = _load_chapter_book_section(root, key)
            if key == "targets":
                actual_targets = identities
            else:
                actual_handwritten = identities
        else:
            values = _load_output_book_section(root, key)
        sections[key] = values

    _assert_exact_identities(actual_targets, expected_targets, "target")
    _assert_exact_identities(actual_handwritten, expected_handwritten, "handwritten")
    document = {**meta, "scriptSets": script_sets, **sections}
    _document_owner_plans(document)
    return document


def _document_owner_plans(document):
    expected_keys = (*BOOK_GENERATION_META_KEYS, "scriptSets", *GROUPED_KEYS)
    if not isinstance(document, dict) or tuple(document) != expected_keys:
        raise ValueError("book-generation document has wrong keys or key order")
    _reject_dangerous_keys(document, "book-generation document")
    if document.get("version") != 1 or isinstance(document.get("version"), bool):
        raise ValueError("book-generation document version must be 1")
    if not isinstance(document.get("sourceBaseUrl"), str):
        raise ValueError("book-generation document sourceBaseUrl must be a string")
    if not isinstance(document.get("scriptSets"), dict):
        raise ValueError("book-generation document scriptSets must be an object")
    script_sets = document["scriptSets"]
    for entry_id, values in script_sets.items():
        if (
            not isinstance(entry_id, str)
            or SAFE_BOOK_SLUG.fullmatch(entry_id) is None
            or WINDOWS_RESERVED.fullmatch(entry_id)
        ):
            raise ValueError(f"unsafe book-generation script-set id: {entry_id!r}")
        if not isinstance(values, list):
            raise ValueError(f"book-generation script set {entry_id} must be an array")

    plans = {}
    identities = {"targets": set(), "handwritten": set()}
    outputs = set()
    used_script_sets = set()
    for key in GROUPED_KEYS:
        values = document.get(key)
        if not isinstance(values, list):
            raise ValueError(f"book-generation document {key} must be an array")
        section_plans = []
        seen = set()
        for index, value in enumerate(values):
            label = f"book-generation document {key}[{index}]"
            if key in ("targets", "handwritten"):
                if not isinstance(value, dict):
                    raise ValueError(f"{label}: owner value must be a JSON object")
                identity = _chapter_identity(
                    value.get("language"), value.get("chapter"), label
                )
                _output_basename(value, label)
                name = f"{value['language']}-{value['chapter']:04d}.json"
                identities[key].add(identity)
            else:
                language, basename = _output_basename(value, label)
                identity = (language, basename)
                name = f"{language}-{basename}.json"
            if identity in seen:
                raise ValueError(f"duplicate {key} identity in document: {identity}")
            output = value.get("output") if isinstance(value, dict) else None
            if output in outputs:
                raise ValueError(f"duplicate book-generation output: {output}")
            outputs.add(output)
            script_set = value.get("scriptSet") if isinstance(value, dict) else None
            if script_set is not None:
                if not isinstance(script_set, str) or script_set not in script_sets:
                    raise ValueError(f"{label}: unknown script set {script_set!r}")
                used_script_sets.add(script_set)
            seen.add(identity)
            section_plans.append((name, value))
        if [name for name, _ in section_plans] != sorted(
            name for name, _ in section_plans
        ):
            raise ValueError(
                f"book-generation document {key} must be in owner-name order"
            )
        plans[key] = section_plans
    unused_script_sets = set(script_sets) - used_script_sets
    if unused_script_sets:
        raise ValueError(
            "book-generation script sets have no owning declaration: "
            + ", ".join(sorted(unused_script_sets))
        )
    overlap = identities["targets"] & identities["handwritten"]
    if overlap:
        raise ValueError(
            "book-generation chapter is both target and handwritten: "
            + ", ".join(sorted(overlap))
        )
    return plans, identities


def write_book_generation_language(
    root,
    language,
    document,
    *,
    expected_target_identities=None,
    expected_handwritten_identities=None,
):
    """Synchronise one language's direct owners without touching any other owner."""
    language = _safe_book_language(language)
    plans, identities = _document_owner_plans(document)
    expected_targets = _normalise_expected_identities(
        expected_target_identities, "expected_target_identities"
    )
    expected_handwritten = _normalise_expected_identities(
        expected_handwritten_identities, "expected_handwritten_identities"
    )
    _assert_exact_identities(identities["targets"], expected_targets, "target")
    _assert_exact_identities(
        identities["handwritten"], expected_handwritten, "handwritten"
    )

    current = load_book_generation(root)
    if any(
        current[key] != document[key]
        for key in (*BOOK_GENERATION_META_KEYS, "scriptSets")
    ):
        raise ValueError(
            "writer may not change book-generation metadata or script sets"
        )
    for key in GROUPED_KEYS:
        current_foreign = [
            value for value in current[key] if value.get("language") != language
        ]
        desired_foreign = [
            value for value in document[key] if value.get("language") != language
        ]
        if current_foreign != desired_foreign:
            raise ValueError(
                f"writer for {language} may not change foreign {key} owners"
            )

    for key in GROUPED_KEYS:
        directory = BOOK_GENERATION_SECTIONS[key]
        desired = {
            name: value
            for name, value in plans[key]
            if value.get("language") == language
        }
        existing = {}
        for name in _strict_owner_names(root, directory):
            value = _read(root, "core", "book-generation.d", directory, name)
            if isinstance(value, dict) and value.get("language") == language:
                existing[name] = value
        for name, value in desired.items():
            _write_if_changed(
                root, "core", "book-generation.d", directory, name, value=value
            )
        for name in existing:
            if name not in desired:
                _remove(root, "core", "book-generation.d", directory, name)

    written = load_book_generation(
        root,
        expected_target_identities=expected_targets,
        expected_handwritten_identities=expected_handwritten,
    )
    if written != document:
        raise ValueError(f"book-generation write for {language} did not round-trip")
