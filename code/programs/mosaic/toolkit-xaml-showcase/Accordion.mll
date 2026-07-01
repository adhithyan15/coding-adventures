// Accordion.mll — layout for the Accordion (v0.7).
//
//   Column [ accordion ]
//     For (each: slot: headers, as: header, index: i)
//       Column [ accordion-item ]
//         HostButton [ accordion-header ] (label: header, onClick: emit: onToggle)
//         Text [ accordion-body ] (content: slot: bodies[i])
//
// v0.7 limitation — bodies always render:
// ---------------------------------------
// A proper Accordion shows body i only when `open-index == i`. That
// needs an `If (when: <index> == <slot>)` expression — comparison
// inside `If`'s `when` clause hasn't shipped in UI29's expression
// language yet (every .mll in the toolkit so far uses truthiness
// only — see Toast.mll's `If (when: slot: open)`).
//
// Until that lands, the host simulates the open/close by clearing
// `bodies[i]` to an empty string when the panel should be closed.
// Empty body text + the .msl's zero padding-on-empty produces a
// closed-looking row. Hacky but unblocks v0.7; a follow-up will
// swap in the proper `If` expression once the kernel supports it.
//
// `open-index` and `onToggle` are still part of the .mil so callers'
// glue code stays forward-compatible when the proper expression
// support lands.

layout Accordion {
  Column [ accordion ] {
    For ( each: slot: headers , as: header , index: i ) {
      Column [ accordion-item ] {
        HostButton [ accordion-header ] (
          label : header ,
          onClick : emit: onToggle
        )
        Text [ accordion-body ] (
          content : slot: bodies
        )
      }
    }
  }
}
