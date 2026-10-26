### Equivalent XAML list identities

Reuse live value-equivalent IReadOnlyList properties with scalar or nested scalar
contents before assignment. Preserve setter invocation, strict conversion, batch
release, and authoritative correction of local mutations. Unsupported shapes and
getter/enumerator failures retain normal replacement behavior. Executed host tests
cover identity, changed contents, nulls, fallback and bounded enumeration.
