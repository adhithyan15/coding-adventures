### Fixed - MIL slot defaults

Slots with authored MIL defaults now emit non-null Dart fields and matching
optional constructor defaults. Reusable components can consume defaulted text,
number, and boolean values without analyzer errors.

