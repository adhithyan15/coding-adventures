"""Safe Python boundary for HL21's canonical JSON shard directories.

The TypeScript validators remain the authoritative schema gates. These helpers
keep older Python authoring/report scripts usable after aggregate ledgers were
removed while refusing path escapes, symlinks, and ambiguous shard identities.
"""

import json
import os
import re
import secrets
import stat

GROUPED_KEYS = (
    "referenceAppendices",
    "glossaries",
    "answerKeys",
    "indexes",
    "targets",
    "handwritten",
)
SAFE_TRACK = re.compile(r"^[a-z][a-z0-9-]*$")
SAFE_CURRICULUM_ID = re.compile(r"^[A-Z][A-Z0-9-]*$")
SECTION_NAME = re.compile(r"^(\d+)-([A-Z][A-Z0-9-]*)\.json$")
CHAPTER_NAME = re.compile(r"^(\d{4})\.json$")
NOFOLLOW = getattr(os, "O_NOFOLLOW", 0)
DIRECTORY = getattr(os, "O_DIRECTORY", 0)


def _safe_track(track):
    if not isinstance(track, str) or not SAFE_TRACK.fullmatch(track):
        raise ValueError(f"unsafe track id: {track!r}")
    return track


def _root(root):
    real_root = os.path.realpath(root)
    if not os.path.isdir(real_root):
        raise ValueError(f"ledger root is not a directory: {root}")
    return real_root


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
        if stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
            raise ValueError(f"refusing non-directory ledger ancestor: {current}")

    relative = os.path.relpath(os.path.realpath(current), real_root)
    if relative == os.pardir or relative.startswith(os.pardir + os.sep) or os.path.isabs(relative):
        raise ValueError(f"ledger path escapes root: {current}")
    return current


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
    if stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
        raise ValueError(f"refusing non-file ledger shard: {path}")
    return path


def _read(root, *parts):
    path = _file(root, *parts)
    descriptor = os.open(path, os.O_RDONLY | NOFOLLOW)
    with os.fdopen(descriptor, encoding="utf-8") as handle:
        return json.load(handle)


def _write_if_changed(root, *parts, value):
    """Atomically replace one regular shard without ever following its name."""
    body = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
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
    with os.scandir(directory) as entries:
        for entry in entries:
            if not entry.name.endswith(".json"):
                continue
            info = entry.stat(follow_symlinks=False)
            if entry.is_symlink() or not stat.S_ISREG(info.st_mode):
                raise ValueError(f"refusing non-file ledger shard: {entry.path}")
            names.append(entry.name)
    return sorted(names)


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


def load_book_generation(root):
    document = _read(root, "core", "book-generation.d", "_meta.json")
    for key in GROUPED_KEYS:
        document[key] = []
    for name in _files(root, "core", "book-generation.d"):
        if name == "_meta.json":
            continue
        shard = _read(root, "core", "book-generation.d", name)
        for key in GROUPED_KEYS:
            document[key].extend(shard.get(key, []))
    return document


def write_book_generation_language(root, language, document):
    language = _safe_track(language)
    shard = {}
    for key in GROUPED_KEYS:
        values = [entry for entry in document[key] if entry["language"] == language]
        if values:
            shard[key] = values
    _write_if_changed(
        root,
        "core",
        "book-generation.d",
        f"{language}.json",
        value=shard,
    )
