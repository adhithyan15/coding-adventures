# UI88 — Images from app data: finish the kernel `Image`, and rich content as blocks

**Status:** proposed (design note; nothing implemented). Unblocks Engram's
rendering gaps (#13936 LaTeX, card images, #13938 image occlusion) and Journal
photos. Needs the owner's decisions in §6.

**Builds on:**
- [UI29 — primitive kernel](UI29-primitive-kernel.md), which already lists
  `Image` as a kernel leaf;
- [UI13 — mosmodel](UI13-mosmodel.md), which already has an `image` slot type
  ("opaque image reference — backend-specific handle");
- [UI14 — moslayout](UI14-moslayout.md) (`Image ( slot: … )`);
- [UI59 — `files.open`](UI59-files-open-effect.md), which already returns picked
  image **bytes** to the app (`photo-picker-app`).

---

## 1. What exists, per backend

Mosaic does not need a new display primitive. It has one, and it is
unfinished where it matters.

| backend | `Image` today | a slot-bound source? |
| --- | --- | --- |
| React / HTML / web component | `<img src>` | yes |
| XAML | `<Image Source="{x:Bind …}">` (slot, literal or expression) | yes (string → `ImageSource`) |
| Qt | QML `Image` | yes (`source`) |
| Flutter | `Image.network` / `Image.asset` for a **string literal** | **no** |
| SwiftUI | `Image(systemName:)` placeholder ("a real image-asset pipeline is a follow-up") | **no** |
| Compose | **no `Image` arm**: the primitive is rejected as unknown | **no** |

So on the native backends Journal and Engram are driven on, an app cannot
show a picture it holds. That includes one UI59 just handed it.

## 2. Who needs it

- **Engram:** card fields carry `<img src="…">` media, which no build shows
  today. Every Engram UI draws a card face as plain text (see
  `engram-latex-rendering.md` §0). LaTeX Tier 1 is `<img src="latex-….png">`.
  Image occlusion is an image with masks over it.
- **Journal:** photos in an entry.
- **photo-picker-app:** showing the photo it just picked.

## 3. Proposal

### 3.1 The portable value of an `image` slot: a `data:` URI

An app produces the image from bytes it holds (media, a picked file, a
rendered formula), so the portable form is
`data:<mime>;base64,<bytes>`, with `image/png`, `image/jpeg`, `image/gif`,
`image/webp` and `image/svg+xml`. A backend may also accept `https:` URLs
(the web does), but apps must not depend on it.

- **Limit:** 16 MiB decoded per image, the `files.*` cap. Larger is refused
  at the app boundary, not in the host.
- **Failure:** an undecodable image renders as an empty box of its authored
  size with its accessible label. It never crashes the host.
- **Accessibility:** `Image` gains the `a11y-label` / `a11y-hidden` props
  `Text` already has, so decorative images can be hidden and meaningful ones
  named.

### 3.2 Finishing each backend

| backend | decoding (to verify when built) |
| --- | --- |
| Compose (Desktop) | new `Image` arm: bytes → `org.jetbrains.skia.Image.makeFromEncoded(…).toComposeImageBitmap()`; SVG via `loadSvgPainter` |
| SwiftUI | `NSImage(data:)` / `UIImage(data:)` → `Image(nsImage:)` |
| Flutter | `Image.memory(base64Decode(…))`, `SvgPicture` needs a package, so SVG may be deferred |
| XAML | `BitmapImage` from a stream; `data:` URIs are not native, so the binding needs a converter |
| Qt | QML `Image` with a `data:` source (verify), else a small `QQuickImageProvider` |
| web | unchanged |

Each backend gets an emitter test and a render in its screenshot harness.
The decoder runs off the UI thread where the toolkit allows.

### 3.3 Rich content as blocks, not HTML in the host

Card fields are HTML. Rendering HTML in five native toolkits would mean five
HTML renderers, five sanitisers and five attack surfaces. Instead the app
turns content into **blocks** and the layout draws them with the existing
kernel:

```text
slot card-blocks : list<list<text>> ;   // [is-image, value, label]
                                        // is-image: "" for text, "image" for an image
For ( each: slot: card-blocks , as: b ) {
  If ( when: ( b[0] ) ) { Image [ card-image ] ( source: ( b[1] ) , a11y-label: ( b[2] ) ) }
  Else                  { Text  [ card-text ]  ( content: ( b[1] ) ) }
}
```

A non-empty first cell is the image flag, because `when:` tests
truthiness, the same way Calendar's `If ( when: ( cell[2] ) )` does. No new
comparison syntax is needed.

- **Parsing and sanitising happen once, in Rust.** Engram already scans
  field HTML (`html_scan.rs`). Script, event attributes and remote URLs never
  reach a host, because blocks carry only text and `data:` images.
- **Engram:**
  - an `<img src=name>` becomes an image block from that media file;
  - `[latex]` / `[$]` / `[$$]` become image blocks from `latex-<sha1>`
    (`engram-latex-rendering.md` §2);
  - `[sound:…]` becomes a play control later (#13937).
- **Deferred:** inline formatting (bold, italic, colour spans). A first cut
  renders each text block plainly; styled runs are a follow-up block kind.
- `Image`'s `source` must accept an expression (`b[1]`). XAML already does,
  and the other backends need the same.

## 4. Order

1. Compose `Image` (the lane Engram, Journal and Trestle are driven on in
   CI), with photo-picker-app showing its picked photo as the first consumer.
2. Engram card blocks on Compose: images, then LaTeX Tier 1.
3. SwiftUI, Flutter, XAML, then verify Qt.

## 5. What this does not decide

- Remote images on native hosts, caching, animated images.
- Cost: large decks re-send `data:` bytes on each update. A media-id plus
  host-cache scheme is a later optimisation, if renders show it matters.
- MathJax typesetting (`engram-latex-rendering.md` Tier 2).

## 6. Decisions for the owner

1. **`data:` URIs as the portable `image` value** (§3.1)?
2. **Blocks built by the app** (§3.3), instead of an HTML-subset primitive
   rendered by each host?
3. **Order** (§4): Compose and photo-picker first?
