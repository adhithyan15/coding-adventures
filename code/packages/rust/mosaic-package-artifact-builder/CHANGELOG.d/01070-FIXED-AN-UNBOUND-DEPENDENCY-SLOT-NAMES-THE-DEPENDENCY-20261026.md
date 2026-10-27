### Fixed — an unbound dependency slot fails naming the dependency (E6 F1)

A package build that mounts `pkg::P::C` without binding one of `C`'s
required slots now fails with `package reference error for component '…'`.
The message comes from the resolver's new `SlotBindingMissing`. It names
`pkg::P::C`, the slot and the call site, and says how to fix it.

Before, the error was `pipeline error … UnknownSlot` naming the consumer.
If the consumer happened to have a slot with the same name, the build
succeeded, and the consumer's value was rendered inside the dependency.
`an_unbound_dependency_slot_fails_naming_the_dependency` covers that case.

Validation of the inlined tree in `resolve_layout_package_references` now
reports every error rather than `errs[0]`. A single error keeps its old
form. `every_validation_error_after_inlining_is_reported` covers two
duplicate parts reported together.
