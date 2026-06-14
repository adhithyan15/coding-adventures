// Badge.mll — layout for the Badge component.
//
// A pill-shaped Box wrapping a single Text. The pill shape comes
// from a high border-radius in .msl; the .mll itself is content-
// shape-agnostic.

layout Badge {
  Box [ badge ] {
    Text [ badge-text ] (
      content : slot: label
    )
  }
}
