"""Top-level HIR container with JSON round-trip."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

from hdl_ir.module import Library, Module
from hdl_ir.provenance import Provenance


def _prov_to_dict(p: Provenance | None) -> dict[str, object] | None:
    return p.to_dict() if p is not None else None


def _prov_from_dict(d: object) -> Provenance | None:
    if isinstance(d, dict):
        return Provenance.from_dict(d)
    return None


SCHEMA_VERSION = "0.1.0"


@dataclass
class HIRStats:
    module_count: int
    instance_count: int
    process_count: int
    cont_assign_count: int
    net_count: int


@dataclass
class HIR:
    """Top-level HIR document. Holds the top module name and the module
    dictionary; optionally a libraries map for VHDL multi-library designs.
    """

    top: str
    modules: dict[str, Module] = field(default_factory=dict)
    libraries: dict[str, Library] = field(default_factory=dict)
    provenance: Provenance | None = None
    version: str = SCHEMA_VERSION

    # ------------------------------------------------------------------
    # JSON round-trip
    # ------------------------------------------------------------------

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "format": "HIR",
            "version": self.version,
            "top": self.top,
            "modules": {k: m.to_dict() for k, m in self.modules.items()},
        }
        if self.libraries:
            d["libraries"] = {k: lib.to_dict() for k, lib in self.libraries.items()}
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> HIR:
        if d.get("format") != "HIR":
            raise ValueError(f"not an HIR document (format={d.get('format')!r})")

        version = str(d.get("version", SCHEMA_VERSION))
        major = version.split(".")[0]
        expected_major = SCHEMA_VERSION.split(".")[0]
        if major != expected_major:
            raise ValueError(
                f"HIR major version mismatch: file is {version}, "
                f"library expects {SCHEMA_VERSION}"
            )

        mods_raw = d.get("modules", {})
        libs_raw = d.get("libraries", {})

        return cls(
            top=str(d["top"]),
            modules={
                k: Module.from_dict(v)  # type: ignore[arg-type]
                for k, v in mods_raw.items()  # type: ignore[union-attr]
            },
            libraries={
                k: Library.from_dict(v)  # type: ignore[arg-type]
                for k, v in libs_raw.items()  # type: ignore[union-attr]
            },
            provenance=_prov_from_dict(d.get("provenance")),
            version=version,
        )

    def to_json(self, path: str | Path) -> None:
        Path(path).write_text(json.dumps(self.to_dict(), indent=2))

    @classmethod
    def from_json(cls, path: str | Path) -> HIR:
        return cls.from_dict(json.loads(Path(path).read_text()))

    def to_json_str(self) -> str:
        return json.dumps(self.to_dict(), indent=2)

    @classmethod
    def from_json_str(cls, s: str) -> HIR:
        return cls.from_dict(json.loads(s))

    # ------------------------------------------------------------------
    # Stats
    # ------------------------------------------------------------------

    def stats(self) -> HIRStats:
        return HIRStats(
            module_count=len(self.modules),
            instance_count=sum(len(m.instances) for m in self.modules.values()),
            process_count=sum(len(m.processes) for m in self.modules.values()),
            cont_assign_count=sum(
                len(m.cont_assigns) for m in self.modules.values()
            ),
            net_count=sum(len(m.nets) for m in self.modules.values()),
        )
