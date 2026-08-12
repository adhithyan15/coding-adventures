# mosaic-std-foundation

The dependency every native Mosaic application can include for a coherent
visual baseline. Version 0.1 supplies:

- package-owned light and dark color tokens;
- a 4/8/16/24/32 spacing scale and 6/10/16 radius scale;
- accessible display, heading, body, and caption components; and
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
    DisplayText ( content: "Welcome" )
    BodyText ( content: "Your native Mosaic app is ready." )
    FoundationIcon ( glyph: "star", accessible-label: "Featured" )
  }
}
```

All exported components are built exclusively from Mosaic kernel primitives
and pass the `native-complete` package profile on SwiftUI, Qt/QML, XAML,
Flutter, and Compose.

## Deliberate v0.1 boundary

A general `Surface` must accept arbitrary Mosaic children. UI29-2 now preserves
one default authored child region when a package reference is expanded, but
standalone exported-component child parameters and named regions are not yet
implemented. A host-native `node` slot is not a portable substitute because
Flutter's JSON runtime cannot materialize a Dart `Widget`. Foundation therefore
does not claim a fake text-only surface. Surface components land after the
remaining composition contract is available; the palette already reserves
surface and border tokens for them.
