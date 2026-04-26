"""ASIC placement: simulated annealing on HPWL + greedy row legalization."""

from __future__ import annotations

import math
import random
from dataclasses import dataclass

from asic_floorplan import Floorplan
from lef_def import Component, Def


@dataclass(frozen=True, slots=True)
class CellSize:
    """Footprint of one cell type."""

    cell_type: str
    width: float
    height: float


@dataclass
class PlacementOptions:
    method: str = "anneal"  # "anneal" only in v0.1.0
    iterations: int = 50_000
    seed: int = 42
    target_density: float = 0.7
    legalize: bool = True


@dataclass
class PlacementReport:
    final_hpwl: float
    cells_placed: int
    runtime_sec: float
    accepted_swaps: int
    rejected_swaps: int


@dataclass
class _PlacedCell:
    """Mutable per-cell placement state."""

    name: str
    cell_type: str
    width: float
    height: float
    x: float
    y: float
    row_index: int


def place(
    *,
    fp: Floorplan,
    cell_sizes: dict[str, CellSize],
    nets: list[list[str]] | None = None,
    options: PlacementOptions | None = None,
) -> tuple[Def, PlacementReport]:
    """Place every Component in the Floorplan onto a legal site.

    `nets`: optional list of nets, each represented as a list of cell instance
    names that share a connection. Used for HPWL minimization. If None, only
    overlap-free placement is computed (no wirelength optimization).
    """
    import time as _time

    if options is None:
        options = PlacementOptions()

    if not fp.rows:
        raise ValueError("floorplan has no rows; cannot place")

    rng = random.Random(options.seed)

    # Initialize: random row + left-to-right within row
    placed_cells: list[_PlacedCell] = []
    row_widths_used: list[float] = [0.0] * len(fp.rows)
    row_capacity = (fp.rows[0].num_x * fp.rows[0].step_x)

    for c in fp.components:
        size = cell_sizes.get(c.cell_type, CellSize(c.cell_type, 1.0, fp.rows[0].step_y or 1.0))
        # Find a row with capacity
        row_idx = _find_row_with_capacity(row_widths_used, row_capacity, size.width, rng)
        if row_idx is None:
            raise ValueError(
                f"placement: cell {c.name!r} (width {size.width}) doesn't fit in any row"
            )
        row = fp.rows[row_idx]
        x = row.origin_x + row_widths_used[row_idx]
        y = row.origin_y
        row_widths_used[row_idx] += size.width
        placed_cells.append(_PlacedCell(
            name=c.name, cell_type=c.cell_type,
            width=size.width, height=size.height,
            x=x, y=y, row_index=row_idx,
        ))

    start = _time.perf_counter()
    accepted = 0
    rejected = 0

    if nets and options.iterations > 0:
        # Simulated annealing on HPWL
        hpwl = _total_hpwl(placed_cells, nets)
        T0 = max(hpwl / max(len(placed_cells), 1), 1.0)
        T = T0
        cooling = (1e-3) ** (1.0 / options.iterations)

        for _ in range(options.iterations):
            i, j = _random_pair(len(placed_cells), rng)
            ci = placed_cells[i]
            cj = placed_cells[j]

            # Tentative swap: swap positions across rows.
            old_xy_i = (ci.x, ci.y, ci.row_index)
            old_xy_j = (cj.x, cj.y, cj.row_index)
            ci.x, cj.x = cj.x, ci.x
            ci.y, cj.y = cj.y, ci.y
            ci.row_index, cj.row_index = cj.row_index, ci.row_index

            new_hpwl = _total_hpwl(placed_cells, nets)
            delta = new_hpwl - hpwl
            if delta < 0 or rng.random() < math.exp(-delta / max(T, 1e-9)):
                accepted += 1
                hpwl = new_hpwl
            else:
                # Revert swap
                ci.x, ci.y, ci.row_index = old_xy_i
                cj.x, cj.y, cj.row_index = old_xy_j
                rejected += 1

            T *= cooling
        final_hpwl = hpwl
    else:
        final_hpwl = 0.0 if not nets else _total_hpwl(placed_cells, nets)

    if options.legalize:
        _legalize(placed_cells, fp)

    runtime = _time.perf_counter() - start

    # Build placed Def
    new_components = [
        Component(
            name=p.name,
            cell_type=p.cell_type,
            placed=True,
            location_x=p.x,
            location_y=p.y,
            orientation="N",
        )
        for p in placed_cells
    ]
    placed_def = Def(
        design="placed",
        die_area=fp.die,
        rows=list(fp.rows),
        components=new_components,
        pins=list(fp.pins),
    )

    report = PlacementReport(
        final_hpwl=final_hpwl,
        cells_placed=len(placed_cells),
        runtime_sec=runtime,
        accepted_swaps=accepted,
        rejected_swaps=rejected,
    )
    return (placed_def, report)


def _find_row_with_capacity(
    widths_used: list[float],
    capacity: float,
    needed: float,
    rng: random.Random,
) -> int | None:
    """Pick a row at random that has room for `needed`. None if no row fits."""
    candidates = [i for i, w in enumerate(widths_used) if w + needed <= capacity]
    if not candidates:
        # Fallback: just pick the row with the most remaining space.
        best = min(range(len(widths_used)), key=lambda i: widths_used[i])
        if widths_used[best] + needed > capacity * 1.5:
            return None
        return best
    return rng.choice(candidates)


def _random_pair(n: int, rng: random.Random) -> tuple[int, int]:
    if n < 2:
        return (0, 0)
    i = rng.randrange(n)
    j = rng.randrange(n - 1)
    if j >= i:
        j += 1
    return (i, j)


def _total_hpwl(cells: list[_PlacedCell], nets: list[list[str]]) -> float:
    """Sum of half-perimeter wirelengths across all nets."""
    by_name = {c.name: c for c in cells}
    total = 0.0
    for net in nets:
        if len(net) < 2:
            continue
        xs: list[float] = []
        ys: list[float] = []
        for name in net:
            c = by_name.get(name)
            if c is None:
                continue
            xs.append(c.x)
            ys.append(c.y)
        if len(xs) < 2:
            continue
        total += (max(xs) - min(xs)) + (max(ys) - min(ys))
    return total


def _legalize(cells: list[_PlacedCell], fp: Floorplan) -> None:
    """Greedy row-by-row legalization: snap cells to row coordinates and
    eliminate overlaps by left-to-right packing within each row."""
    by_row: dict[int, list[_PlacedCell]] = {i: [] for i in range(len(fp.rows))}
    for c in cells:
        by_row[c.row_index].append(c)

    for row_idx, row_cells in by_row.items():
        row = fp.rows[row_idx]
        # Sort by current x position
        row_cells.sort(key=lambda c: c.x)
        cursor = row.origin_x
        for c in row_cells:
            c.x = cursor
            c.y = row.origin_y
            cursor += c.width
