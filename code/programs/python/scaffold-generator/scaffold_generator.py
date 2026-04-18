"""scaffold-generator — Generate CI-ready package scaffolding.

=== What This Program Does ===

This tool generates correctly-structured, CI-ready package directories for
the coding-adventures monorepo. It supports all six languages: Python, Go,
Ruby, TypeScript, Rust, and Elixir.

=== Why This Tool Exists ===

The lessons.md file documents 12+ recurring categories of CI failures caused
by agents hand-crafting packages inconsistently:

  - Missing BUILD files
  - TypeScript "main" pointing to dist/ instead of src/
  - Missing transitive dependency installs in BUILD files
  - Ruby require ordering (deps before own modules)
  - Rust workspace Cargo.toml not updated

This tool eliminates those failures. Run it, get a package that compiles,
lints, and passes tests. Then fill in the business logic.

=== How CLI Builder Powers This ===

The entire CLI interface is defined in ``scaffold-generator.json``. This
program never parses a single argument by hand. CLI Builder handles all
parsing, validation, and help generation.
"""

from __future__ import annotations

import json
import os
import re
import sys
from datetime import date, timezone
from pathlib import Path

SPEC_FILE = str(Path(__file__).parent.parent.parent / "scaffold-generator.json")

VALID_LANGUAGES = ["python", "go", "ruby", "typescript", "rust", "elixir", "perl", "lua", "swift", "haskell"]
KEBAB_RE = re.compile(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$")


# =========================================================================
# Name normalization
# =========================================================================

def to_snake_case(kebab: str) -> str:
    """Convert 'my-package' to 'my_package'."""
    return kebab.replace("-", "_")


def to_camel_case(kebab: str) -> str:
    """Convert 'my-package' to 'MyPackage'."""
    return "".join(part.capitalize() for part in kebab.split("-"))


def to_joined_lower(kebab: str) -> str:
    """Convert 'my-package' to 'mypackage' (Go package name)."""
    return kebab.replace("-", "")


def dir_name(kebab: str, lang: str) -> str:
    """Return the directory name for a package in a given language."""
    if lang in ("ruby", "elixir"):
        return to_snake_case(kebab)
    return kebab


# =========================================================================
# Dependency resolution
# =========================================================================

def read_deps(pkg_dir: str, lang: str) -> list[str]:
    """Read direct local dependencies of a package from its metadata files."""
    readers = {
        "python": _read_python_deps,
        "go": _read_go_deps,
        "ruby": _read_ruby_deps,
        "typescript": _read_ts_deps,
        "rust": _read_rust_deps,
        "elixir": _read_elixir_deps,
        "perl": _read_perl_deps,
        "lua": _read_lua_deps,
        "swift": _read_swift_deps,
        "haskell": _read_haskell_deps,
    }
    return readers[lang](pkg_dir)


def _read_python_deps(pkg_dir: str) -> list[str]:
    """Parse BUILD file for -e ../ entries."""
    build_path = os.path.join(pkg_dir, "BUILD")
    if not os.path.exists(build_path):
        return []
    deps = []
    with open(build_path) as f:
        for line in f:
            # Find ALL -e ../ entries on each line (new format puts them all on one line)
            remaining = line
            while True:
                idx = remaining.find("-e ../")
                if idx < 0:
                    idx = remaining.find('-e "../')
                if idx < 0:
                    break
                rest = remaining[idx:]
                if rest.startswith('-e "../'):
                    rest = rest[7:]  # skip `-e "../`
                else:
                    rest = rest[6:]  # skip `-e ../`
                dep = ""
                for c in rest:
                    if c in (' ', '"', "'", '\n'):
                        break
                    dep += c
                if dep and dep != ".":
                    deps.append(dep)
                remaining = remaining[idx + 6:]
    return deps


def _read_go_deps(pkg_dir: str) -> list[str]:
    """Parse go.mod replace directives for ../dep paths."""
    mod_path = os.path.join(pkg_dir, "go.mod")
    if not os.path.exists(mod_path):
        return []
    deps = []
    with open(mod_path) as f:
        for line in f:
            if "=> ../" in line:
                idx = line.index("=> ../")
                rest = line[idx + 6:].strip()
                dep = rest.split()[0] if rest.split() else ""
                if dep:
                    deps.append(dep)
    return deps


def _read_ruby_deps(pkg_dir: str) -> list[str]:
    """Parse Gemfile for path: '../dep' entries."""
    gemfile_path = os.path.join(pkg_dir, "Gemfile")
    if not os.path.exists(gemfile_path):
        return []
    deps = []
    with open(gemfile_path) as f:
        for line in f:
            if 'path:' in line and '"../' in line:
                idx = line.index('"../')
                rest = line[idx + 4:]  # skip past "../
                dep = ""
                for c in rest:
                    if c == '"':
                        break
                    dep += c
                dep = dep.replace("_", "-")
                if dep:
                    deps.append(dep)
    return deps


def _read_ts_deps(pkg_dir: str) -> list[str]:
    """Parse package.json dependencies with file:../ values."""
    pkg_json_path = os.path.join(pkg_dir, "package.json")
    if not os.path.exists(pkg_json_path):
        return []
    with open(pkg_json_path) as f:
        try:
            pkg = json.load(f)
        except json.JSONDecodeError:
            return []
    deps_obj = pkg.get("dependencies", {})
    deps = []
    for val in deps_obj.values():
        if isinstance(val, str) and val.startswith("file:../"):
            dep = val.removeprefix("file:../")
            if dep:
                deps.append(dep)
    return deps


def _read_rust_deps(pkg_dir: str) -> list[str]:
    """Parse Cargo.toml for path = '../dep' entries."""
    cargo_path = os.path.join(pkg_dir, "Cargo.toml")
    if not os.path.exists(cargo_path):
        return []
    deps = []
    with open(cargo_path) as f:
        for line in f:
            if 'path = "../' in line:
                idx = line.index('path = "../')
                rest = line[idx + 11:]
                dep = ""
                for c in rest:
                    if c == '"':
                        break
                    dep += c
                if dep:
                    deps.append(dep)
    return deps


def _read_elixir_deps(pkg_dir: str) -> list[str]:
    """Parse mix.exs for path: '../dep' entries."""
    mix_path = os.path.join(pkg_dir, "mix.exs")
    if not os.path.exists(mix_path):
        return []
    deps = []
    with open(mix_path) as f:
        for line in f:
            if 'path: "../' in line:
                idx = line.index('path: "../')
                rest = line[idx + 10:]
                dep = ""
                for c in rest:
                    if c == '"':
                        break
                    dep += c
                dep = dep.replace("_", "-")
                if dep:
                    deps.append(dep)
    return deps


def _read_perl_deps(pkg_dir: str) -> list[str]:
    """Parse cpanfile for requires 'coding-adventures-X' entries."""
    import re
    cpanfile_path = os.path.join(pkg_dir, "cpanfile")
    if not os.path.exists(cpanfile_path):
        return []
    deps = []
    pattern = re.compile(r"requires\s+['\"]coding-adventures-([^'\"]+)['\"]")
    with open(cpanfile_path) as f:
        for line in f:
            m = pattern.search(line)
            if m:
                deps.append(m.group(1))
    return deps


def _read_lua_deps(pkg_dir: str) -> list[str]:
    # Stub for lua
    return []


def _read_swift_deps(pkg_dir: str) -> list[str]:
    # Stub for swift
    return []


def _read_haskell_deps(pkg_dir: str) -> list[str]:
    """Parse cabal file for dependencies entries."""
    import glob
    cabal_files = glob.glob(os.path.join(pkg_dir, "*.cabal"))
    if not cabal_files:
        return []
    cabal_path = cabal_files[0]
    deps = []
    import re
    pattern = re.compile(r"coding-adventures-([a-zA-Z0-9-]+)")
    self_name = os.path.basename(pkg_dir)
    with open(cabal_path) as f:
        for line in f:
            m = pattern.search(line)
            if m:
                if "name:" in line or "executable" in line or "library" in line or m.group(1) == self_name:
                    continue
                deps.append(m.group(1))
    return deps


def transitive_closure(
    direct_deps: list[str], lang: str, base_dir: str,
) -> list[str]:
    """Compute all transitive dependencies via BFS."""
    visited: set[str] = set()
    queue = list(direct_deps)

    while queue:
        dep = queue.pop(0)
        if dep in visited:
            continue
        visited.add(dep)
        dep_dir = os.path.join(base_dir, dir_name(dep, lang))
        for dd in read_deps(dep_dir, lang):
            if dd not in visited:
                queue.append(dd)

    return sorted(visited)


def topological_sort(
    all_deps: list[str], lang: str, base_dir: str,
) -> list[str]:
    """Return dependencies in leaf-first (install) order via Kahn's algorithm."""
    dep_set = set(all_deps)

    # Build graph: for each dep, list what it depends on (within our set)
    graph: dict[str, list[str]] = {dep: [] for dep in all_deps}
    for dep in all_deps:
        dep_dir = os.path.join(base_dir, dir_name(dep, lang))
        for dd in read_deps(dep_dir, lang):
            if dd in dep_set:
                graph[dep].append(dd)

    # In-degree: how many deps does this node have within the set
    in_degree = {dep: len(graph[dep]) for dep in all_deps}

    # Start with leaves (in_degree == 0)
    queue = sorted(dep for dep in all_deps if in_degree[dep] == 0)
    result: list[str] = []

    while queue:
        node = queue.pop(0)
        result.append(node)
        # Decrease in-degree for nodes that depend on this one
        for dep in all_deps:
            if node in graph[dep]:
                in_degree[dep] -= 1
                if in_degree[dep] == 0:
                    queue.append(dep)
                    queue.sort()

    if len(result) != len(all_deps):
        msg = f"circular dependency detected: resolved {len(result)} of {len(all_deps)}"
        raise ValueError(msg)

    return result


# =========================================================================
# File generation
# =========================================================================

def _write_file(path: str, content: str) -> None:
    """Write content to a file, creating parent directories as needed."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(content)


def generate_python(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], ordered_deps: list[str],
) -> None:
    """Generate Python package files."""
    snake = to_snake_case(pkg_name)

    pyproject = f"""[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "coding-adventures-{pkg_name}"
version = "0.1.0"
description = "{description}"
requires-python = ">=3.12"
license = "MIT"
authors = [{{ name = "Adhithya Rajasekaran" }}]
readme = "README.md"

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

[tool.hatch.build.targets.wheel]
packages = ["src/{snake}"]

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--cov={snake} --cov-report=term-missing --cov-fail-under=80"

[tool.coverage.run]
source = ["src/{snake}"]

[tool.coverage.report]
fail_under = 80
show_missing = true
"""

    init_py = f'''"""{pkg_name} — {description}

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
{layer_ctx}"""

__version__ = "0.1.0"
'''

    test_py = f'''"""Tests for {pkg_name}."""

from {snake} import __version__


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
'''

    install_parts = ["python -m pip install"]
    for dep in ordered_deps:
        install_parts.append(f"-e ../{dep}")
    install_parts.extend(["-e .[dev]", "--quiet"])
    build_lines = [" ".join(install_parts)]
    build_lines.append("python -m pytest tests/ -v")
    build = "\n".join(build_lines) + "\n"

    # ── BUILD_windows: uses uv instead of pip, single-line dep installs ──
    build_win_lines = ["uv venv --quiet --clear"]
    if ordered_deps:
        dep_flags = " ".join(f"-e ../{dep}" for dep in ordered_deps)
        build_win_lines.append(f"uv pip install {dep_flags} --quiet")
    build_win_lines.append("uv pip install --no-deps -e .[dev] --quiet")
    build_win_lines.append("uv pip install pytest pytest-cov ruff mypy --quiet")
    build_win_lines.append("uv run --no-project python -m pytest tests/ -v")
    build_windows = "\n".join(build_win_lines) + "\n"

    _write_file(os.path.join(target_dir, "pyproject.toml"), pyproject)
    _write_file(os.path.join(target_dir, "src", snake, "__init__.py"), init_py)
    _write_file(os.path.join(target_dir, "tests", "__init__.py"), "")
    _write_file(os.path.join(target_dir, "tests", f"test_{snake}.py"), test_py)
    _write_file(os.path.join(target_dir, "BUILD"), build)
    _write_file(os.path.join(target_dir, "BUILD_windows"), build_windows)


def generate_go(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], all_deps: list[str],
) -> None:
    """Generate Go package files."""
    go_pkg = to_joined_lower(pkg_name)
    snake = to_snake_case(pkg_name)

    go_mod = f"module github.com/adhithyan15/coding-adventures/code/packages/go/{pkg_name}\n\ngo 1.26\n"
    if direct_deps:
        go_mod += "\nrequire (\n"
        for dep in direct_deps:
            go_mod += f"\tgithub.com/adhithyan15/coding-adventures/code/packages/go/{dep} v0.0.0\n"
        go_mod += ")\n\nreplace (\n"
        for dep in all_deps:
            go_mod += f"\tgithub.com/adhithyan15/coding-adventures/code/packages/go/{dep} => ../{dep}\n"
        go_mod += ")\n"

    src = f"""// Package {go_pkg} provides {description}.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
// {layer_ctx}
package {go_pkg}
"""

    test = f"""package {go_pkg}

import "testing"

func TestPackageLoads(t *testing.T) {{
\tt.Log("{pkg_name} package loaded successfully")
}}
"""

    _write_file(os.path.join(target_dir, "go.mod"), go_mod)
    _write_file(os.path.join(target_dir, f"{snake}.go"), src)
    _write_file(os.path.join(target_dir, f"{snake}_test.go"), test)
    _write_file(os.path.join(target_dir, "BUILD"), "go test ./... -v -cover\n")


def generate_ruby(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], all_deps: list[str],
) -> None:
    """Generate Ruby package files."""
    snake = to_snake_case(pkg_name)
    camel = to_camel_case(pkg_name)

    gemspec = f'''# frozen_string_literal: true

require_relative "lib/coding_adventures/{snake}/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_{snake}"
  spec.version       = CodingAdventures::{camel}::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "{description}"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {{
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }}

'''
    for dep in direct_deps:
        dep_snake = to_snake_case(dep)
        gemspec += f'  spec.add_dependency "coding_adventures_{dep_snake}", "~> 0.1"\n'
    gemspec += '  spec.add_development_dependency "minitest", "~> 5.0"\n'
    gemspec += '  spec.add_development_dependency "rake", "~> 13.0"\nend\n'

    gemfile = '# frozen_string_literal: true\n\nsource "https://rubygems.org"\ngemspec\n'
    if all_deps:
        gemfile += "\n# All transitive path dependencies.\n"
        for dep in all_deps:
            dep_snake = to_snake_case(dep)
            gemfile += f'gem "coding_adventures_{dep_snake}", path: "../{dep_snake}"\n'

    rakefile = '''# frozen_string_literal: true

require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: :test
'''

    entry = "# frozen_string_literal: true\n\n"
    if direct_deps:
        entry += "# IMPORTANT: Require dependencies FIRST, before own modules.\n"
        for dep in direct_deps:
            dep_snake = to_snake_case(dep)
            entry += f'require "coding_adventures_{dep_snake}"\n'
        entry += "\n"
    entry += f'require_relative "coding_adventures/{snake}/version"\n\n'
    entry += f"module CodingAdventures\n  # {description}\n  module {camel}\n  end\nend\n"

    version_rb = f"""# frozen_string_literal: true

module CodingAdventures
  module {camel}
    VERSION = "0.1.0"
  end
end
"""

    test_rb = f"""# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_{snake}"

class Test{camel} < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::{camel}::VERSION
  end
end
"""

    _write_file(os.path.join(target_dir, f"coding_adventures_{snake}.gemspec"), gemspec)
    _write_file(os.path.join(target_dir, "Gemfile"), gemfile)
    _write_file(os.path.join(target_dir, "Rakefile"), rakefile)
    _write_file(os.path.join(target_dir, "lib", f"coding_adventures_{snake}.rb"), entry)
    _write_file(os.path.join(target_dir, "lib", "coding_adventures", snake, "version.rb"), version_rb)
    _write_file(os.path.join(target_dir, "test", f"test_{snake}.rb"), test_rb)
    _write_file(os.path.join(target_dir, "BUILD"), "bundle install --quiet\nbundle exec rake test\n")


def generate_typescript(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], ordered_deps: list[str],
) -> None:
    """Generate TypeScript package files."""
    deps_json = ""
    if direct_deps:
        entries = [f'    "@coding-adventures/{dep}": "file:../{dep}"' for dep in direct_deps]
        deps_json = ",\n".join(entries)

    package_json = f"""{{"name": "@coding-adventures/{pkg_name}",
  "version": "0.1.0",
  "description": "{description}",
  "type": "module",
  "main": "src/index.ts",
  "scripts": {{
    "build": "tsc",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage"
  }},
  "author": "Adhithya Rajasekaran",
  "license": "MIT",
  "dependencies": {{
{deps_json}
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "vitest": "^3.0.0",
    "@vitest/coverage-v8": "^3.0.0"
  }}
}}
"""
    # Fix the JSON formatting (the opening brace)
    package_json = '{\n  "name": "@coding-adventures/' + pkg_name + '",\n'
    package_json += f'  "version": "0.1.0",\n'
    package_json += f'  "description": "{description}",\n'
    package_json += '  "type": "module",\n'
    package_json += '  "main": "src/index.ts",\n'
    package_json += '  "scripts": {\n'
    package_json += '    "build": "tsc",\n'
    package_json += '    "test": "vitest run",\n'
    package_json += '    "test:coverage": "vitest run --coverage"\n'
    package_json += '  },\n'
    package_json += '  "author": "Adhithya Rajasekaran",\n'
    package_json += '  "license": "MIT",\n'
    package_json += '  "dependencies": {\n'
    if direct_deps:
        entries = [f'    "@coding-adventures/{dep}": "file:../{dep}"' for dep in direct_deps]
        package_json += ",\n".join(entries) + "\n"
    package_json += '  },\n'
    package_json += '  "devDependencies": {\n'
    package_json += '    "typescript": "^5.0.0",\n'
    package_json += '    "vitest": "^3.0.0",\n'
    package_json += '    "@vitest/coverage-v8": "^3.0.0"\n'
    package_json += '  }\n'
    package_json += '}\n'

    tsconfig = """{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true
  },
  "include": ["src"]
}
"""

    vitest_config = """import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
"""

    index_ts = f"""/**
 * @coding-adventures/{pkg_name}
 *
 * {description}
 *
 * This package is part of the coding-adventures monorepo.
 * {layer_ctx}
 */

export const VERSION = "0.1.0";
"""

    test_ts = f"""import {{ describe, it, expect }} from "vitest";
import {{ VERSION }} from "../src/index.js";

describe("{pkg_name}", () => {{
  it("has a version", () => {{
    expect(VERSION).toBe("0.1.0");
  }});
}});
"""

    build = "npm ci --quiet\nnpx vitest run --coverage\n"

    _write_file(os.path.join(target_dir, "package.json"), package_json)
    _write_file(os.path.join(target_dir, "tsconfig.json"), tsconfig)
    _write_file(os.path.join(target_dir, "vitest.config.ts"), vitest_config)
    _write_file(os.path.join(target_dir, "src", "index.ts"), index_ts)
    _write_file(os.path.join(target_dir, "tests", f"{pkg_name}.test.ts"), test_ts)
    _write_file(os.path.join(target_dir, "BUILD"), build)


def generate_rust(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str],
) -> None:
    """Generate Rust package files."""
    cargo = f"""[package]
name = "{pkg_name}"
version = "0.1.0"
edition = "2021"
description = "{description}"

[dependencies]
"""
    for dep in direct_deps:
        cargo += f'{dep} = {{ path = "../{dep}" }}\n'

    lib_rs = f"""//! # {pkg_name}
//!
//! {description}
//!
//! This crate is part of the coding-adventures monorepo.
//! {layer_ctx}

#[cfg(test)]
mod tests {{
    #[test]
    fn it_loads() {{
        assert!(true, "{pkg_name} crate loaded successfully");
    }}
}}
"""

    _write_file(os.path.join(target_dir, "Cargo.toml"), cargo)
    _write_file(os.path.join(target_dir, "src", "lib.rs"), lib_rs)
    _write_file(os.path.join(target_dir, "BUILD"), f"cargo test -p {pkg_name} -- --nocapture\n")


def generate_elixir(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], ordered_deps: list[str],
) -> None:
    """Generate Elixir package files."""
    snake = to_snake_case(pkg_name)
    camel = to_camel_case(pkg_name)

    deps_str = ""
    for i, dep in enumerate(direct_deps):
        dep_snake = to_snake_case(dep)
        comma = "," if i < len(direct_deps) - 1 else ""
        deps_str += f"      {{:coding_adventures_{dep_snake}, path: \"../{dep_snake}\"}}{comma}\n"

    mix_exs = f"""defmodule CodingAdventures.{camel}.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_{snake},
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
{deps_str}    ]
  end
