# Two bans on one file need two messages, or the remediation advice contradicts itself

A symlink named `latexmkrc` violates both of this lint's rules. Collapsing them with
`elif` — reporting only the symlink — looked like sensible de-duplication and was a bug in
the *advice*, not the detection. The symlink message ends:

    Replace the link with the real file, or delete it.

For this path that instructs the author to create a real `latexmkrc`, which is precisely
what the other ban exists to prevent. The detection was fine either way; the report was
telling somebody to do the dangerous thing.

**When one artefact trips several rules, de-duplicate the FAILURE, never the GUIDANCE.**
One non-zero exit, one entry per rule — because the remediation for rule A can be a
violation of rule B, and the reader follows whichever text you printed.
