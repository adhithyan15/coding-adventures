# mosaic-std-foundation

The dependency every native Mosaic application can include for a coherent
visual baseline. Version 0.1 supplies:

- package-owned light and dark color tokens;
- a 4/8/16/24/32 spacing scale and 6/10/16 radius scale;
- accessible display, heading, body, and caption components; and
- a themed `Surface` that accepts arbitrary Mosaic children; and
- a semantic icon component whose accessible label is required by its MIL
  contract.

```toml
[dependencies]
mosaic-std-foundation = "0.1.0"
```

Dependency token defaults automatically style the foundation components during
package composition. To use the same vocabulary in an application's own MSL,
pass `tokens/foundation.json` as the application's `--token-palette`; this keeps
the palette as one included source of truth instead of copying its values.
Explicit application values remain the highest-precedence overrides.

```mll
layout Welcome {
  Column [ root ] {
    pkg::mosaic-std-foundation::Surface {
      pkg::mosaic-std-foundation::DisplayText ( content: "Welcome" )
      pkg::mosaic-std-foundation::BodyText (
        content: "Your native Mosaic app is ready."
      )
      pkg::mosaic-std-foundation::FoundationIcon (
        glyph: "star",
        accessible-label: "Featured"
      )
    }
  }
}
```

All components are built exclusively from Mosaic kernel primitives. A consuming
package expands `Surface` and its authored children before emission and passes
the `native-complete` profile on SwiftUI, Qt/QML, XAML, Flutter, and Compose.
The typography and icon leaf components also pass that profile as standalone
exports.

## Deliberate v0.1 boundary

UI29-2 currently preserves one default authored child region during package
expansion. That is sufficient for `Surface` in an included Foundation package,
which is the supported portable path. Compiling `Surface` itself as a standalone
host-facing component still reports
`composition.child-slot-parameter-unimplemented`; backend child parameters and
named regions remain follow-up work rather than being silently approximated.
