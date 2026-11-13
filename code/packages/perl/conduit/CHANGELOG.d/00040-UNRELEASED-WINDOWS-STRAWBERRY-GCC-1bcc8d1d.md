## Unreleased — Windows: use Strawberry's own gcc

- `check-perl-extension-windows.ps1` now puts Strawberry Perl's `c\bin` (its
  MinGW gcc) ahead of other toolchains on PATH and logs which `gcc` it found.
  The shim compile in `perl-bridge` failed with no compiler diagnostic
  (`error building xs.dll`) when another gcc was resolved first.