end
"""

    lib_ex = f"""defmodule CodingAdventures.{camel} do
  @moduledoc \"\"\"
  {description}

  This module is part of the coding-adventures monorepo.
  {layer_ctx}
  \"\"\"
end
"""

    test_exs = f"""defmodule CodingAdventures.{camel}Test do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.{camel})
  end
end
"""

    if ordered_deps:
        parts = [f"cd ../{to_snake_case(dep)} && mix deps.get --quiet && mix compile --quiet" for dep in ordered_deps]
        parts.append(f"cd ../{snake} && mix deps.get --quiet && mix test --cover")
        build = " && \\\n".join(parts) + "\n"
    else:
        build = "mix deps.get --quiet && mix test --cover\n"

    _write_file(os.path.join(target_dir, "mix.exs"), mix_exs)
    _write_file(os.path.join(target_dir, "lib", "coding_adventures", f"{snake}.ex"), lib_ex)
    _write_file(os.path.join(target_dir, "test", f"{snake}_test.exs"), test_exs)
    _write_file(os.path.join(target_dir, "test", "test_helper.exs"), "ExUnit.start()\n")
    _write_file(os.path.join(target_dir, "BUILD"), build)


def generate_perl(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], ordered_deps: list[str],
) -> None:
    """Generate Perl package files."""
    camel = to_camel_case(pkg_name)

    # Makefile.PL
    prereq_lines = "".join(
        f"        'CodingAdventures::{to_camel_case(dep)}' => 0,\n"
        for dep in direct_deps
    )
    makefile_pl = (
        "use strict;\nuse warnings;\nuse ExtUtils::MakeMaker;\n\nWriteMakefile(\n"
        f"    NAME             => 'CodingAdventures::{camel}',\n"
        f"    VERSION_FROM     => 'lib/CodingAdventures/{camel}.pm',\n"
        f"    ABSTRACT         => '{description}',\n"
        "    AUTHOR           => 'coding-adventures',\n"
        "    LICENSE          => 'mit',\n"
        "    MIN_PERL_VERSION => '5.026000',\n"
        "    PREREQ_PM        => {\n"
        f"{prereq_lines}"
        "    },\n"
        "    TEST_REQUIRES    => {\n        'Test2::V0' => 0,\n    },\n"
        "    META_MERGE       => {\n        'meta-spec' => { version => 2 },\n"
        "        resources   => {\n            repository => {\n"
        "                type => 'git',\n"
        "                url  => 'https://github.com/adhithyan15/coding-adventures.git',\n"
        "                web  => 'https://github.com/adhithyan15/coding-adventures',\n"
        "            },\n        },\n    },\n);\n"
    )

    # cpanfile
    runtime_lines = "".join(
        f"requires 'coding-adventures-{dep}';\n" for dep in direct_deps
    )
    cpanfile = (
        ("# Runtime dependencies\n" + runtime_lines + "\n" if direct_deps else "")
        + "# Test dependencies\non 'test' => sub {\n    requires 'Test2::V0';\n};\n"
    )

    # Source module
    layer_line = f"#\n# {layer_ctx}\n" if layer_ctx else ""
    dep_imports = "".join(
        f"use CodingAdventures::{to_camel_case(dep)};\n" for dep in direct_deps
    )
    module = f"""package CodingAdventures::{camel};

