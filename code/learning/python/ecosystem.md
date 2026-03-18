# The Modern Python Ecosystem (2025-2026)

Python's tooling has undergone a revolution. The old world of `setup.py`, `requirements.txt`, `pip`, `virtualenv`, `flake8`, `black`, `isort` — all separate tools with separate configs — is being replaced by faster, unified alternatives.

## uv — The Package/Project Manager

**What it replaces:** pip, pip-tools, pipenv, poetry, pyenv, virtualenv

**What it is:** A single Rust-based tool that handles everything:
- Installing Python versions (`uv python install 3.12`)
- Creating virtual environments (`uv venv`)
- Installing packages (`uv pip install requests`)
- Managing project dependencies (`uv add requests`)
- Running scripts (`uv run hello.py`)
- Building and publishing packages (`uv build`, `uv publish`)

**Why it exists:** pip is slow. poetry is slow. pipenv is slow. All of them are written in Python, which means they bootstrap slowly and resolve dependencies slowly. uv is written in Rust and is 10-100x faster.

**Key commands:**
```bash
uv init                  # Create a new project (generates pyproject.toml)
uv add requests          # Add a dependency
uv add --dev pytest      # Add a dev dependency
uv run python script.py  # Run a script in the project's environment
uv build                 # Build a distributable package (.whl, .tar.gz)
uv publish               # Publish to PyPI
uv sync                  # Install all dependencies from pyproject.toml
```

**How it manages environments:** uv automatically creates a `.venv` directory in your project. When you `uv run`, it uses that environment. You never manually activate/deactivate.

## ruff — The Linter and Formatter

**What it replaces:** flake8, black, isort, pyflakes, pycodestyle, pydocstyle, bandit, and dozens more

**What it is:** A single Rust-based tool that:
- Lints your code (finds bugs, style issues, security problems)
- Formats your code (consistent style, like black)
- Sorts imports (like isort)

**Why it exists:** flake8 is slow. Running flake8 + black + isort means three separate tools, three configs, three passes over your code. ruff does all of it in one pass, 100x faster.

**Key commands:**
```bash
ruff check .             # Lint all files
ruff check --fix .       # Lint and auto-fix what it can
ruff format .            # Format all files (like black)
```

**Configuration** lives in `pyproject.toml`:
```toml
[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = [
    "E",    # pycodestyle errors
    "W",    # pycodestyle warnings
    "F",    # pyflakes
    "I",    # isort (import sorting)
    "UP",   # pyupgrade (use modern Python syntax)
    "B",    # bugbear (common bugs)
    "SIM",  # simplify (suggest simpler code)
    "ANN",  # annotations (type hint enforcement)
]
```

## pyproject.toml — The One Config File

**What it replaces:** setup.py, setup.cfg, requirements.txt, MANIFEST.in, .flake8, pytest.ini, .isort.cfg, .coveragerc

**What it is:** A single TOML file that configures everything:
- Build system (how to build the package)
- Project metadata (name, version, description, dependencies)
- Tool configuration (ruff, pytest, coverage, mypy)

**Why it exists:** Python projects used to have 5-10 config files scattered around. PEP 517/518/621 standardized on pyproject.toml as the one source of truth.

**Structure:**
```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "my-package"
version = "0.1.0"
description = "What this package does"
requires-python = ">=3.12"
license = "MIT"
authors = [{ name = "Your Name", email = "you@example.com" }]
dependencies = []

[project.optional-dependencies]
dev = ["pytest", "pytest-cov", "ruff", "mypy"]

[tool.ruff]
# ruff config here

[tool.pytest.ini_options]
# pytest config here

[tool.coverage.run]
# coverage config here
```

## Type Hints

**What they are:** Annotations that describe what types a function expects and returns.

```python
# Without type hints
def add(a, b):
    return a + b

# With type hints
def add(a: int, b: int) -> int:
    return a + b
```

**Key facts:**
- Added in Python 3.5 (PEP 484), improved significantly through 3.12
- NOT enforced at runtime — Python ignores them when executing
- Caught by type checkers: mypy, pyright, pytype
- IDEs use them for autocomplete and error detection
- They make code self-documenting

**Modern syntax (Python 3.10+):**
```python
# Old way
from typing import Optional, Union, List, Dict

def process(items: List[str], default: Optional[int] = None) -> Union[str, int]:
    ...

# New way (3.10+) — use built-in types directly
def process(items: list[str], default: int | None = None) -> str | int:
    ...
```

**py.typed marker:** A file named `py.typed` in your package signals to type checkers that your package includes type annotations (PEP 561).

## hatchling — The Build Backend

**What it replaces:** setuptools (the old default)

**What it is:** A modern, standards-compliant build backend. When you run `uv build` or `pip install .`, hatchling is what actually creates the distributable package.

**Why hatchling over setuptools:** setuptools carries decades of legacy. hatchling is simpler, faster, and follows modern standards (PEP 517/518) without legacy baggage. It also supports the `src` layout natively.

## The src Layout

**What it is:** Putting your package code in `src/package_name/` instead of directly in `package_name/`.

```
# Flat layout (old way)         # src layout (modern way)
my_package/                     src/
├── __init__.py                     my_package/
├── module.py                           __init__.py
tests/                                  module.py
├── test_module.py              tests/
pyproject.toml                      test_module.py
                                pyproject.toml
```

**Why it matters:** With the flat layout, `import my_package` might import from your local directory instead of the installed version. This masks bugs — your tests pass locally but the published package is broken. The `src` layout forces you to install the package before importing it, catching these issues early.

## Devbox — The Environment Manager

**What it replaces:** Docker (for dev environments), manual tool installation

**What it is:** A Nix-based tool that provides isolated, reproducible development environments. Instead of installing Python, uv, and ruff globally, you declare them in a `devbox.json`:

```json
{
  "packages": ["python312", "uv", "ruff"]
}
```

Run `devbox shell` and you have exactly those tools available — no global installs, no version conflicts, same environment on every machine.

**Why not Docker?** Docker wraps everything in a container with filesystem overhead, networking complexity, and slow volume mounts on macOS. Devbox uses Nix under the hood to provide isolated tools natively — no container, no VM, no overhead.

## How they all fit together

```
devbox.json          → Provides Python, uv, ruff (environment)
pyproject.toml       → Defines the project, dependencies, tool config (configuration)
uv                   → Manages dependencies and virtual environment (package management)
ruff                 → Lints and formats the code (code quality)
pytest + coverage    → Runs tests and measures coverage (testing)
mypy                 → Checks type hints (type safety)
hatchling            → Builds the package for PyPI (distribution)
```

All configured in one file (pyproject.toml), all provided by one environment (devbox.json).
