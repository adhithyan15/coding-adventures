### Added — package styles understand model-declared slot states

Package composition now passes every component's `one-of` slot values into
mosstyle compilation. This applies independently to the parent and each
dependency component, so merged backend-neutral style IR preserves the owning
slot for model-declared states without confusing same-named slots across
package boundaries (UI49, #14299).

