### Why code-behind stubs and not full plumbing

ContentDialog is not driven by a simple `IsOpen` DP — the caller
must `await dialog.ShowAsync()` to present it. The lifecycle
plumbing lives on the host project's code-behind, the same shape
the HTML/React backends use for the equivalent dialog primitive.
This emitter writes the XAML element, the comment contract, and
the Closed event handler; the host writes the ShowAsync/Hide
side. A follow-up PR can lift this into an emitted attached
property + a small static helper class — leaves the spec-shape
intact today.

