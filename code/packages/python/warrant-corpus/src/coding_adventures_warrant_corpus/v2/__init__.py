"""The v2 lane runner library (VER04r, slices S6a and S10e).

The v1 runner (the rest of this package) reads ``warrant-corpus-v1``, a closed
profile with ``U64`` arithmetic that *refuses* on overflow.  The v2 profile
(``warrant-corpus-v2``) is richer -- signed types, one wrapping rule, structural
values, chunked suites, fault controls as data -- and its conformance kit
(``code/specs/fixtures/warrant-corpus-conformance-v1/v2-*.json``) demands a new
evaluator.  This sub-package is that evaluator.  It implements the levels
``ir.core`` (S6a) and, since S10e, ``ir.fixed`` (the 35 built-ins of Appendix A.1)
and ``ir.tuple`` (tuples, and ``match`` on a ``Result``); ``ir.data``,
``ir.list`` and ``ir.seq`` are defined but not implemented, and what needs one
is ``level-missing``:

====================  =========================================================
module                what it implements
====================  =========================================================
:mod:`.wtypes`        section 2.1 types: parse, ``flows`` and ``unify``
:mod:`.values`        section 2.3 value decoder, section 2.4 stream encoder
:mod:`.arith`         section 3: wrapping, ``Nat``/``Int`` window
:mod:`.ir`            section 4 + Appendix A: checker, call counting, laziness
:mod:`.fixed`         Appendix A.1: the value rules of the ``ir.fixed`` built-ins
:mod:`.patterns`      Appendix A.1: ``match`` patterns, usefulness (``ir.tuple``)
:mod:`.domains`       section 5.1: descriptors, ``size()`` and ``nth()``
:mod:`.chunks`        sections 5.2 to 5.5: chunk files, enumerating one chunk alone
:mod:`.faults`        section 8: the three fault wrappers
:mod:`.semantics`     the seams a wrong implementation would get wrong
:mod:`.kit`           the conformance harness over the v2 kit
====================  =========================================================

Reading order::

    wtypes -> semantics -> arith -> fixed -> patterns -> values -> ir
        -> domains -> chunks -> faults -> kit

It reads no corpus, binding or manifest yet: the corpus reader, the binding, the
session and the manifest are slice S6b, and moving ``python/ct-compare`` to v2 is
S6c.  It does not import the oracle (``python/warrant-oracle``): a lane is an
independent implementation of the spec, and the kit is what proves the two agree.
"""

from __future__ import annotations

from .errors import IrOverflow, KitError, Refusal
from .ir import IMPLEMENTED_LEVELS, Program
from .kit import KitReport, run_kit
from .semantics import STANDARD, Semantics

__all__ = [
    "IMPLEMENTED_LEVELS",
    "STANDARD",
    "IrOverflow",
    "KitError",
    "KitReport",
    "Program",
    "Refusal",
    "Semantics",
    "run_kit",
]
