---
category: Compiler / VM / language pipeline
---

# A tree walk that treats only tagged nodes as nodes skips the arms of a match, so every rewrite of an IR tree needs a case with the thing inside an arm

An IR node is a list that starts with a tag, and a `match` holds its arms as `[pattern, body]` pairs, which are not nodes.  The
conformance kit's `_rewrite_subjects` (each `subject_op` becomes a call to its stand-in) walked children that were nodes and
returned an arm list untouched, so a subject call inside a `match` arm survived the rewrite and the oracle failed with "a subject call is
forbidden here".  No S5 case had a `match`, so nothing noticed until S10d.  When a walk rewrites or counts nodes, give `match` its own
case, and add a vector with the thing inside an arm (a subject call in the arm not taken, in the scrutinee, and in a nested arm).
