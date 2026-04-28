"""Top-level MOSFET wrapper. NMOS / PMOS unification."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Protocol

from mosfet_models.level1 import Level1Params, MosResult, evaluate_level1


class MosfetType(Enum):
    NMOS = "NMOS"
    PMOS = "PMOS"


class MosfetModel(Protocol):
    """Common interface for any MOSFET I-V model."""

    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float) -> MosResult: ...


@dataclass(frozen=True, slots=True)
class Level1Model:
    params: Level1Params

    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float = 300.15) -> MosResult:
        return evaluate_level1(self.params, V_GS, V_DS, V_BS, T)


@dataclass(frozen=True, slots=True)
class MOSFET:
    """A MOSFET tied to a specific I-V model. PMOS callers get sign-flipped
    inputs/outputs."""

    type: MosfetType
    model: MosfetModel

    def dc(
        self,
        V_GS: float,
        V_DS: float,
        V_BS: float = 0.0,
        T: float = 300.15,
    ) -> MosResult:
        if self.type == MosfetType.PMOS:
            r = self.model.dc(-V_GS, -V_DS, -V_BS, T)
            return MosResult(
                Id=-r.Id,
                gm=r.gm,
                gds=r.gds,
                gmb=r.gmb,
                Cgs=r.Cgs,
                Cgd=r.Cgd,
                Cgb=r.Cgb,
                Cbs=r.Cbs,
                Cbd=r.Cbd,
                region=r.region,
            )
        return self.model.dc(V_GS, V_DS, V_BS, T)
