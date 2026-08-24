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

An earlier repeatability repair does not satisfy this profile if its active
fronts still invoke non-strict MyPy. The separately owned Hash Functions
correction MUST add `--strict` to both existing named-interpreter commands and
their complete-recipe expectations. It MUST preserve every other active line,
package dependency, capability declaration, and runtime behavior. Validation
MUST prove strict checking on the package's complete `src` and `tests` trees,
then repeat the Windows front twice and exercise the hash-dependent closure.

The RESP/TCP owner MUST repair `resp-protocol` before `tcp-server` and preserve
the fixture's current RESP prerequisite while the separately owned dependency-
contract reconciliation remains pending. TCP's strict MyPy command MUST use
`--follow-untyped-imports` because the installed RESP package has no PEP 561
marker. Dormant lint, formatting, and type-check findings MAY receive bounded,
behavior-preserving corrections within the two owned roots. Because TCP's
baseline coverage clears its 95% gate by fewer than two executable lines, the
repair MUST add focused tests for both cleanup-exception paths instead of
relying on that fragile margin or lowering the threshold. It MUST NOT change
runtime dependency metadata, capability declarations, protocol semantics, or
the DT23/DT24 behavior boundary.

## Data-store quoted-editable repair profile

The `windows-quoted-editable` owner MUST repair
`in-memory-data-store-engine` before `in-memory-data-store`. Both packages MUST
adopt the generated named-environment guarantees above while preserving their
leaf-to-root repository-local dependency order:

1. the engine installs `hash-functions`, then `hyperloglog` and
   `in-memory-data-store-protocol`, then itself; and
2. the composed store installs `hash-functions`, then `hyperloglog`,
   `in-memory-data-store-protocol`, and `resp-protocol`, then
   `in-memory-data-store-engine`, then itself.

The Windows fronts MUST replace the invalid quoted editable requirement
`".[dev]"` with the unquoted `.[dev]` token and retain the existing
`--no-deps` editable-package plus explicit development-tool split. Canonical
fronts MUST install their own unquoted `.[dev]` requirement through the named
environment. Every strict MyPy command MUST use `--follow-untyped-imports`
because the installed `hash-functions` package, and the composed store's
installed `resp-protocol` package, lack PEP 561 markers.

Dormant type-check findings MAY receive only bounded, behavior-preserving
cleanup: discriminate each scalar `EngineResponse` kind before narrowing its
value, and prove decoded AOF frame and element shapes before typed test use.
The composed store's broad `ignore_missing_imports` configuration MUST be
removed once the complete local closure is installed and followed. Existing
95% coverage thresholds MUST remain unchanged. The repair MUST NOT change
runtime dependency metadata, versions, capability manifests, RESP semantics,
data-store behavior, or AOF filesystem authority. The separately registered
filesystem-authority review remains selection-blocked and outside this repair.

The regression MUST compare all four complete recipes exactly. Windows
runtime validation MUST run both fronts twice consecutively from clean
package-local copies with uv 0.11.28 and Python 3.13, then prove the audit
shrinks only by these two packages. Validation MUST also build distributable
artifacts, exercise the store's direct dependent closure, and run the Go build
tool's dry plan and real affected closure.

## Graph dependency-chain repair profile

The `python-graph-build-front-idempotence` owner MUST repair `graph` before
`directed-graph`. The graph package is the typed DT00 leaf and
`directed-graph` is its DT01 dependent; no other package root belongs to this
owner. Both packages MUST adopt the generated named-environment guarantees
above while preserving this exact leaf-to-root installation order:

1. `graph` creates its environment and installs only its own `.[dev]` editable
   requirement; and
2. `directed-graph` installs `../graph` before its own `.[dev]` editable
   requirement.

The Windows fronts MUST retain the existing `--no-deps` editable-package plus
explicit development-tool split. Every front MUST invoke Ruff lint, Ruff
format checking, strict MyPy, and pytest through its explicit package-local
interpreter. MyPy MUST analyze the complete `src` and `tests` trees without
`--follow-untyped-imports`: the installed graph prerequisite publishes its
`py.typed` marker and is therefore part of the typed dependency contract.

Dormant lint, formatting, and type-check findings MAY receive only bounded,
behavior-preserving cleanup within these two roots. Such cleanup MAY modernize
generic syntax to the declared Python 3.12 floor, make iterative DFS stack
frames explicitly iterable for strict typing, add missing test-local type
annotations, and sort imports or exports. It MUST NOT change graph algorithms,
public behavior, package metadata, runtime dependencies, versions, capability
manifests, or the DT00/DT01 boundary. Existing 95% coverage thresholds and both
packages' PEP 561 markers MUST remain unchanged.

The regression MUST compare all four complete recipes exactly. Windows runtime
validation MUST execute both fronts twice consecutively from clean
package-local copies with uv 0.11.28 and Python 3.13, then prove the live audit
shrinks only by `graph` and `directed-graph`. Validation MUST also build both
distributable artifacts, exercise a representative selection from the direct
dependent closure, and run the Go build tool's dry plan and real affected
closure.

## Legacy named-environment repair profile

The separately owned `python/in-memory-data-store-protocol` front was not in
the uv audit corpus because its Windows recipe creates a standard-library venv.
It is nevertheless an affected-plan prerequisite and MUST converge on the same
named-environment guarantees. Both fronts MUST:

1. create package-local `.venv` with
   `uv venv .venv --quiet --no-project --clear --python 3.13`;
2. install the editable package through `uv pip --python .venv`, using the
   Windows no-dependency package/tool split so resolution cannot escape into a
   workspace or an unavailable registry dependency;
3. invoke Ruff lint, Ruff format checking, strict MyPy, and pytest through the
   explicit platform interpreter; and
4. retain the package's short-traceback pytest contract.

No active line may invoke ambient `python`, a pip console launcher, or `uv run`.
The regression MUST compare both complete recipes exactly. Windows runtime
validation MUST execute every active line twice consecutively, verify Python
3.13, and prove the second run clears and replaces the first environment. The
canonical recipe MUST run on Unix CI and remain structurally identical apart
from platform interpreter paths and the Windows install split.
