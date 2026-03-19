"""Hazard Detection — keeping the CPU pipeline from tripping over itself.

This package detects and resolves the three types of pipeline hazards
that occur in a pipelined CPU:

- **Data hazards** (RAW): an instruction needs a register value that
  a previous instruction hasn't written yet. Resolved by forwarding
  or stalling.

- **Control hazards**: a branch was mispredicted, so the pipeline
  fetched wrong instructions. Resolved by flushing.

- **Structural hazards**: two instructions need the same hardware
  resource at the same time. Resolved by stalling.

This package is standalone — it works with any pipeline implementation.
It only needs PipelineSlot descriptors of what's in each stage.

Quick start:

    from hazard_detection import HazardUnit, PipelineSlot, HazardAction

    unit = HazardUnit()
    result = unit.check(if_slot, id_slot, ex_slot, mem_slot)

    if result.action == HazardAction.STALL:
        # freeze the pipeline for result.stall_cycles cycles
        ...
"""

from hazard_detection.control_hazard import ControlHazardDetector
from hazard_detection.data_hazard import DataHazardDetector
from hazard_detection.hazard_unit import HazardUnit
from hazard_detection.structural_hazard import StructuralHazardDetector
from hazard_detection.types import HazardAction, HazardResult, PipelineSlot

__all__ = [
    "ControlHazardDetector",
    "DataHazardDetector",
    "HazardAction",
    "HazardResult",
    "HazardUnit",
    "PipelineSlot",
    "StructuralHazardDetector",
]
