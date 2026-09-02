# UI46: Generated Content and Marker Layout

## Status

Implemented by `layout-generated` and consumed through shared Layout IR.

## Contract

Generated `::before`, `::after`, and `::marker` content is represented as an
ordinary text `LayoutNode`. Its `ext["generated"]` map records `kind`, marker
`position`, `markerGap`, and `semanticOwner: false`. The originating element
retains DOM identity, link behavior, and accessibility ownership.

`layout-generated` owns scoped CSS counters, decimal/alphabetic/Roman/symbolic
counter formatting, generated-content evaluation, extension decoding, and
diagnostics. Producers may supply attributes and authored counter operations;
layout and paint remain independent from HTML and CSS.

Outside markers use an inline gutter: the marker is positioned before the
content edge without reducing the content line width. Inside markers remain in
the ordinary inline run. Paint and hit testing consume both as regular text.

## Invariants

- Counter arithmetic saturates and missing counters resolve to zero.
- Invalid styles fall back at the producer boundary and never reach hosts.
- Generated nodes do not receive the originating DOM id or navigation target.
- Unknown extension values are ignored, with deterministic diagnostics.
