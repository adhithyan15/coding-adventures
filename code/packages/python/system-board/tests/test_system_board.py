"""Tests for the S06 System Board -- the critical integration test."""

from system_board import (
    BootPhase,
    DefaultSystemConfig,
    SystemBoard,
)


# =========================================================================
# THE CRITICAL TEST: Boot to Hello World
# =========================================================================


class TestBootToHelloWorld:
    def test_hello_world_on_display(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        snap = board.display_snapshot()
        assert snap is not None
        assert snap.contains("Hello World")

    def test_idle_after_hello_world(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        assert board.is_idle()


# =========================================================================
# Power-On Tests
# =========================================================================


class TestPowerOn:
    def test_new_board(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        assert not board.powered

    def test_power_on(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        assert board.powered
        assert board.cpu is not None
        assert board.display is not None
        assert board.interrupt_ctrl is not None
        assert board.kernel is not None
        assert board.disk_image is not None

    def test_double_power_on(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.power_on()
        assert board.powered


# =========================================================================
# Phase Transition Tests
# =========================================================================


class TestPhases:
    def test_boot_phases(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        phases = board.trace.phases()
        assert len(phases) > 0
        assert BootPhase.POWER_ON in phases
        assert BootPhase.BIOS in phases

    def test_boot_phase_order(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        phases = board.trace.phases()
        for i in range(1, len(phases)):
            prev = board.trace.phase_start_cycle(phases[i - 1])
            curr = board.trace.phase_start_cycle(phases[i])
            assert curr >= prev

    def test_boot_phase_string(self) -> None:
        assert str(BootPhase.POWER_ON) == "PowerOn"
        assert str(BootPhase.BIOS) == "BIOS"
        assert str(BootPhase.BOOTLOADER) == "Bootloader"
        assert str(BootPhase.KERNEL_INIT) == "KernelInit"
        assert str(BootPhase.USER_PROGRAM) == "UserProgram"
        assert str(BootPhase.IDLE) == "Idle"


# =========================================================================
# Boot Trace Tests
# =========================================================================


class TestBootTrace:
    def test_has_events(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        assert len(board.trace.events) > 0

    def test_events_have_descriptions(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        for e in board.trace.events:
            assert e.description != ""

    def test_total_cycles(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        assert board.trace.total_cycles() > 0

    def test_phase_start_cycle(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        assert board.trace.phase_start_cycle(BootPhase.POWER_ON) == 0
        # Invalid phase returns -1 (use a valid enum member that won't appear in trace)
        assert board.trace.phase_start_cycle(None) == -1  # type: ignore[arg-type]

    def test_events_in_phase(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        events = board.trace.events_in_phase(BootPhase.POWER_ON)
        assert len(events) > 0


# =========================================================================
# Display Tests
# =========================================================================


class TestDisplay:
    def test_display_after_boot(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        snap = board.display_snapshot()
        assert snap is not None
        assert snap.contains("Hello World")


# =========================================================================
# Keystroke Tests
# =========================================================================


class TestKeystroke:
    def test_inject_keystroke(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        board.inject_keystroke(ord("A"))
        assert len(board.kernel.keyboard_buffer) == 1
        assert board.kernel.keyboard_buffer[0] == ord("A")


# =========================================================================
# Cycle Count Tests
# =========================================================================


class TestCycleCount:
    def test_positive_cycles(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        assert board.get_cycle_count() > 0

    def test_budget_respected(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(100000)
        assert board.get_cycle_count() <= 100000


# =========================================================================
# Error Handling Tests
# =========================================================================


class TestErrorHandling:
    def test_step_before_power_on(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.step()
        assert board.cycle == 0

    def test_run_before_power_on(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        trace = board.run(100)
        assert trace is not None

    def test_run_zero_cycles(self) -> None:
        board = SystemBoard(DefaultSystemConfig())
        board.power_on()
        board.run(0)
        assert board.get_cycle_count() == 0


# =========================================================================
# Config Tests
# =========================================================================


class TestConfig:
    def test_default_config(self) -> None:
        config = DefaultSystemConfig()
        assert config.memory_size == 1024 * 1024
        assert config.display_config.columns == 80
        assert config.display_config.rows == 25
