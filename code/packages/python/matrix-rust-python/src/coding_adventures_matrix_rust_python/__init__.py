"""
coding_adventures_matrix_rust_python — Python wrapper for the Rust matrix
execution layer.

This package is a thin re-export of the `matrix_rust_python` C extension
shipped by ``code/packages/rust/matrix-rust-python/``.  It provides:

- A stable Python-side import path (``coding_adventures_matrix_rust_python``)
  so consumers (e.g. future ``ml-framework-*`` Python packages) aren't
  coupled to the C extension's internal module name.
- Type hints (via the bundled ``__init__.pyi`` stub) so IDEs and
  ``mypy`` can typecheck calls into the extension.
- A precise ``ImportError`` when the underlying C extension isn't
  installed (rather than a generic ``ModuleNotFoundError`` from the
  Python import machinery, which doesn't say *what* you need to do).

The C extension is built by the Rust crate; today it's not on PyPI
(see MX09 Phase 5 for the publish plan).  Until then, install it
manually before importing this package.

Public API
----------

- :class:`Graph` — wraps a ``matrix_ir::Graph`` parsed from JSON
- :class:`Runtime` — owns the CPU executor; ``Runtime.run(graph, inputs)``
- :func:`graph_round_trip_json` — JSON → Graph → JSON smoke
- :func:`run_graph_on_cpu` — JSON envelope one-shot execution

Example
-------

.. code-block:: python

    import coding_adventures_matrix_rust_python as m
    import json

    graph = m.Graph(json.dumps({...}))     # parse once
    print(graph.describe())                # "Graph(tensors=4, ops=3, ...)"

    rt = m.Runtime()
    outputs = rt.run(graph, [b"...", b"..."])
    # outputs is list[bytes] of little-endian f32 payloads
"""

# Re-export the C extension's public surface.  We use a try/except
# around the import so we can replace Python's default
# ``ModuleNotFoundError: No module named 'matrix_rust_python'`` with
# a precise error message that points at the install instructions.
try:
    from matrix_rust_python import (  # type: ignore[import-not-found]
        Graph,
        Runtime,
        graph_round_trip_json,
        run_graph_on_cpu,
    )
except ImportError as e:
    # The error has a chained cause (the original ModuleNotFoundError)
    # so debugging tools that walk __cause__ still see the root.
    raise ImportError(
        "coding_adventures_matrix_rust_python: the underlying "
        "'matrix_rust_python' C extension is not installed.\n"
        "\n"
        "Build it from the Rust crate at code/packages/rust/matrix-rust-python/:\n"
        "  cargo build -p matrix-rust-python --release\n"
        "  cp target/release/libmatrix_rust_python.{so,dylib} \\\n"
        "     $(python -c 'import sysconfig; print(sysconfig.get_paths()[\"purelib\"])')/matrix_rust_python.so\n"
        "\n"
        "Once MX09 Phase 5 lands a PyPI publish workflow, this will\n"
        "become `pip install matrix-rust-python` instead.\n"
    ) from e


__all__ = [
    "Graph",
    "Runtime",
    "graph_round_trip_json",
    "run_graph_on_cpu",
]

# Version-string convenience for consumers that want to log it.
# Bumps in lockstep with the C extension's Cargo.toml version when
# the surface area changes; pure-doc updates of this wrapper get a
# patch bump that doesn't necessarily match the extension.
__version__ = "0.1.0"
