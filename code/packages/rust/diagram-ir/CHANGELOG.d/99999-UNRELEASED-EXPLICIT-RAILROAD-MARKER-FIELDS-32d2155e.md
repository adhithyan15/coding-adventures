## Unreleased - Explicit Railroad marker fields

Retain required start_marker/end_marker points on LayoutedRailroadRule;
external Rust literals must provide both. Semantic source labels stay distinct
from displayed rule/special text. Checked layout validates both coordinates.
Complete affected fourteen-package validation passes 902 tests/38 targets and
strict lint. Arbitrary forged-layout safety and marker enclosure remain open.
Miri DEFERRED/PENDING, never passed.
