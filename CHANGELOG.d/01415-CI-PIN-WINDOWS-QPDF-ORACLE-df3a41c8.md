### CI: use a pinned qpdf release on Windows

Windows now downloads qpdf 12.4.2 from its GitHub release, verifies the
published SHA-256 before extraction, and requires the extracted executable to
report the pinned version. The independent PDF oracle remains mandatory.
