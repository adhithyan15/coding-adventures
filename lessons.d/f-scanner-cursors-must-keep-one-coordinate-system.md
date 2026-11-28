# F# scanner cursors must keep one coordinate system

### The failure

A scanner that takes an absolute source offset and returns an absolute next
offset can silently lose fields if its caller subtracts the slice start before
calling, then adds that start again to the returned position. The coordinates
remain in range, so compilation and simple inputs can look correct.

### The fix

Choose one coordinate system for the scanner interface. Keep the cursor,
boundary, and returned next position all absolute, and test attributes after
leading document text as well as attributes at source offset zero.
