- **DraftEditor 0.2.0 (E1).** Journal requires `mosaic-pkg-draft-editor`
  `0.2.0` and binds its new `body-placeholder` slot to `""` at the
  `DraftEditor` call site, so the entry body shows no hint, as before: the
  visible "Entry" label names it. The binding is required, because a package
  slot the consumer does not bind fails the build with `UnknownSlot` on
  every backend (`mosaic-editor-convergence-audit-v1.md` R1). The generated
  hosts gain an empty placeholder on the body field (`placeholder=""` on the
  web, `hintText: ""` on Flutter, `placeholderText: ""` on Qt, an empty
  `PlaceholderText` on XAML and an empty hint `Text` on Compose; SwiftUI
  omits it), which draws nothing. Nothing else in Journal changes.
