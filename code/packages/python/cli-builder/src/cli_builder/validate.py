"""Standalone validation functions for CLI Builder specs.

=== Why standalone functions? ===

The ``SpecLoader`` class validates specs as part of loading them. It raises
``SpecError`` on the first problem it finds — which is perfect when you want
to *use* the spec, but inconvenient when you just want to *check* it.

These functions wrap ``SpecLoader.load()`` in a try/except and return a
``ValidationResult`` instead of raising. This makes them ideal for:

- **Linters and CI checks** — validate a spec file without crashing the process.
- **Editor integrations** — show errors inline without exception handling.
- **Testing** — assert that a spec is valid (or invalid with specific errors)
  using simple attribute checks.

=== Architecture note ===

Both functions reuse the exact same validation logic as ``SpecLoader.load()``.
There is no separate validation codepath to keep in sync. The only difference
is error handling: exceptions become list entries.

=== Current limitation ===

Because ``SpecLoader.load()`` raises on the *first* error it finds, the
``errors`` list will always contain at most one entry. A future version could
collect multiple errors, but the single-error behavior is still useful — it
tells you *what* is wrong and *where*.
"""

from __future__ import annotations

import tempfile

from cli_builder.errors import SpecError
from cli_builder.spec_loader import SpecLoader
from cli_builder.types import ValidationResult


def validate_spec(spec_path: str) -> ValidationResult:
    """Validate a CLI Builder JSON spec file without raising exceptions.

    This function loads the file at ``spec_path``, parses the JSON, and runs
    every validation rule that ``SpecLoader.load()`` would apply. Instead of
    raising ``SpecError`` on failure, it returns a ``ValidationResult`` with
    ``valid=False`` and a descriptive error message.

    === Example ===

    ::

        result = validate_spec("myapp.json")
        if not result.valid:
            print(f"Invalid spec: {result.errors[0]}")

    Args:
        spec_path: Path to the JSON spec file (absolute or relative).

    Returns:
        A ``ValidationResult``. If the spec is valid, ``valid`` is ``True``
        and ``errors`` is empty. If invalid, ``valid`` is ``False`` and
        ``errors`` contains one or more descriptive strings.
    """
    try:
        SpecLoader(spec_path).load()
        return ValidationResult(valid=True, errors=[])
    except SpecError as exc:
        # SpecError.message contains the human-readable description without
        # the "CliBuilder spec error: " prefix that __str__ adds.
        return ValidationResult(valid=False, errors=[exc.message])
    except Exception as exc:
        # Catch-all for truly unexpected failures (e.g., permission denied
        # wrapping, OS-level issues not caught by SpecLoader). We don't want
        # the caller to have to handle exceptions at all.
        return ValidationResult(valid=False, errors=[str(exc)])


def validate_spec_string(json_string: str) -> ValidationResult:
    """Validate a CLI Builder spec from an in-memory JSON string.

    This is the same as ``validate_spec()``, but reads from a string instead
    of a file path. Internally it writes the string to a temporary file and
    delegates to ``SpecLoader``.

    === Why a temp file? ===

    ``SpecLoader`` is designed around file paths — it reads, parses, and
    validates in one method. Rather than duplicating that logic or refactoring
    the class (which would be a larger change), we write to a temp file. The
    overhead is negligible for validation use cases (specs are small JSON
    documents, typically under 10 KB).

    === Example ===

    ::

        spec_json = '{"cli_builder_spec_version": "1.0", ...}'
        result = validate_spec_string(spec_json)
        assert result.valid

    Args:
        json_string: A string containing JSON. Does not need to be valid
            JSON — invalid JSON will produce ``valid=False`` with an
            appropriate error message.

    Returns:
        A ``ValidationResult``, same as ``validate_spec()``.
    """
    # Write to a temporary file so we can reuse SpecLoader unchanged.
    # The temp file is created with delete=False so it persists long enough
    # for SpecLoader to read it. Python's garbage collector will eventually
    # clean up the file handle; the OS cleans up the temp directory.
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            suffix=".json",
            delete=False,
            encoding="utf-8",
        ) as f:
            f.write(json_string)
            tmp_path = f.name
        return validate_spec(tmp_path)
    except Exception as exc:
        # If even creating the temp file fails (disk full, etc.), report it.
        return ValidationResult(valid=False, errors=[str(exc)])
