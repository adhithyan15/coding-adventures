### Changed — ductusview defaults are frozen

- `DEFAULTS` became an export when the filmstrip ledger started scaling the
  pen for tiny marks. It is now `Object.freeze`d as well as typed `Readonly`, so
  no importer can change the defaults for every other renderer at run time.
