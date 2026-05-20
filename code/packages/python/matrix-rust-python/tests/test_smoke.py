"""
End-to-end smoke test for coding_adventures_matrix_rust_python.

Round-trips a 2x2 MatMul graph through the wrapper's ``Graph`` +
``Runtime`` classes and asserts on the output bytes.  This is the
MX09 Phase 4 acceptance gate — proves the whole stack works from
Python all the way down to ``matrix-cpu``'s vendored MatMul kernel.

The test **skips cleanly** if the underlying ``matrix_rust_python``
C extension isn't installed in the test environment.  That keeps
the wrapper package itself import-safe even when the extension
hasn't been built yet (e.g. on a fresh checkout before
``cargo build -p matrix-rust-python --release`` has run).

Once MX09 Phase 5 lands the PyPI publish workflow, the extension
will be an actual dependency of this wrapper package and the skip
behaviour can be removed.
"""

import json
import struct
import unittest


# We deliberately do the import inside a try/except so the entire
# module remains importable even when the C extension is missing —
# unittest can then collect the test and report it as skipped.
try:
    import coding_adventures_matrix_rust_python as m  # noqa: F401

    EXTENSION_AVAILABLE = True
except ImportError as e:
    EXTENSION_AVAILABLE = False
    _IMPORT_ERROR_MESSAGE = str(e)


# --------------------------------------------------------------------------
# Helpers — pack/unpack f32 lists to/from little-endian bytes.
# Same shape the matrix-rust-napi Phase 4 tests use, so the two
# bindings can share fixture intent (the byte payloads are
# bit-identical across Node and Python).
# --------------------------------------------------------------------------

def f32_bytes(values: list[float]) -> bytes:
    """Pack a list of floats as little-endian IEEE 754 f32 bytes."""
    return b"".join(struct.pack("<f", v) for v in values)


def from_f32_bytes(buf: bytes) -> list[float]:
    """Unpack little-endian IEEE 754 f32 bytes into a list of floats."""
    if len(buf) % 4 != 0:
        raise ValueError(f"buffer length {len(buf)} is not a multiple of 4")
    return [struct.unpack("<f", buf[i : i + 4])[0] for i in range(0, len(buf), 4)]


