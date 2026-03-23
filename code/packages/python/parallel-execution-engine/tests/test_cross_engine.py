"""Cross-engine tests — verify same computation produces same results on all engines.

This is the educational payoff of having multiple engines: you can run the
SAME computation on NVIDIA-style SIMT, AMD-style SIMD, Google-style systolic,
and Apple-style MAC arrays, and verify they all produce the same numerical
results — just with different execution traces, cycle counts, and utilization.
"""

from __future__ import annotations

from clock import Clock
from gpu_core import fmul, halt, limm

from parallel_execution_engine import (
    ExecutionModel,
    MACArrayConfig,
    MACArrayEngine,
    MACOperation,
    MACScheduleEntry,
    SystolicArray,
    SystolicConfig,
    WarpConfig,
    WarpEngine,
    WavefrontConfig,
    WavefrontEngine,
)


class TestCrossEngineScalarMultiply:
    """All engines compute a * b and get the same result."""

    def test_simt_multiply(self) -> None:
        """SIMT: each thread computes 3.0 * 4.0 = 12.0."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()])
        engine.run()
        for t in engine.threads:
            assert t.core.registers.read_float(2) == 12.0

    def test_simd_multiply(self) -> None:
        """SIMD: all lanes compute 3.0 * 4.0 = 12.0."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()])
        engine.run()
        for lane in range(4):
            assert engine.vrf.read(2, lane) == 12.0

    def test_systolic_multiply(self) -> None:
        """Systolic: 1x1 matmul is just a multiply."""
        array = SystolicArray(SystolicConfig(rows=1, cols=1), Clock())
        result = array.run_matmul(
            activations=[[3.0]],
            weights=[[4.0]],
        )
        assert abs(result[0][0] - 12.0) < 0.01

    def test_mac_multiply(self) -> None:
        """MAC: one MAC unit computes 3.0 * 4.0 = 12.0."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=1), Clock())
        engine.load_inputs([3.0])
        engine.load_weights([4.0])
        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.MAC,
                input_indices=[0], weight_indices=[0], output_index=0,
            ),
            MACScheduleEntry(
                cycle=2, operation=MACOperation.REDUCE, output_index=0,
            ),
            MACScheduleEntry(
                cycle=3, operation=MACOperation.STORE_OUTPUT, output_index=0,
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()
        assert abs(engine.read_outputs()[0] - 12.0) < 0.01


class TestCrossEngineDotProduct:
    """All engines compute dot(a, b) = sum(a[i] * b[i])."""

    def test_simt_dot_product(self) -> None:
        """SIMT: each thread multiplies one pair, then manual sum."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([
            fmul(2, 0, 1),  # R2 = R0 * R1
            halt(),
        ])
        a = [1.0, 2.0, 3.0, 4.0]
        b = [5.0, 6.0, 7.0, 8.0]
        for t in range(4):
            engine.set_thread_register(t, 0, a[t])
            engine.set_thread_register(t, 1, b[t])
        engine.run()

        # Each thread has a[t]*b[t], sum manually
        total = sum(
            engine.threads[t].core.registers.read_float(2) for t in range(4)
        )
        # 1*5 + 2*6 + 3*7 + 4*8 = 5+12+21+32 = 70
        assert abs(total - 70.0) < 0.1

    def test_simd_dot_product(self) -> None:
        """SIMD: all lanes multiply in parallel."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([fmul(2, 0, 1), halt()])
        a = [1.0, 2.0, 3.0, 4.0]
        b = [5.0, 6.0, 7.0, 8.0]
        for lane in range(4):
            engine.set_lane_register(lane, 0, a[lane])
            engine.set_lane_register(lane, 1, b[lane])
        engine.run()

        total = sum(engine.vrf.read(2, lane) for lane in range(4))
        assert abs(total - 70.0) < 0.1

    def test_mac_dot_product(self) -> None:
        """MAC: parallel MACs + reduce."""
        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        engine.load_weights([5.0, 6.0, 7.0, 8.0])
        schedule = [
            MACScheduleEntry(
                cycle=1, operation=MACOperation.MAC,
                input_indices=[0, 1, 2, 3], weight_indices=[0, 1, 2, 3],
                output_index=0,
            ),
            MACScheduleEntry(
                cycle=2, operation=MACOperation.REDUCE, output_index=0,
            ),
        ]
        engine.load_schedule(schedule)
        engine.run()
        assert abs(engine.read_outputs()[0] - 70.0) < 0.1


class TestCrossEngineMatmul:
    """Test matrix multiplication across systolic and MAC engines."""

    def test_systolic_matmul_matches_mac(self) -> None:
        """Systolic 2x2 matmul should match MAC 2x2 matmul."""
        A = [[1.0, 2.0], [3.0, 4.0]]
        W = [[5.0, 6.0], [7.0, 8.0]]
        # Expected: [[19, 22], [43, 50]]

        # Systolic
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        systolic_result = array.run_matmul(activations=A, weights=W)

        # MAC (manual 4-element matmul)
        mac = MACArrayEngine(MACArrayConfig(num_macs=2), Clock())

        # C[0][0] = A[0][0]*W[0][0] + A[0][1]*W[1][0] = 1*5+2*7 = 19
        mac.load_inputs([1.0, 2.0])
        mac.load_weights([5.0, 7.0])
        mac.load_schedule([
            MACScheduleEntry(
                cycle=1, operation=MACOperation.MAC,
                input_indices=[0, 1], weight_indices=[0, 1], output_index=0,
            ),
            MACScheduleEntry(
                cycle=2, operation=MACOperation.REDUCE, output_index=0,
            ),
        ])
        mac.run()

        # Both should agree on C[0][0] = 19
        assert abs(systolic_result[0][0] - 19.0) < 0.1
        assert abs(mac.read_outputs()[0] - 19.0) < 0.1


class TestCrossEngineExecutionModels:
    """Verify that each engine reports the correct execution model."""

    def test_all_models(self) -> None:
        warp = WarpEngine(WarpConfig(warp_width=4), Clock())
        wave = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        systolic = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        mac = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())

        assert warp.execution_model == ExecutionModel.SIMT
        assert wave.execution_model == ExecutionModel.SIMD
        assert systolic.execution_model == ExecutionModel.SYSTOLIC
        assert mac.execution_model == ExecutionModel.SCHEDULED_MAC

    def test_all_have_names(self) -> None:
        warp = WarpEngine(WarpConfig(warp_width=4), Clock())
        wave = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        systolic = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        mac = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())

        assert warp.name == "WarpEngine"
        assert wave.name == "WavefrontEngine"
        assert systolic.name == "SystolicArray"
        assert mac.name == "MACArrayEngine"
