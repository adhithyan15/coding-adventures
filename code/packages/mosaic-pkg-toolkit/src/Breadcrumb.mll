// Breadcrumb.mll — layout for the Breadcrumb (v0.4).
//
//   Row [ breadcrumb ]
//     For (each: slot: crumbs, as: crumb, index: i)
//       HostLink [ breadcrumb-link ] (
//         href: "#",
//         label: crumb,
//         external: false,
//         onActivate: emit: onSelect
//       )
//
// v0.4 swaps the per-crumb HostButton for HostLink, the UI29-4
// kernel primitive promoted from the toolkit. The platform-correct
// a11y semantics (role="link" on web, Link on SwiftUI/Flutter,
// rich-text anchor on Qt, Hyperlink on XAML) come from the kernel
// for free now — no per-backend chrome in the toolkit.
//
// href default `"#"`: the toolkit doesn't know the host's routing
// scheme, so we ship an inert anchor and let the host's
// `onActivate` handler resolve the destination from the crumb
// index. A follow-up could add an `hrefs: list<text>` slot for
// hosts that want true per-crumb URLs (enabling right-click
// → Copy Link, middle-click → open-in-new-tab, etc.); v0.4 keeps
// the interface backwards-compatible.
//
// `external: false` tells the kernel this is in-app navigation
// (no browser-window opening) so the React/Flutter emitters skip
// the `window.open` / `launchUrl` hint and route through
// onActivate instead.

layout Breadcrumb {
  Row [ breadcrumb ] {
    For ( each: slot: crumbs , as: crumb , index: i ) {
      HostLink [ breadcrumb-link ] (
        href : "#" ,
        label : crumb ,
        external : false ,
        onActivate : emit: onSelect
      )
    }
  }
}
