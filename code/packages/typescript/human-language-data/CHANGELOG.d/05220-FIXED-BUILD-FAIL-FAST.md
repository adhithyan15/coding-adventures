### Fixed — fail-fast Human Language data builds

- Stop the package BUILD on a failed prerequisite install or grammar-cell
  drift check instead of allowing later tests to mask the error.
- Select a real Python 3 interpreter across Unix and Windows, including the
  Windows `py -3` launcher when `python3` is only a Store alias.