# ============================================================================
# CodingAdventures::{camel} — {description}
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#{layer_line}#
# Usage:
#
#   use CodingAdventures::{camel};
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

{dep_imports}
# TODO: Implement {camel}

1;

__END__

=head1 NAME

CodingAdventures::{camel} - {description}

=head1 SYNOPSIS

    use CodingAdventures::{camel};

=head1 DESCRIPTION

{description}

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
"""

    # t/00-load.t
    load_t = f"""use strict;
use warnings;
use Test2::V0;

use_ok('CodingAdventures::{camel}');

# Verify the module exports a version number.
ok(CodingAdventures::{camel}->VERSION, 'has a VERSION');

done_testing;
"""

    # t/01-basic.t
    basic_t = f"""use strict;
use warnings;
use Test2::V0;

use CodingAdventures::{camel};

# TODO: Replace this placeholder with real tests.
ok(1, '{camel} module loaded successfully');

done_testing;
"""

    # BUILD
    build_lines = [
        f"cd ../{dep} && cpanm --with-test --installdeps --quiet .\n"
        for dep in ordered_deps
    ]
    build_lines.append("cpanm --with-test --installdeps --quiet .\n")
    build_lines.append("prove -l -v t/\n")
    build = "".join(build_lines)

    os.makedirs(os.path.join(target_dir, "lib", "CodingAdventures"), exist_ok=True)
    os.makedirs(os.path.join(target_dir, "t"), exist_ok=True)

    _write_file(os.path.join(target_dir, "Makefile.PL"), makefile_pl)
    _write_file(os.path.join(target_dir, "cpanfile"), cpanfile)
    _write_file(os.path.join(target_dir, "lib", "CodingAdventures", f"{camel}.pm"), module)
    _write_file(os.path.join(target_dir, "t", "00-load.t"), load_t)
    _write_file(os.path.join(target_dir, "t", "01-basic.t"), basic_t)
    _write_file(os.path.join(target_dir, "BUILD"), build)


def generate_haskell(
    target_dir: str, pkg_name: str, description: str,
    layer_ctx: str, direct_deps: list[str], ordered_deps: list[str],
) -> None:
    """Generate Haskell package files."""
    pkg_name_haskell = f"coding-adventures-{pkg_name}"
    module_name = to_camel_case(pkg_name)

    cabal = f"""cabal-version: 3.0
