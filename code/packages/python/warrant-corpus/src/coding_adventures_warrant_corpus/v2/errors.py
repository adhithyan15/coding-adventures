"""The two ways a v2 conformance computation can stop early.

A conformance case has an *outcome*, and some outcomes are refusals: the
decoder says ``value``, the IR checker says ``ill-typed``, the chunk validator
says ``chunk-file``.  Those words are the kit's own closed vocabulary
(VER04r section 11, R25), so a refusal carries exactly one of them as ``code``
and the harness compares the code with the case's ``expect``.

* :class:`Refusal` -- the input is refused with a kit code.  Every library
  function in :mod:`.values`, :mod:`.ir`, :mod:`.domains`, :mod:`.chunks` and
  :mod:`.faults` raises it; the lane's own failure ids (``stream-mismatch``,
  ``arg-mutated`` ...) are a different set and live in :mod:`..errors`.
* :class:`KitError` -- the *case* is broken (a negative index, a stand-in model
  of the wrong signature).  VER04r R45(f): that is a defect of the fixture,
  never an outcome, so it is not a refusal and no ``expect`` can name it.
"""

from __future__ import annotations


class Refusal(Exception):
    """Refused with the kit code ``code``; ``message`` is for humans only."""

    def __init__(self, code: str, message: str = "") -> None:
        super().__init__(f"{code}: {message}" if message else code)
        self.code = code
        self.message = message


class IrOverflow(Refusal):
    """A ``Nat``/``Int`` left the exact window, or a ``Bytes`` grew too large.

    ``klass`` is the split of VER04r section 3.2: ``"corpus"`` when no
    ``subject_op`` sits beneath the node that overflowed (the generator could
    have evaluated that node, so an honest corpus never contains the overflow),
    ``"case"`` when one does (the subject's answer made it overflow, which is
    an ordinary case failure).
    """

    def __init__(self, klass: str, message: str = "") -> None:
        super().__init__("ir-overflow", message)
        self.klass = klass


class KitError(Exception):
    """The fixture case itself is malformed (VER04r R45(f)); never an outcome."""
