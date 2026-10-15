## Unreleased - Executed four-unit offset controls

Add ten separate four-unit sources while retaining the prior 36 sources and
216 actual backend cells. Six available backends execute all 46 sources and
276 cells with exact stdout and normal exit, including ordered CL3/CR5 markers.
CLR is unavailable and not counted. Full unfiltered LANG passes 486/0; all six
affected packages pass 1,784/0 with one existing ignore over 78 targets, plus
strict lint. Focused cells are included rather than added to full-suite totals.
Hosted/native CI remains separate. Miri DEFERRED/PENDING, never passed.