name:          {pkg_name_haskell}
version:       0.1.0
synopsis:      {description}
license:       MIT
author:        Adhithya Rajasekaran
maintainer:    Adhithya Rajasekaran
build-type:    Simple

library
    exposed-modules:  {module_name}
    build-depends:    base >=4.14
"""
    for dep in ordered_deps:
        cabal += f"                      , coding-adventures-{dep}\n"
    cabal += f"""    hs-source-dirs:   src
    default-language: Haskell2010

test-suite spec
    type:             exitcode-stdio-1.0
    main-is:          Spec.hs
    build-depends:    base >=4.14
                    , {pkg_name_haskell}
"""
    for dep in ordered_deps:
        cabal += f"                    , coding-adventures-{dep}\n"
    cabal += """    hs-source-dirs:   test
    default-language: Haskell2010
"""

    lib_hs = f"""module {module_name} where

-- | {description}
-- {layer_ctx}
someFunc :: IO ()
someFunc = putStrLn "someFunc"
"""

    spec_hs = f"""import {module_name}

main :: IO ()
main = do
    putStrLn "Test suite not yet implemented."
"""

    cabal_project = "packages: .\n"
    for dep in ordered_deps:
        cabal_project += f"          ../{dep}\n"

    build = "cabal test all\n"

    os.makedirs(os.path.join(target_dir, "src"), exist_ok=True)
    os.makedirs(os.path.join(target_dir, "test"), exist_ok=True)

    _write_file(os.path.join(target_dir, f"{pkg_name_haskell}.cabal"), cabal)
    _write_file(os.path.join(target_dir, "cabal.project"), cabal_project)
    _write_file(os.path.join(target_dir, "src", f"{module_name}.hs"), lib_hs)
    _write_file(os.path.join(target_dir, "test", "Spec.hs"), spec_hs)
    _write_file(os.path.join(target_dir, "BUILD"), build)


def generate_common_files(
    target_dir: str, pkg_name: str, description: str,
    lang: str, layer: int, direct_deps: list[str],
) -> None:
    """Generate README.md and CHANGELOG.md."""
    today = date.today().isoformat()

    changelog = f"""# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - {today}

