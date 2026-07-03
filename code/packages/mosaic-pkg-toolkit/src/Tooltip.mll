// Tooltip.mll — layout for the toolkit Tooltip (v0.4).
//
//   HostTooltip [ tooltip-wrapper ] ( text: slot: message ) {
//     Text [ tooltip-label ] ( content: slot: label )
//   }
//
// HostTooltip wraps its single child (the spec's `target`); we
// wrap a `Text` widget so the visible label inherits toolkit
// font-family / size defaults. Each backend then maps to its
// native tooltip chrome (DOM `title=` attribute, SwiftUI `.help`,
// Qt `ToolTip.text`, XAML `ToolTipService.ToolTip`, Flutter
// `Tooltip(message:, child:)`).
//
// The kernel HostTooltip prop is `text` (per UI29-4 spec §3.2);
// the toolkit slot is `message` because `text` is a grammar
// keyword in .mil files. The mapping `text: slot: message`
// connects the kernel-side `text` prop to the toolkit-side
// `message` slot.

layout Tooltip {
  HostTooltip [ tooltip-wrapper ] (
    text : slot: message
  ) {
    Text [ tooltip-label ] (
      content : slot: label
    )
  }
}
