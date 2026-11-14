"""The Warrant binding table resolves against the real exports.

The whole corpus run lives in ``BUILD`` (``tests/warrant_evidence.py check``);
this test only proves, inside pytest, that ``evidence.bindings`` is well-formed
and names real, callable exports, so a rename here fails fast.
"""

from pathlib import Path

from coding_adventures_warrant_corpus import parse_binding
from coding_adventures_warrant_corpus.binding import resolve_exports

import coding_adventures_ct_compare as ct

PACKAGE = Path(__file__).resolve().parents[1]


def test_bindings_name_the_real_exports() -> None:
    binding = parse_binding((PACKAGE / "evidence.bindings").read_bytes())
    exports = resolve_exports(binding, PACKAGE)
    assert exports["eq"] is ct.ct_eq
    assert exports["eq_fixed"] is ct.ct_eq_fixed
    assert exports["select_bytes"] is ct.ct_select_bytes
    assert exports["eq_u64"] is ct.ct_eq_u64
    assert binding.package == "python/ct-compare"
