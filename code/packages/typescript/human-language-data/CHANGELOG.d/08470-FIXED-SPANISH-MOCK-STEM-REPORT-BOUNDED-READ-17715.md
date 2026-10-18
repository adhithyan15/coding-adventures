### Fixed — Spanish mock-stem reporting reads only its own track (#17715)

- Reject unsupported levels before vocabulary or corpus I/O, including calls
  from JavaScript where the TypeScript level union provides no protection.
- Derive reporter taught forms from Spanish lessons and curriculum membership
  plus the shared spine instead of parsing all language tracks. The reporter
  and the existing mock audit still share the same headword conversion.
- Keep every stem/option bucket and the 30-second test ceiling intact; the
  focused regression suite covers early rejection without a real corpus root.