### Added

- Initial package scaffolding generated by scaffold-generator
"""

    readme = f"# {pkg_name}\n\n{description}\n"
    if layer > 0:
        readme += f"\n## Layer {layer}\n\nThis package is part of Layer {layer} of the coding-adventures computing stack.\n"
    if direct_deps:
        readme += "\n## Dependencies\n\n"
        for dep in direct_deps:
            readme += f"- {dep}\n"
    readme += "\n## Development\n\n```bash\n# Run tests\nbash BUILD\n```\n"

    _write_file(os.path.join(target_dir, "README.md"), readme)
    _write_file(os.path.join(target_dir, "CHANGELOG.md"), changelog)


def update_rust_workspace(repo_root: str, pkg_name: str) -> bool:
    """Add a crate to the workspace Cargo.toml members list. Returns True on success."""
    workspace_path = os.path.join(repo_root, "code", "packages", "rust", "Cargo.toml")
    if not os.path.exists(workspace_path):
        return False
    with open(workspace_path) as f:
        content = f.read()
    if f'"{pkg_name}"' in content:
        return True  # already listed
    idx = content.find("members = [")
    if idx < 0:
        return False
    close_idx = content.index("]", idx)
    new_entry = f'  "{pkg_name}",\n'
    content = content[:close_idx] + new_entry + content[close_idx:]
    with open(workspace_path, "w") as f:
        f.write(content)
    return True


# =========================================================================
# Main scaffold logic
# =========================================================================

def find_repo_root() -> str:
    """Walk up from cwd to find the git root."""
    d = os.getcwd()
    while True:
        if os.path.exists(os.path.join(d, ".git")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            raise RuntimeError("not inside a git repository")  # noqa: TRY003
        d = parent


def scaffold_one(
    pkg_name: str, pkg_type: str, lang: str,
    direct_deps: list[str], layer: int, description: str,
    dry_run: bool, repo_root: str,
) -> None:
    """Scaffold a package for a single language."""
    base_category = "packages" if pkg_type == "library" else "programs"
    base_dir = os.path.join(repo_root, "code", base_category, lang)
    d_name = dir_name(pkg_name, lang)
    target_dir = os.path.join(base_dir, d_name)

    if os.path.exists(target_dir):
        raise FileExistsError(f"directory already exists: {target_dir}")

    for dep in direct_deps:
        dep_dir = os.path.join(base_dir, dir_name(dep, lang))
        if not os.path.exists(dep_dir):
            msg = f"dependency {dep!r} not found for {lang} at {dep_dir}"
            raise FileNotFoundError(msg)

    all_deps = transitive_closure(direct_deps, lang, base_dir)
    ordered_deps = topological_sort(all_deps, lang, base_dir)

    layer_ctx = f"Layer {layer} in the computing stack." if layer > 0 else ""

    if dry_run:
        print(f"[dry-run] Would create {lang} package at: {target_dir}")
        print(f"  Direct deps: {direct_deps}")
        print(f"  All transitive deps: {all_deps}")
        print(f"  Install order: {ordered_deps}")
        return

    os.makedirs(target_dir, exist_ok=True)

    generators = {
        "python": lambda: generate_python(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps),
        "go": lambda: generate_go(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps),
        "ruby": lambda: generate_ruby(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps),
        "typescript": lambda: generate_typescript(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps),
        "rust": lambda: generate_rust(target_dir, pkg_name, description, layer_ctx, direct_deps),
        "elixir": lambda: generate_elixir(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps),
        "perl": lambda: generate_perl(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps),
        "lua": lambda: None,
        "swift": lambda: None,
        "haskell": lambda: generate_haskell(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps),
    }
    generators[lang]()
    generate_common_files(target_dir, pkg_name, description, lang, layer, direct_deps)

    print(f"Created {lang} package at: {target_dir}")

    if lang == "rust":
        if update_rust_workspace(repo_root, pkg_name):
            print("  Updated code/packages/rust/Cargo.toml workspace members")
        else:
            print(f'  WARNING: Manually add "{pkg_name}" to code/packages/rust/Cargo.toml members', file=sys.stderr)
        print("  Run: cargo build --workspace (to verify)")
    elif lang == "typescript":
        print(f"  Run: cd {target_dir} && npm install (to generate package-lock.json)")
    elif lang == "go":
        print(f"  Run: cd {target_dir} && go mod tidy")


def main() -> None:
    """Entry point: parse args via CLI Builder, then scaffold packages."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"scaffold-generator: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    pkg_name = result.arguments.get("package-name", "")
    pkg_type = result.flags.get("type", "library") or "library"
    lang_str = result.flags.get("language", "all") or "all"
    deps_str = result.flags.get("depends-on", "") or ""
    layer_val = result.flags.get("layer", 0) or 0
    description = result.flags.get("description", "") or ""
    dry_run = bool(result.flags.get("dry-run", False))

    if not KEBAB_RE.match(pkg_name):
        print(f"scaffold-generator: invalid package name {pkg_name!r} (must be kebab-case)", file=sys.stderr)
        raise SystemExit(1)

    if lang_str == "all":
        languages = list(VALID_LANGUAGES)
    else:
        languages = []
        for lang in lang_str.split(","):
            lang = lang.strip()
            if lang not in VALID_LANGUAGES:
                print(f"scaffold-generator: unknown language {lang!r}", file=sys.stderr)
                raise SystemExit(1)
            languages.append(lang)

    direct_deps = [d.strip() for d in deps_str.split(",") if d.strip()] if deps_str else []
    for dep in direct_deps:
        if not KEBAB_RE.match(dep):
            print(f"scaffold-generator: invalid dependency name {dep!r}", file=sys.stderr)
            raise SystemExit(1)

    repo_root = find_repo_root()
    layer = int(layer_val) if layer_val else 0

    had_error = False
    for lang in languages:
        try:
            scaffold_one(pkg_name, pkg_type, lang, direct_deps, layer, description, dry_run, repo_root)
        except (FileExistsError, FileNotFoundError, ValueError) as exc:
            print(f"scaffold-generator [{lang}]: {exc}", file=sys.stderr)
            had_error = True

    if had_error:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
