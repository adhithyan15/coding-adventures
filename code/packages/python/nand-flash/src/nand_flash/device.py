"""A tiny, deterministic model of logical single-level-cell NAND media.

The model has just enough physics to make the database lesson concrete: bits
start at one, a page can be programmed once, and erase is a block operation.
It deliberately leaves timing, ECC, wear leveling, and controller policy out.
"""

from __future__ import annotations

from typing import Final

MAX_BLOCKS: Final = 1024
MAX_PAGES_PER_BLOCK: Final = 256
MAX_PAGE_SIZE_BYTES: Final = 65536
MAX_TOTAL_BYTES: Final = 16 * 1024 * 1024
MAX_SAFE_INTEGER: Final = 2**53 - 1


class NANDFlashError(Exception):
    """Base class for stable, expected device-operation failures."""


class GeometryError(NANDFlashError, ValueError):
    """The requested device geometry is outside the supported teaching bounds."""


class AddressError(NANDFlashError, IndexError):
    """A zero-based block or page address does not exist on this device."""


class PayloadError(NANDFlashError, ValueError):
    """A page payload is not bytes of exactly the configured page size."""


class ProgramError(NANDFlashError, RuntimeError):
    """A page has already consumed its one allowed program operation."""


class EraseLimitError(NANDFlashError, RuntimeError):
    """The block has exhausted its configured erase-cycle budget."""


class NANDFlash:
    """Logical SLC NAND with full-page program and block-granular erase.

    ``block_count`` and ``pages_per_block`` form a rectangular array. A fresh
    cell stores FF everywhere. Program changes a complete page once; erase
    restores every page in one block and increments that block's cycle counter.
    """

    __slots__ = (
        "_block_count",
        "_pages_per_block",
        "_page_size_bytes",
        "_max_erase_cycles",
        "_pages",
        "_programmed",
        "_erase_counts",
    )

    def __init__(
        self,
        block_count: int,
        pages_per_block: int,
        page_size_bytes: int,
        max_erase_cycles: int | None = None,
    ) -> None:
        """Create erased media, rejecting booleans and overlarge geometry."""
        dimensions = (block_count, pages_per_block, page_size_bytes)
        if (
            any(type(value) is not int for value in dimensions)
            or not 1 <= block_count <= MAX_BLOCKS
            or not 1 <= pages_per_block <= MAX_PAGES_PER_BLOCK
            or not 1 <= page_size_bytes <= MAX_PAGE_SIZE_BYTES
            or block_count * pages_per_block * page_size_bytes > MAX_TOTAL_BYTES
            or (
                max_erase_cycles is not None
                and (
                    type(max_erase_cycles) is not int
                    or not 0 <= max_erase_cycles <= MAX_SAFE_INTEGER
                )
            )
        ):
            raise GeometryError("geometry values are outside supported bounds")

        self._block_count = block_count
        self._pages_per_block = pages_per_block
        self._page_size_bytes = page_size_bytes
        self._max_erase_cycles = max_erase_cycles
        erased_page = bytes([0xFF]) * page_size_bytes
        self._pages = [
            [erased_page for _ in range(pages_per_block)] for _ in range(block_count)
        ]
        self._programmed = [[False] * pages_per_block for _ in range(block_count)]
        self._erase_counts = [0] * block_count

    @property
    def block_count(self) -> int:
        """Number of erase blocks (zero-based block indices)."""
        return self._block_count

    @property
    def pages_per_block(self) -> int:
        """Number of pages in each erase block."""
        return self._pages_per_block

    @property
    def page_size_bytes(self) -> int:
        """Number of bytes in each page."""
        return self._page_size_bytes

    @property
    def max_erase_cycles(self) -> int | None:
        """Configured per-block erase ceiling, or ``None`` for unlimited."""
        return self._max_erase_cycles

    def _block(self, block: int) -> int:
        """Validate a block independently so all commands share one rule."""
        if type(block) is not int or not 0 <= block < self._block_count:
            raise AddressError("block index out of range")
        return block

    def _page(self, page: int) -> int:
        """Validate a page after its block, as specified by the shared contract."""
        if type(page) is not int or not 0 <= page < self._pages_per_block:
            raise AddressError("page index out of range")
        return page

    def read(self, block: int, page: int) -> bytes:
        """Return the immutable contents of one page."""
        b = self._block(block)
        p = self._page(page)
        return bytes(self._pages[b][p])

    def program(self, block: int, page: int, data: bytes) -> None:
        """Program a full page once; failed validation leaves state untouched."""
        b = self._block(block)
        p = self._page(page)
        if type(data) is not bytes:
            raise PayloadError("page data must be bytes")
        if len(data) != self._page_size_bytes:
            raise PayloadError("page data must match page_size_bytes")
        if self._programmed[b][p]:
            raise ProgramError("page has already been programmed since erase")

        # A newly erased page contains only ones, so every byte payload obeys
        # NAND's one-to-zero direction. This mask expresses the electrical rule.
        self._pages[b][p] = bytes(
            old & new for old, new in zip(self._pages[b][p], data, strict=True)
        )
        self._programmed[b][p] = True

    def erase_block(self, block: int) -> None:
        """Erase every page in a block, unless that block has reached its limit."""
        b = self._block(block)
        if self._erase_counts[b] >= MAX_SAFE_INTEGER or (
            self._max_erase_cycles is not None
            and self._erase_counts[b] >= self._max_erase_cycles
        ):
            raise EraseLimitError("block erase limit reached")
        erased_page = bytes([0xFF]) * self._page_size_bytes
        self._pages[b] = [erased_page for _ in range(self._pages_per_block)]
        self._programmed[b] = [False] * self._pages_per_block
        self._erase_counts[b] += 1

    def snapshot(self) -> dict[str, object]:
        """Return detached, JSON-compatible state for learning and conformance."""
        return {
            "pages": [[page.hex() for page in block] for block in self._pages],
            "erase_counts": list(self._erase_counts),
            "programmed": [list(block) for block in self._programmed],
        }
