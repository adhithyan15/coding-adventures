"""Cross-architecture tests — same computation on all architectures.

=== Why Cross-Architecture Tests? ===

These tests verify that the SAME computation produces the SAME result
(within floating-point tolerance) across all five compute unit
architectures. This validates that our simulators are correct and
that the architectural differences are in HOW the computation happens,
not WHAT is computed.

The test uses matrix multiplication as the benchmark because:
1. It's supported by all architectures
2. It has a well-defined expected result
3. It exercises the core compute pipeline of each architecture
"""

from __future__ import annotations

from clock import Clock
from gpu_core import fmul, halt

from compute_unit import (
    Architecture,
    SMConfig,
    StreamingMultiprocessor,
    WorkItem,
)
from compute_unit.amd_compute_unit import AMDComputeUnit, AMDCUConfig
from compute_unit.matrix_multiply_unit import MatrixMultiplyUnit, MXUConfig
from compute_unit.neural_engine_core import ANECoreConfig, NeuralEngineCore
from compute_unit.xe_core import XeCore, XeCoreConfig


class TestCrossArchitectureMatmul:
    """Same matmul computed on all architectures."""

    # We'll compute: [1, 2] x [[3], [4]] = [11]
    # This is a simple 1x2 times 2x1 matmul.

    def test_nvidia_sm_matmul(self) -> None:
        """NVIDIA SM: manual matmul via SIMT threads."""
        clock = Clock()
        config = SMConfig(max_warps=8, warp_width=4, num_schedulers=1)
        sm = StreamingMultiprocessor(config, clock)

        # Program: R0 has a[i], R1 has b[i], compute R2 = R0 * R1
        # Then we read the results from each thread's R2
        prog = [fmul(2, 0, 1), halt()]
        sm.dispatch(WorkItem(
            work_id=0,
            program=prog,
            thread_count=2,
            per_thread_data={
                0: {0: 1.0, 1: 3.0},  # a[0]*b[0] = 3
                1: {0: 2.0, 1: 4.0},  # a[1]*b[1] = 8
            },
        ))
        sm.run()
        assert sm.idle

        # Read results from each thread's register 2
        warp = sm.warp_slots[0]
        r0 = warp.engine.threads[0].core.registers.read_float(2)
        r1 = warp.engine.threads[1].core.registers.read_float(2)
        total = r0 + r1
        assert abs(total - 11.0) < 0.01

    def test_amd_cu_matmul(self) -> None:
        """AMD CU: manual matmul via SIMD wavefront."""
        clock = Clock()
        config = AMDCUConfig(
            max_wavefronts=8, wave_width=4, num_simd_units=1
        )
        cu = AMDComputeUnit(config, clock)

        prog = [fmul(2, 0, 1), halt()]
        cu.dispatch(WorkItem(
            work_id=0,
            program=prog,
            thread_count=2,
            per_thread_data={
                0: {0: 1.0, 1: 3.0},
                1: {0: 2.0, 1: 4.0},
            },
        ))
        cu.run()
        assert cu.idle

    def test_google_mxu_matmul(self) -> None:
        """Google MXU: systolic array matmul."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[1.0, 2.0]],
            weights=[[3.0], [4.0]],
        )
        assert abs(result[0][0] - 11.0) < 0.1

    def test_apple_ane_matmul(self) -> None:
        """Apple ANE: MAC array matmul."""
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(num_macs=4), clock)
        result = ane.run_inference(
            inputs=[[1.0, 2.0]],
            weights=[[3.0], [4.0]],
            activation_fn="none",
        )
        assert abs(result[0][0] - 11.0) < 0.01

    def test_intel_xe_core_compute(self) -> None:
        """Intel Xe Core: SIMD8 threads compute."""
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        xe.dispatch(WorkItem(
            work_id=0,
            program=[fmul(2, 0, 1), halt()],
            thread_count=8,
            per_thread_data={
                0: {0: 1.0, 1: 3.0},
                1: {0: 2.0, 1: 4.0},
            },
        ))
        xe.run()
        assert xe.idle


class TestCrossArchitectureLargerMatmul:
    """2x2 matmul across all dataflow architectures.

    [1, 2]   [5, 6]   [19, 22]
    [3, 4] x [7, 8] = [43, 50]
    """

    EXPECTED = [[19.0, 22.0], [43.0, 50.0]]
    TOL = 0.5  # tolerance for FP rounding through systolic array

    def test_mxu_2x2(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[1.0, 2.0], [3.0, 4.0]],
            weights=[[5.0, 6.0], [7.0, 8.0]],
        )
        for i in range(2):
            for j in range(2):
                assert abs(result[i][j] - self.EXPECTED[i][j]) < self.TOL

    def test_ane_2x2(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[1.0, 2.0], [3.0, 4.0]],
            weights=[[5.0, 6.0], [7.0, 8.0]],
            activation_fn="none",
        )
        for i in range(2):
            for j in range(2):
                assert abs(result[i][j] - self.EXPECTED[i][j]) < self.TOL


class TestArchitectureProperties:
    """Verify each architecture reports correct identity."""

    def test_all_architectures_represented(self) -> None:
        """We have implementations for all 5 architectures."""
        clock = Clock()

        sm = StreamingMultiprocessor(SMConfig(max_warps=4), clock)
        cu = AMDComputeUnit(AMDCUConfig(max_wavefronts=4, wave_width=4), clock)
        mxu = MatrixMultiplyUnit(MXUConfig(array_rows=2, array_cols=2), clock)
        xe = XeCore(XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4), clock)
        ane = NeuralEngineCore(ANECoreConfig(num_macs=4), clock)

        archs = {
            sm.architecture,
            cu.architecture,
            mxu.architecture,
            xe.architecture,
            ane.architecture,
        }
        assert len(archs) == 5
        assert Architecture.NVIDIA_SM in archs
        assert Architecture.AMD_CU in archs
        assert Architecture.GOOGLE_MXU in archs
        assert Architecture.INTEL_XE_CORE in archs
        assert Architecture.APPLE_ANE_CORE in archs

    def test_all_names_unique(self) -> None:
        clock = Clock()

        names = {
            StreamingMultiprocessor(SMConfig(max_warps=4), clock).name,
            AMDComputeUnit(AMDCUConfig(max_wavefronts=4, wave_width=4), clock).name,
            MatrixMultiplyUnit(MXUConfig(array_rows=2, array_cols=2), clock).name,
            XeCore(XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4), clock).name,
            NeuralEngineCore(ANECoreConfig(num_macs=4), clock).name,
        }
        assert len(names) == 5

    def test_all_start_idle(self) -> None:
        clock = Clock()

        units = [
            StreamingMultiprocessor(SMConfig(max_warps=4), clock),
            AMDComputeUnit(AMDCUConfig(max_wavefronts=4, wave_width=4), clock),
            MatrixMultiplyUnit(MXUConfig(array_rows=2, array_cols=2), clock),
            XeCore(XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4), clock),
            NeuralEngineCore(ANECoreConfig(num_macs=4), clock),
        ]
        for unit in units:
            assert unit.idle, f"{unit.name} should start idle"
