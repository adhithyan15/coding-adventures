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
    if "letters" in meta or "marks" in meta:
        raise ValueError(f"{script}: script metadata must not carry letters or marks")
    glyph_owners = {}
    return {
        **meta,
        "letters": _script_entries(
            root, script, "letters", "glyph", glyph_owners
        ),
        "marks": _script_entries(root, script, "marks", "mark", glyph_owners),
    }


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
    if stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
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
    with os.scandir(directory) as iterator:
        for entry in iterator:
            info = entry.stat(follow_symlinks=False)
            if entry.is_symlink():
                raise ValueError(
                    f"refusing symbolic-link book-generation owner: {entry.path}"
                )
            if not (stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)):
                raise ValueError(
                    f"refusing special book-generation owner: {entry.path}"
                )
            entries.append((entry.name, stat.S_ISDIR(info.st_mode)))
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


def _strict_owner_names(root, directory):
    names = []
    for name, is_directory in _strict_directory_entries(
        root, "core", "book-generation.d", directory
    ):
        if is_directory:
            raise ValueError(
                f"unexpected nested book-generation owner directory: {directory}/{name}"
            )
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
