# UI68: Form-Associated Custom Elements

Venture exposes a host-neutral ElementInternals-style contract for autonomous
custom elements. Parsing retains empty custom elements plus authored `name`,
`form`, labels, ARIA naming, descriptions, and roles. The shared control model
assigns stable keys and document ordinals, while attachment is explicit so an
unregistered element cannot accidentally participate in a form.

Attached internals own an optional string, file, or ordered entry-list value
and an independently retained restoration value of the same shape. Text,
entry count, individual file, and aggregate payload limits are checked before
state changes. Explicit form owners and containing-form indexes can be
reassociated without toolkit state. Successful entries merge with native
controls by document ordinal before URL-encoded or multipart serialization.

Validity flags, a required non-empty message for invalid state, and an opaque
validation anchor are projected through reusable diagnostics. Disabled state
bars validation and serialization. Association, disabled, reset, and
restore/autocomplete transitions enqueue typed lifecycle callbacks for the
script host; reset deliberately invokes the callback rather than guessing a
component's default value.

Labels and authored accessibility metadata initialize the projection.
Internals may override role, name, and description, publish numeric value
metadata, and accept shared SetValue, ClearValue, Increment, and Decrement
actions. BrowserSession re-exports the same contract to every native and web
host, so generated toolkits neither own custom-element form state nor
reimplement lifecycle, validation, or serialization policy.
