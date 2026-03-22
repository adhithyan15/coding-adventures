"""SimulatedKeyboard -- a character device representing a keyboard.

==========================================================================
How Keyboards Work
==========================================================================

When you press a key on a physical keyboard, the following happens:

  1. The keyboard controller detects the key press.
  2. It generates a "scan code" (a number identifying which key was pressed).
  3. It raises interrupt 33 (IRQ 1 on the original IBM PC).
  4. The CPU pauses what it's doing and runs the keyboard ISR
     (Interrupt Service Routine).
  5. The ISR reads the scan code from the keyboard controller's I/O port.
  6. The ISR translates the scan code to an ASCII character.
  7. The ISR deposits the character into the keyboard buffer.
  8. The CPU resumes what it was doing.

Later, when a program calls sys_read on the keyboard device, the kernel
calls keyboard.read(), which pops characters from this buffer.

==========================================================================
SimulatedKeyboard Design
==========================================================================

Our simulated keyboard skips steps 1-3 (no physical hardware). Instead,
test code or the simulated BIOS directly pushes characters into the buffer
using inject_keystrokes(). The rest of the pipeline (reading from the
buffer) works identically to a real keyboard.

The keyboard is a CHARACTER device because it produces a stream of bytes
(one keystroke at a time) with no random access. You cannot "seek" to
keystroke #47 -- keystrokes arrive in the order they are pressed.

The keyboard is READ-ONLY: writing to it returns -1. (You cannot force
a key to be pressed by writing bytes to the keyboard!)
"""

from collections import deque

from device_driver_framework.device import CharacterDevice


class SimulatedKeyboard(CharacterDevice):
    """A simulated keyboard backed by an in-memory keystroke buffer.

    Keystrokes are injected into the buffer (simulating ISR deposits)
    and read out by the kernel via the read() method.

    Args:
        name: Device name (default "keyboard0").
        minor: Minor number (default 0).
        interrupt_number: IRQ for key press (default 33).
    """

    def __init__(
        self,
        name: str = "keyboard0",
        minor: int = 0,
        interrupt_number: int = 33,
    ) -> None:
        super().__init__(
            name=name,
            major=2,  # Major 2 = keyboard driver (from the spec)
            minor=minor,
            interrupt_number=interrupt_number,
        )
        # The keystroke buffer: a FIFO queue of individual bytes.
        # In a real system, the keyboard ISR pushes scan codes here
        # whenever interrupt 33 fires.
        self._buffer: deque[int] = deque()

    def init(self) -> None:
        """Initialize the keyboard by clearing the input buffer.

        On a real keyboard, initialization might also involve:
        - Setting the keyboard controller to scan code set 2
        - Enabling the keyboard interrupt on the PIC
        - Turning on the keyboard LEDs (Num Lock, Caps Lock, etc.)
        """
        self._buffer.clear()
        self.initialized = True

    def inject_keystrokes(self, data: bytes) -> None:
        """Simulate keystrokes by pushing bytes into the input buffer.

        This replaces the physical keyboard + ISR pipeline. In our
        simulation, test code or the BIOS calls this method to simulate
        key presses.

        Args:
            data: The bytes to inject (each byte = one keystroke).
        """
        for byte in data:
            self._buffer.append(byte)

    def read(self, count: int) -> bytes:
        """Read up to `count` keystrokes from the buffer.

        Returns whatever is available, up to `count` bytes. If the buffer
        is empty, returns b"" (no data available, non-blocking).

        This is how the kernel gets keyboard input:
          1. User presses keys -> ISR fills buffer (via inject_keystrokes)
          2. Program calls sys_read -> kernel calls keyboard.read()
          3. read() pops from buffer and returns the keystrokes

        Args:
            count: Maximum number of bytes to read.

        Returns:
            A bytes object with 0 to `count` bytes.
        """
        result = bytearray()
        while len(result) < count and self._buffer:
            result.append(self._buffer.popleft())
        return bytes(result)

    def write(self, data: bytes) -> int:
        """Attempt to write to the keyboard (always fails).

        You cannot write to a keyboard -- it is an input-only device.
        This method always returns -1 to signal an error.

        Why not raise an exception? Because in Unix, write() returns -1
        and sets errno to EINVAL for unsupported operations. We follow
        the same convention for consistency with real operating systems.
        """
        return -1

    @property
    def buffer_size(self) -> int:
        """Return the number of keystrokes currently in the buffer."""
        return len(self._buffer)
