# Changelog

## 0.1.0

- Add paired deterministic batch collection, calibration and rank analysis.
- Add the indexed-array command line workload and JSON report.
- Validate optional workload metadata before core report projection, retaining valid Unicode identifiers and rejecting blank, NUL, malformed Unicode and wrong-type values.
- Report OS and architecture metadata from interpreter compile/runtime constants, without calling host-probing platform helpers.
