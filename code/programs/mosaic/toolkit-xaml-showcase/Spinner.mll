// Spinner.mll — layout for the Spinner component.
//
// A Stack with a single Icon child. The Stack lets the .msl
// position the icon and any annotation overlay; the Icon itself
// renders the spinning glyph (Segoe Fluent / SF Symbols /
// fontawesome — backend-specific). Animation is the .msl's job
// where the target backend supports it.
//
// Why Stack and not Box?
// ----------------------
// Spinners often layer a visible glyph over a transparent
// background, and Stack is the kernel's z-axis container. A Box
// would be visually identical for the v0.1 single-glyph case, but
// Stack reserves the option for a future PR to overlay a
// progress-percentage Text on top of the glyph without changing
// the .mll surface.

layout Spinner {
  Stack [ spinner ] {
    Icon [ spinner-glyph ] (
      glyph : "spinner"
    )
  }
}
