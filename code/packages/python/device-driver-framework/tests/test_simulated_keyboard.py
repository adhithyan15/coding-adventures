"""Tests for SimulatedKeyboard."""

from device_driver_framework.device import DeviceType
from device_driver_framework.simulated_keyboard import SimulatedKeyboard


class TestSimulatedKeyboard:
    """Verify SimulatedKeyboard input buffer and read/write behavior."""

    def test_default_configuration(self) -> None:
        """Default keyboard should have correct name, major, minor, irq."""
        kb = SimulatedKeyboard()
        assert kb.name == "keyboard0"
        assert kb.device_type == DeviceType.CHARACTER
        assert kb.major == 2
        assert kb.minor == 0
        assert kb.interrupt_number == 33

    def test_read_empty_buffer(self) -> None:
        """Reading with an empty buffer should return empty bytes."""
        kb = SimulatedKeyboard()
        assert kb.read(10) == b""

    def test_inject_and_read(self) -> None:
        """Injected keystrokes should be readable in FIFO order."""
        kb = SimulatedKeyboard()
        kb.inject_keystrokes(b"Hello")
        data = kb.read(5)
        assert data == b"Hello"

    def test_read_partial(self) -> None:
        """Reading fewer bytes than available should leave the rest."""
        kb = SimulatedKeyboard()
        kb.inject_keystrokes(b"ABCDE")
        first = kb.read(3)
        assert first == b"ABC"
        rest = kb.read(10)
        assert rest == b"DE"

    def test_read_more_than_available(self) -> None:
        """Requesting more bytes than available should return what's there."""
        kb = SimulatedKeyboard()
        kb.inject_keystrokes(b"Hi")
        data = kb.read(100)
        assert data == b"Hi"

    def test_write_returns_negative_one(self) -> None:
        """Writing to a keyboard should return -1 (not supported)."""
        kb = SimulatedKeyboard()
        assert kb.write(b"test") == -1

    def test_init_clears_buffer(self) -> None:
        """init() should clear any buffered keystrokes."""
        kb = SimulatedKeyboard()
        kb.inject_keystrokes(b"leftover")
        kb.init()
        assert kb.initialized is True
        assert kb.read(10) == b""

    def test_buffer_size(self) -> None:
        """buffer_size should reflect the number of buffered keystrokes."""
        kb = SimulatedKeyboard()
        assert kb.buffer_size == 0
        kb.inject_keystrokes(b"ABC")
        assert kb.buffer_size == 3
        kb.read(1)
        assert kb.buffer_size == 2

    def test_multiple_injections(self) -> None:
        """Multiple inject calls should accumulate in the buffer."""
        kb = SimulatedKeyboard()
        kb.inject_keystrokes(b"AB")
        kb.inject_keystrokes(b"CD")
        assert kb.read(4) == b"ABCD"

    def test_custom_name_and_minor(self) -> None:
        """Custom name and minor should be respected."""
        kb = SimulatedKeyboard(name="keyboard1", minor=1)
        assert kb.name == "keyboard1"
        assert kb.minor == 1
