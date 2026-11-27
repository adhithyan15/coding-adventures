"""A tiny, deterministic model of disk head and platter timing.

The model uses abstract integer ticks. It is deliberately an educational
functional model, not a prediction of any physical drive.
"""

from dataclasses import dataclass


class HddError(Exception):
    """Base class for requests the modeled drive cannot accept."""


class InvalidLbaError(HddError):
    """The LBA is not an integer sector address on this medium."""


class InvalidSectorDataError(HddError):
    """Write data is not bytes-like or is not exactly one sector long."""


class ClockOverflowError(HddError):
    """A successful operation would exceed the portable exact clock limit."""


_MAX_SAFE_TICK = 2**53 - 1


@dataclass(frozen=True, slots=True)
class HddGeometryV1:
    """The fixed v1 teaching geometry."""

    surfaces: int = 2
    tracks_per_surface: int = 4
    sectors_per_track: int = 4
    bytes_per_sector: int = 8
    seek_ticks_per_track: int = 3
    transfer_ticks_per_sector: int = 1

    @property
    def sector_count(self) -> int:
        """Number of addressable sectors in the medium."""
        return self.surfaces * self.tracks_per_surface * self.sectors_per_track

    @property
    def sectors_per_cylinder(self) -> int:
        """Sectors across every surface at one track coordinate."""
        return self.surfaces * self.sectors_per_track


@dataclass(frozen=True, slots=True)
class IoTiming:
    """Exact abstract clock costs for one successful sector operation."""

    start_tick: int
    seek_ticks: int
    rotation_ticks: int
    transfer_ticks: int
    total_ticks: int
    end_tick: int


@dataclass(frozen=True, slots=True)
class SectorResult:
    """Data returned by a read together with its elapsed model time."""

    data: bytes
    timing: IoTiming


@dataclass(frozen=True, slots=True)
class HddState:
    """Immutable, owned snapshot of clock, head, and medium contents."""

    tick: int
    head_track: int
    sectors: tuple[bytes, ...]


class HddSimulator:
    """One synchronous disk, with FIFO order defined by call order."""

    def __init__(self) -> None:
        self._geometry = HddGeometryV1()
        self._tick = 0
        self._head_track = 0
        zero = bytes(self._geometry.bytes_per_sector)
        self._sectors = [zero for _ in range(self._geometry.sector_count)]

    @property
    def geometry(self) -> HddGeometryV1:
        """The immutable geometry profile for this simulator."""
        return self._geometry

    def get_state(self) -> HddState:
        """Return a frozen snapshot; later operations cannot change it."""
        return HddState(self._tick, self._head_track, tuple(self._sectors))

    def read_sector(self, lba: int) -> SectorResult:
        """Read exactly one sector, advancing deterministic timing state."""
        target_track, target_sector = self._validate_lba(lba)
        timing = self._timing_for(target_track, target_sector)
        data = self._sectors[lba]
        self._complete(timing, target_track)
        return SectorResult(data, timing)

    def write_sector(self, lba: int, data: bytes | bytearray | memoryview) -> IoTiming:
        """Atomically replace one sector after validating the whole request."""
        target_track, target_sector = self._validate_lba(lba)
        if type(data) not in (bytes, bytearray, memoryview):
            raise InvalidSectorDataError("sector data must be bytes-like")
        try:
            data_size = data.nbytes if isinstance(data, memoryview) else len(data)
        except ValueError as error:
            # A released memoryview has no readable size; reject it before the
            # request can affect the modeled medium or clock.
            raise InvalidSectorDataError(
                "sector data view is no longer usable"
            ) from error
        if data_size != self._geometry.bytes_per_sector:
            raise InvalidSectorDataError(
                "sector data must contain exactly "
                f"{self._geometry.bytes_per_sector} bytes"
            )
        # Copy only after exact, built-in-type and size checks. This bounds the
        # allocation and avoids calling user-defined __bytes__ methods.
        try:
            sector_data = bytes(data)
        except (TypeError, ValueError) as error:
            raise InvalidSectorDataError(
                "sector data view is no longer usable"
            ) from error
        timing = self._timing_for(target_track, target_sector)
        self._sectors[lba] = sector_data
        self._complete(timing, target_track)
        return timing

    def _validate_lba(self, lba: int) -> tuple[int, int]:
        if type(lba) is not int:
            raise InvalidLbaError("LBA must be an integer sector address")
        if not 0 <= lba < self._geometry.sector_count:
            raise InvalidLbaError("LBA is outside this medium")
        track = lba // self._geometry.sectors_per_cylinder
        sector = lba % self._geometry.sectors_per_track
        return track, sector

    def _timing_for(self, target_track: int, target_sector: int) -> IoTiming:
        start = self._tick
        seek = (
            abs(target_track - self._head_track)
            * self._geometry.seek_ticks_per_track
        )
        phase_after_seek = (start + seek) % self._geometry.sectors_per_track
        rotation = (target_sector - phase_after_seek) % self._geometry.sectors_per_track
        transfer = self._geometry.transfer_ticks_per_sector
        total = seek + rotation + transfer
        end = start + total
        if end > _MAX_SAFE_TICK:
            raise ClockOverflowError("operation would exceed the maximum exact tick")
        return IoTiming(start, seek, rotation, transfer, total, end)

    def _complete(self, timing: IoTiming, target_track: int) -> None:
        self._tick = timing.end_tick
        self._head_track = target_track
