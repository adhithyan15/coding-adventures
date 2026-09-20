# OCAML07 representative CI authority

`manifest.json` is the closed machine-readable authority for the OCAML07
representative package workflow.  The validator binds the exact OCAML03 pins,
the four-package dependency order, the independent OCAML06 analyzer, the two
installed downstream receipts, all three targets, the governed source trees,
and the workflow itself.

The tree digests cover each relative filename, byte length, and file content in
sorted order.  The workflow digest is also compiled into
`ocaml_representative_ci.py`, so changing the manifest and workflow together is
not sufficient to widen execution.  Run this after an intentional governed
change:

```text
python code/scripts/ocaml_representative_ci.py validate-repository
python -m unittest discover -s code/scripts/tests -p test_ocaml_representative_ci.py
```

Never refresh a digest to silence a failure.  First review the package,
fixture, action, command, permission, and evidence changes against OCAML07.
