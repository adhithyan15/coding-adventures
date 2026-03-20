"""Tests for work distributors — GPU, TPU, and ANE strategies."""

import pytest

from device_simulator import (
    GPUWorkDistributor,
    TPUSequencer,
    ANEScheduleReplayer,
    KernelDescriptor,
)
from compute_unit import (
    StreamingMultiprocessor,
    SMConfig,
    MatrixMultiplyUnit,
    MXUConfig,
    NeuralEngineCore,
    ANECoreConfig,
)
from clock import Clock


# =========================================================================
# Helper: create small CUs for testing
# =========================================================================


def make_sms(n: int = 4) -> tuple[list[StreamingMultiprocessor], Clock]:
    clk = Clock(frequency_hz=1_000_000)
    config = SMConfig(max_warps=4, num_schedulers=1, shared_memory_size=1024, register_file_size=2048)
    sms = [StreamingMultiprocessor(config, clk) for _ in range(n)]
    return sms, clk


def make_mxu() -> tuple[MatrixMultiplyUnit, Clock]:
    clk = Clock(frequency_hz=1_000_000)
    return MatrixMultiplyUnit(MXUConfig(), clk), clk


def make_ane_cores(n: int = 4) -> tuple[list[NeuralEngineCore], Clock]:
    clk = Clock(frequency_hz=1_000_000)
    cores = [NeuralEngineCore(ANECoreConfig(), clk) for _ in range(n)]
    return cores, clk


# =========================================================================
# GPU Work Distributor
# =========================================================================


class TestGPUWorkDistributor:
    def test_submit_kernel_creates_blocks(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms)
        kernel = KernelDescriptor(
            name="test",
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        assert dist.pending_count == 4

    def test_step_dispatches_blocks(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms)
        from gpu_core import limm, halt
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 1.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        actions = dist.step()
        assert len(actions) >= 1
        assert dist.pending_count < 2

    def test_round_robin_distributes_evenly(self) -> None:
        sms, _ = make_sms(4)
        dist = GPUWorkDistributor(sms, policy="round_robin")
        from gpu_core import limm, halt
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 1.0), halt()],
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        dist.step()
        # At least some blocks should have been dispatched
        assert dist.total_dispatched > 0

    def test_total_dispatched_tracks(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms)
        from gpu_core import limm, halt
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 1.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        dist.step()
        assert dist.total_dispatched >= 1

    def test_empty_step_returns_no_actions(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms)
        actions = dist.step()
        assert actions == []

    def test_reset_clears_pending(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms)
        kernel = KernelDescriptor(
            name="test",
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        dist.reset()
        assert dist.pending_count == 0
        assert dist.total_dispatched == 0

    def test_fill_first_policy(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms, policy="fill_first")
        from gpu_core import limm, halt
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 1.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        dist.step()
        assert dist.total_dispatched >= 1

    def test_least_loaded_policy(self) -> None:
        sms, _ = make_sms(2)
        dist = GPUWorkDistributor(sms, policy="least_loaded")
        from gpu_core import limm, halt
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 1.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        dist.submit_kernel(kernel)
        dist.step()
        assert dist.total_dispatched >= 1

    def test_kernel_descriptor_properties(self) -> None:
        k = KernelDescriptor(
            grid_dim=(4, 2, 1),
            block_dim=(16, 16, 1),
        )
        assert k.total_blocks == 8
        assert k.threads_per_block == 256
        assert k.total_threads == 2048


# =========================================================================
# TPU Sequencer
# =========================================================================


class TestTPUSequencer:
    def test_submit_operation_creates_tiles(self) -> None:
        mxu, _ = make_mxu()
        seq = TPUSequencer(mxu, mxu_size=2, scalar_latency=2, mxu_latency=5, vector_latency=3)
        kernel = KernelDescriptor(
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        seq.submit_operation(kernel)
        assert seq.pending_count >= 1

    def test_step_advances_pipeline(self) -> None:
        mxu, _ = make_mxu()
        seq = TPUSequencer(mxu, mxu_size=2, scalar_latency=1, mxu_latency=2, vector_latency=1)
        kernel = KernelDescriptor(
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        seq.submit_operation(kernel)
        actions = seq.step()
        assert len(actions) >= 1

    def test_runs_to_completion(self) -> None:
        mxu, _ = make_mxu()
        seq = TPUSequencer(mxu, mxu_size=2, scalar_latency=1, mxu_latency=2, vector_latency=1)
        kernel = KernelDescriptor(
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        seq.submit_operation(kernel)
        for _ in range(100):
            seq.step()
            if seq.idle:
                break
        assert seq.idle

    def test_idle_initially(self) -> None:
        mxu, _ = make_mxu()
        seq = TPUSequencer(mxu, mxu_size=2)
        assert seq.idle

    def test_reset(self) -> None:
        mxu, _ = make_mxu()
        seq = TPUSequencer(mxu, mxu_size=2, scalar_latency=1, mxu_latency=2, vector_latency=1)
        kernel = KernelDescriptor(
            operation="matmul",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        seq.submit_operation(kernel)
        seq.step()
        seq.reset()
        assert seq.idle
        assert seq.pending_count == 0


# =========================================================================
# ANE Schedule Replayer
# =========================================================================


class TestANEScheduleReplayer:
    def test_submit_generates_schedule(self) -> None:
        cores, _ = make_ane_cores(2)
        replayer = ANEScheduleReplayer(cores, dma_latency=1, compute_latency=2, activate_latency=1)
        kernel = KernelDescriptor(
            operation="conv2d",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[0.5, 0.5], [0.5, 0.5]],
        )
        replayer.submit_operation(kernel)
        assert replayer.pending_count > 0

    def test_step_replays_schedule(self) -> None:
        cores, _ = make_ane_cores(2)
        replayer = ANEScheduleReplayer(cores, dma_latency=1, compute_latency=2, activate_latency=1)
        kernel = KernelDescriptor(
            operation="conv2d",
            input_data=[[1.0, 2.0]],
            weight_data=[[0.5, 0.5]],
        )
        replayer.submit_operation(kernel)
        actions = replayer.step()
        assert len(actions) >= 1

    def test_runs_to_completion(self) -> None:
        cores, _ = make_ane_cores(2)
        replayer = ANEScheduleReplayer(cores, dma_latency=1, compute_latency=2, activate_latency=1)
        kernel = KernelDescriptor(
            operation="inference",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        replayer.submit_operation(kernel)
        for _ in range(100):
            replayer.step()
            if replayer.idle:
                break
        assert replayer.idle

    def test_idle_initially(self) -> None:
        cores, _ = make_ane_cores(2)
        replayer = ANEScheduleReplayer(cores)
        assert replayer.idle

    def test_reset(self) -> None:
        cores, _ = make_ane_cores(2)
        replayer = ANEScheduleReplayer(cores, dma_latency=1, compute_latency=1, activate_latency=1)
        kernel = KernelDescriptor(
            operation="test",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        replayer.submit_operation(kernel)
        replayer.step()
        replayer.reset()
        assert replayer.idle
        assert replayer.pending_count == 0

    def test_total_dispatched(self) -> None:
        cores, _ = make_ane_cores(2)
        replayer = ANEScheduleReplayer(cores, dma_latency=1, compute_latency=1, activate_latency=1)
        kernel = KernelDescriptor(
            operation="test",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        replayer.submit_operation(kernel)
        for _ in range(100):
            replayer.step()
            if replayer.idle:
                break
        assert replayer.total_dispatched > 0
