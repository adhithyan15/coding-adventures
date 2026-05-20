"""
Type stubs for coding_adventures_matrix_rust_python.

Mirrors the surface that the `matrix_rust_python` C extension exposes,
so IDEs and `mypy` can type-check calls into the extension without
parsing the cdylib.

Keep these in sync with:
- code/packages/rust/matrix-rust-python/src/lib.rs  (module-level fns)
- code/packages/rust/matrix-rust-python/src/classes.rs  (Graph, Runtime)
"""

__version__: str
__all__: list[str]


def graph_round_trip_json(json_string: str) -> str:
    """
    Decode a matrix-ir-json wire-format ``Graph`` and re-encode it.

    Returns the re-encoded JSON string (compact form, canonical field
    order).  Raises :class:`ValueError` on malformed or schema-invalid
    JSON.

    This is the smoke function that proves the matrix-ir-json wire
    format survives a trip through the Python C API boundary.
    """


def run_graph_on_cpu(envelope_json: str) -> str:
    """
    Plan and execute a Graph on the CPU executor.

    Envelope shape::

        in : {
                "graph":  <matrix-ir-json schema>,
                "inputs": ["<lowercase-hex bytes>", ...]
              }
        out: {
                "outputs": ["<lowercase-hex bytes>", ...]
              }

    Returns the result envelope JSON.  Raises :class:`ValueError` on
    malformed JSON, missing fields, invalid hex, planner errors,
    executor errors, or graphs whose total placed-tensor byte size
    exceeds the 4 GiB cap.
    """


class Graph:
    """
    Wraps a parsed ``matrix_ir::Graph`` for repeated execution.

    Constructed from a JSON string in the matrix-ir-json wire format.
    Parsing happens once at construction; the parsed graph is held
    inside the instance via a boxed pointer (freed on garbage
    collection).
    """

    def __init__(self, json_string: str) -> None: ...

    def to_json(self) -> str:
        """Re-serialise the wrapped Graph as matrix-ir-json wire format."""

    def describe(self) -> str:
        """
        Return a short human-readable summary, e.g.
        ``"Graph(tensors=4, ops=3, inputs=1, outputs=1, constants=2)"``.
        """


class Runtime:
    """
    Stateless wrapper around the CPU executor.

    In v0 each :meth:`run` call internally builds a fresh
    ``matrix_runtime::Runtime`` + ``CpuExecutor``.  The class exists
    so the API surface matches MX09's eventual shape (once
    Runtime-level options like an executor pool or GPU backends
    land, the Python API won't have to change).
    """

    def __init__(self) -> None: ...

    def run(self, graph: Graph, inputs: list[bytes]) -> list[bytes]:
        """
        Plan and execute ``graph`` with ``inputs`` as the per-input
        little-endian byte payloads.  Returns one ``bytes`` object
        per ``graph.outputs()`` tensor.

        Raises :class:`ValueError` on planner/executor errors,
        :class:`TypeError` on wrong argument types, :class:`ValueError`
        on graphs exceeding the 4 GiB total-buffer cap.
        """
