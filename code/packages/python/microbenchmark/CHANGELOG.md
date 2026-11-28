# Changelog

## 0.1.0

- Bound public `analyze` input to 10,000 pairs before ratio transformation.
- Add wheel, sdist fixture, and isolated installed-consumer checks to BUILD.
- Document `Report.from_dict` as a decoded-object projection, not raw JSON
  duplicate-key ingress.
- Enforce the VER07ab closed dict shape, balanced pair order, and complete
  metadata group while preserving the object-only API.
- Add an unsigned capability proposal for monotonic time and standard output.
- Add paired deterministic batch collection, calibration and rank analysis.
- Add the indexed-array command line workload and JSON report.
- Validate optional workload metadata before core report projection, retaining valid Unicode identifiers and rejecting blank, NUL, malformed Unicode and wrong-type values.
- Report OS and architecture metadata from interpreter compile/runtime constants, without calling host-probing platform helpers.
