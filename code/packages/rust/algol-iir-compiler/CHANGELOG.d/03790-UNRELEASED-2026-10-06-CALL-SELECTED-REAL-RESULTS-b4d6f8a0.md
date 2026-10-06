# Changed

- Allow conditional expressions with procedure-calling selectors to retain runtime-real formatter provenance when both value branches are direct formatter-safe real procedure calls that do not depend on real name-actual provenance. Provenance-backed local and real-name branches remain conservative across selector calls.
