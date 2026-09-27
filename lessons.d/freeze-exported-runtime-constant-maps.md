# Freeze exported runtime constant maps

TypeScript's `as const` is compile-time only; it does not stop JavaScript callers
from replacing properties on an exported object. Stable protocol identifiers
must survive hostile or accidental runtime mutation, so wrap exported constant
maps in `Object.freeze` and add a runtime regression that attempts mutation
through a widened mutable type.