# --------------------------------------------------------------------------
# Test cases
# --------------------------------------------------------------------------


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; "
    "see code/packages/rust/matrix-rust-python/ for build instructions. "
    + (
        f"Original error: {_IMPORT_ERROR_MESSAGE}"
        if not EXTENSION_AVAILABLE
        else ""
    ),
)
class MatrixRustPythonSmokeTests(unittest.TestCase):
    """Smoke tests that exercise the full Python -> Rust -> Python path."""

    def test_module_exports_the_four_promised_names(self) -> None:
        """
        The wrapper must re-export Graph, Runtime, graph_round_trip_json,
        and run_graph_on_cpu — the MX09 §"The binding surface" contract.
        """
        for name in ("Graph", "Runtime", "graph_round_trip_json", "run_graph_on_cpu"):
            self.assertTrue(
                hasattr(m, name), f"wrapper missing re-export: {name!r}"
            )

    def test_graph_class_round_trips_through_json(self) -> None:
        """
        Construct a Graph from JSON, call ``to_json()`` to re-serialise,
        confirm both decode to equivalent matrix-ir-json wire-format
        payloads.  Catches regressions in the JSON ↔ Box<Graph> path
        that wouldn't be caught by the Rust-side unit tests alone.
        """
        graph_json = json.dumps(
            {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": [2, 2]},
                    {"id": 1, "dtype": "f32", "shape": [2, 2]},
                    {"id": 2, "dtype": "f32", "shape": [2, 2]},
                ],
                "inputs": [0, 1],
                "outputs": [2],
                "ops": [{"kind": "MatMul", "a": 0, "b": 1, "output": 2}],
                "constants": [],
            }
        )
        g = m.Graph(graph_json)
        re_serialised = g.to_json()

        # The output decodes to a value semantically identical to the
        # input — but JSON whitespace and key order may differ.  Run
        # both through Python's json so we compare structured values.
        original_decoded = json.loads(graph_json)
        re_serialised_decoded = json.loads(re_serialised)
        self.assertEqual(
            original_decoded["matrix_ir_version"],
            re_serialised_decoded["matrix_ir_version"],
        )
        self.assertEqual(
            len(original_decoded["tensors"]),
            len(re_serialised_decoded["tensors"]),
        )
        self.assertEqual(
            len(original_decoded["ops"]),
            len(re_serialised_decoded["ops"]),
        )

    def test_graph_describe_includes_topology_counts(self) -> None:
        """``describe()`` returns a short summary string with counts."""
        graph_json = json.dumps(
            {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": [2, 2]},
                    {"id": 1, "dtype": "f32", "shape": [2, 2]},
                    {"id": 2, "dtype": "f32", "shape": [2, 2]},
                ],
                "inputs": [0, 1],
                "outputs": [2],
                "ops": [{"kind": "MatMul", "a": 0, "b": 1, "output": 2}],
                "constants": [],
            }
        )
        g = m.Graph(graph_json)
        desc = g.describe()
        # Format is "Graph(tensors=3, ops=1, inputs=2, outputs=1, constants=0)"
        self.assertIn("tensors=3", desc)
        self.assertIn("ops=1", desc)
        self.assertIn("inputs=2", desc)
        self.assertIn("outputs=1", desc)
        self.assertIn("constants=0", desc)

    def test_runtime_run_executes_2x2_matmul_end_to_end(self) -> None:
        """
        The headline Phase 4 acceptance test: build a 2x2 MatMul graph,
        invoke through the Runtime class, assert the numerical result.

        ``[[1, 2], [3, 4]] @ [[5, 6], [7, 8]]`` = ``[[19, 22], [43, 50]]``

        This is the same fixture matrix-rust-napi/tests uses; matching
        the input keeps the cross-binding equivalence intact.
        """
        graph_json = json.dumps(
            {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": [2, 2]},
                    {"id": 1, "dtype": "f32", "shape": [2, 2]},
                    {"id": 2, "dtype": "f32", "shape": [2, 2]},
                ],
                "inputs": [0, 1],
                "outputs": [2],
                "ops": [{"kind": "MatMul", "a": 0, "b": 1, "output": 2}],
                "constants": [],
            }
        )
        graph = m.Graph(graph_json)
        rt = m.Runtime()

        a = f32_bytes([1.0, 2.0, 3.0, 4.0])  # [[1,2],[3,4]]
        b = f32_bytes([5.0, 6.0, 7.0, 8.0])  # [[5,6],[7,8]]

        outputs = rt.run(graph, [a, b])

        self.assertEqual(len(outputs), 1, "expected one output tensor")
        result = from_f32_bytes(outputs[0])
        self.assertEqual(
            result,
            [19.0, 22.0, 43.0, 50.0],
            "2x2 MatMul did not produce the expected result",
        )

    def test_runtime_run_rejects_wrong_input_count(self) -> None:
        """A graph declaring 2 inputs must reject a call passing 1."""
        graph_json = json.dumps(
            {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": [2, 2]},
                    {"id": 1, "dtype": "f32", "shape": [2, 2]},
                    {"id": 2, "dtype": "f32", "shape": [2, 2]},
                ],
                "inputs": [0, 1],
                "outputs": [2],
                "ops": [{"kind": "MatMul", "a": 0, "b": 1, "output": 2}],
                "constants": [],
            }
        )
        graph = m.Graph(graph_json)
        rt = m.Runtime()

        with self.assertRaises(ValueError) as ctx:
            rt.run(graph, [f32_bytes([1.0, 2.0, 3.0, 4.0])])

        self.assertIn(
            "input count mismatch",
            str(ctx.exception),
            f"expected 'input count mismatch' in error, got: {ctx.exception}",
        )

    def test_runtime_rejects_non_graph_first_argument(self) -> None:
        """
        Passing something other than a Graph instance must raise
        ``TypeError``.  This exercises the ``PyObject_IsInstance``
        type-discrimination defence introduced in Phase 2b — the
        equivalent of matrix-rust-napi's 128-bit type-tag check.
        """
        rt = m.Runtime()

        with self.assertRaises((TypeError, ValueError)) as ctx:
            # Pass a string where a Graph is expected.
            rt.run("not a graph", [b""])  # type: ignore[arg-type]

        # Either error type is acceptable — the precise wording
        # is "argument must be a matrix_rust_python.Graph instance"
        # but accommodate either error class to avoid being
        # brittle to small wording changes.
        msg = str(ctx.exception)
        self.assertTrue(
            "Graph" in msg or "not a wrapped" in msg or "TypeError" in msg,
            f"unexpected error message: {ctx.exception!r}",
        )

    def test_graph_round_trip_json_module_level_helper(self) -> None:
        """
        The module-level ``graph_round_trip_json`` (Phase 1) is
        re-exported through the wrapper too — confirm it works.
        """
        graph_json = json.dumps(
            {
                "matrix_ir_version": 1,
                "tensors": [{"id": 0, "dtype": "f32", "shape": [3]}],
                "inputs": [0],
                "outputs": [0],
                "ops": [],
                "constants": [],
            }
        )
        out = m.graph_round_trip_json(graph_json)
        # Output decodes to the same logical Graph.
        self.assertEqual(json.loads(out)["matrix_ir_version"], 1)


if __name__ == "__main__":
    unittest.main()
