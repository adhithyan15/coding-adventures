## Unreleased - CV2-S5d: the kit gains `v2-binding`

No source change. The runner already digest-checks and skips whole a category it does not know (`v2-binding`, until
S6b), and does so unchanged. `tests/test_v2_conformance.py` pinned the category list and the skip-free report to the six
categories of S5b; it now expects the seventh row (`v2-binding`: indexed 176, run 0, skipped 176, accounted, ok) after
the six, still 782 cases run and none failed. README counts updated (958 v2 cases in the kit, 782 run).
