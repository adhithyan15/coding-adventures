// Nav.mll — layout for the horizontal nav (v0.4).
//
//   Row [ nav ]
//     For (each: slot: items, as: item, index: i)
//       If (when: i == activeIndex)
//         HostLink [ nav-link-active ] (...)
//       Else
//         HostLink [ nav-link ] (...)
//
// v0.4 swaps the per-item HostButton for HostLink — same rationale
// as Breadcrumb's rewrite. The "nav as a row of links" idiom is
// what the DOM `<nav>` semantic element wraps in real-world HTML,
// and the cross-platform native widgets (SwiftUI's `Link`, XAML's
// `Hyperlink`, etc.) all model the same shape. Switching to
// HostLink also brings free keyboard semantics (Tab + Enter to
// activate) the prior HostButton lowering depended on a per-
// backend onClick spelling for.
//
// `external: false` + `href: "#"` is the same pattern as
// Breadcrumb — toolkit owns the layout, host owns routing via
// `onActivate(index)`. active-index remains the public kebab-case
// slot; predicates use activeIndex because emitters expose kebab
// slots as backend-safe camel/Pascal identifiers.

layout Nav {
  Row [ nav ] {
    For ( each: slot: items , as: item , index: i ) {
      If ( when: i == activeIndex ) {
        HostLink [ nav-link-active ] (
          href : "#" ,
          label : item ,
          external : false ,
          onActivate : emit: onSelect
        )
      }
      Else {
        HostLink [ nav-link ] (
          href : "#" ,
          label : item ,
          external : false ,
          onActivate : emit: onSelect
        )
      }
    }
  }
}
