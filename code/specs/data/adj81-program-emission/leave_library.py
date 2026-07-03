"""Compiled rulebook = a library (represents the OFFLINE capable-model derivation +
compilation step; cf. ADJ71 CAS program cache for a real legal source). The runtime
small model never sees this logic -- it only extracts the facts the SCHEMA declares.
"""

# The facts this library needs (the small model's extraction targets).
SCHEMA = {
    "employment_type": "one of: full_time | part_time",
    "hire_year": "integer year the person was hired",
}

PROVENANCE = "claimed_from_model_memory -> would be spider-grounded offline on the capable model"


def leave_days(employment_type: str, hire_year: int) -> int:
    """General rule: 20 days. Override: part-time hired after 2020 -> 12 days."""
    if employment_type == "part_time" and hire_year > 2020:
        return 12
    return 20
