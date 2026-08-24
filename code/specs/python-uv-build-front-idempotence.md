# Python uv BUILD-front idempotence audit

## Status

This document defines the classification contract for the Python package
fronts that create a package-local uv virtual environment. It is an audit
contract, not permission to bulk-edit every package it discovers.

The authoritative build semantics remain [12 — Build System](12-build-system.md)
and [Build Tool Conformance](build-tool-conformance.md). In particular, each
BUILD line executes in a fresh shell and a `BUILD_windows` override is a
standalone recipe rather than a patch over `BUILD`.

## Why this audit exists

`uv venv .venv` refuses to replace an existing environment unless the recipe
uses `--clear`. A front without that option can succeed only in a clean package
directory and then fail before any test command on its next invocation. A
front that also omits `--python` may select a host default older than the
package's declared `requires-python` floor.

The reference repair is `python/heap`: both fronts create `.venv` with
`--no-project --clear --python 3.13`, install through that named environment,
and invoke its interpreter explicitly. Audit findings are compared with that
contract; the audit itself does not modify package recipes.

## Corpus discovery

The reporter MUST inspect Git-visible `code/packages/python/*/BUILD_windows`
files. A package is in the current non-idempotent corpus when all of the
following are true:

1. `BUILD_windows` has exactly one active `uv venv` command targeting `.venv`.
2. That command does not contain `--clear`.
3. The package has a canonical `BUILD` and `pyproject.toml`.

Blank lines and lines whose first non-whitespace character is `#` are not
active commands. Missing or duplicate venv commands, missing companion files,
and an absent or non-string `[project].requires-python` value are hard errors;
the reporter must not silently omit an ambiguous package.

Package names must contain only ASCII letters, digits, `_`, and `-`, beginning
with an alphanumeric character. Every companion must be a regular non-symlink
file whose resolved path remains beneath `code/packages/python`; traversal or
symlink escape is a hard error. Git is invoked without a shell and BUILD
commands are parsed as data, never executed by the reporter.

## Per-front record

The JSON report schema is version 1. Each `fronts` entry is ordered by package
name and contains:

- `package` and the exact `requires_python` declaration;
- ordered repository-local dependency references from each front;
- a `canonical` and `windows` record containing the exact venv command,
  `has_clear`, `has_no_project`, `python_pin`, `test_interpreter`,
  `all_pip_commands_use_named_venv`, and `quoted_editable` fields;
- `local_dependency_symmetric`, which is true only when both fronts install the
  same sibling packages in the same order;
- a sorted `issues` list using the stable values below; and
- `dependency_component`, the sorted weakly connected component within the
  discovered corpus.

Stable issue values are:

- `canonical-missing-clear` and `windows-missing-clear`;
- `canonical-missing-python-pin` and `windows-missing-python-pin`;
- `canonical-missing-no-project` and `windows-missing-no-project`;
- `canonical-implicit-test-interpreter` and
  `windows-implicit-test-interpreter`;
- `canonical-pip-without-named-venv` and
  `windows-pip-without-named-venv`;
- `windows-quoted-editable`; and
- `local-dependency-order-mismatch`.

`test_interpreter` is `explicit-venv` when the pytest command begins with the
platform's `.venv` interpreter, `uv-run` when it begins with `uv run`, and
`other` otherwise. A uv pip command uses the named environment only when its
arguments contain `--python .venv`.

## Aggregate report

The report contains `schema_version`, `python_package_count`, `fronts`, and a
`summary` with:

- `python_package_count`, the Git-visible canonical `BUILD` companions directly
  under `code/packages/python/*` (not nested programs or auxiliary fronts);
- the number of non-idempotent fronts;
- counts missing the Python pin on each platform;
- counts missing `--no-project` on each platform;
- the number of packages with repository-local dependencies;
- the number of dependency components; and
- counts by exact `requires-python` declaration.

JSON keys, package rows, issue lists, dependency lists, and component members
must be deterministic. Markdown is a rendering of the same report and may not
carry additional facts.

## Runtime observation protocol

Static classification does not prove the failure mode. Validation therefore
executes every discovered `BUILD_windows` front twice in one package-scoped,
disposable clean copy of the repository:

1. Start from the audited Git revision with no `.venv` in the package.
2. Run active lines in order using the Windows command executor and stop at the
   first non-zero command, matching the build tool.
3. Run the unchanged front a second time without deleting or replacing
   `.venv`.
4. Record the first and second failing command indexes, process exit codes,
   selected interpreter version, and a payload-bounded diagnostic class.

The receipt records only stable classes such as `requires-python` and
`existing-environment`; it must not commit absolute paths, usernames, complete
tool output, environment variables, or credentials. Disposable directories
must be package-scoped, resolved beneath the validation root before removal,
and excluded from Git.

## Backfill decomposition

The audit closes only when every corpus member has exactly one
dependency-shaped backfill owner recorded initially as `pending` in the parity
state. After the audit merges, an owner may advance through the normal
`pending`, `in-progress`, `pr-open`, and `merged` lifecycle without invalidating
the corpus decomposition. Owners must respect repository-local prerequisite
order. A legacy recipe with a different repair shape may not be hidden inside
a generated-pattern bulk edit. Each backfill must later prove both canonical
and Windows fronts from clean state and on an immediate repeat, then run its
real affected downstream closure.

## Generated-standard repair profile

An owner marked `generated-standard` in the backfill fixture repairs only its
listed package roots and MUST preserve the fixture's dependency order. Each
canonical and Windows front MUST:

1. recreate the package-local environment with
   `uv venv .venv --quiet --no-project --clear --python 3.13`;
2. install every repository-local prerequisite through `uv pip --python
   .venv` before installing the package itself;
3. install the editable package and development tools through that same named
   environment, retaining the Windows `--no-deps` package/tool split where the
   checked-in recipe already needs it; and
4. invoke Ruff lint, Ruff format checking, strict MyPy, and pytest through the
   explicit platform interpreter (`.venv/bin/python` or
   `.venv\Scripts\python.exe`), never ambient `python` or `uv run`. When an
   installed repository-local prerequisite lacks a PEP 561 marker, MyPy MUST
   use `--follow-untyped-imports` so its source is analyzed rather than
   silently ignored.

The repair regression MUST compare the complete active recipe for every owned
front, not search for isolated substrings. Runtime validation MUST run both
platform recipes twice consecutively from clean package copies, confirm the
second run replaces the existing environment, rerun the audit, and prove the
live non-idempotent corpus shrinks by exactly the repaired package set while
the original owner decomposition remains complete.
