# Changelog

## Unreleased — executed two-unit offset controls

Add ten owned two-unit programs to the existing sixteen. Execute all 26 sources
on six available backends (156 actual cells), checking exact stdout and normal
exit, including condition/selected-branch marker order. Preserve the original
96 cells; CLR is unavailable and is not counted. The imported producer runtime
test also passes separately. Full LANG passes 482/0; all six affected packages
pass 1,774 tests over 78 targets, with one existing ignored test, plus strict
lint. Independent exact-source and terminal-log reviews pass. Hosted CI remains
a separate delivery gate. Final-source Miri is DEFERRED/PENDING, never passed.
