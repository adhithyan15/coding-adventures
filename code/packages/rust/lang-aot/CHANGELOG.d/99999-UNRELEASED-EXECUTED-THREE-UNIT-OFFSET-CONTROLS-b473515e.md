## Unreleased - Executed three-unit offset controls

Add ten separate three-unit runtime programs to the previous 26 sources.
Six available backends execute all 36 sources (216 actual cells) with exact
stdout and normal exit, including ordered condition/selected-branch markers.
Previous 156 cells remain exact; CLR is unavailable and not counted. Full LANG
passes 484/0; all six affected packages pass 1,779/0 with one existing ignored
test over 78 targets, plus strict lint. Focused counts are included rather than
added to full-suite totals. Hosted acceptance remains a separate delivery gate.
Final-source Miri DEFERRED/PENDING, never passed.
