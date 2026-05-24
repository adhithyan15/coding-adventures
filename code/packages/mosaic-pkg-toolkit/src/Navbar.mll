// Navbar.mll — layout for the Navbar (v0.10).
//
//   Row [ navbar ]
//     Text [ navbar-brand ] (content: slot: brand)
//     For (each: slot: items, as: item, index: i)
//       HostLink [ navbar-link ] (
//         href: "#",
//         label: item,
//         external: false,
//         onActivate: emit: onSelect
//       )
//
// Brand-on-left, links-after. The .msl handles spacing via margin/
// padding on navbar-brand. Spacer-as-an-explicit-element (push
// links to the right) awaits a kernel Spacer-with-grow primitive
// that v1 doesn't yet expose.
//
// Like Nav and Breadcrumb (v0.4), each link wraps the UI29-4
// HostLink primitive — platform-native role="link" semantics
// + Tab/Enter keyboard activation come from the kernel.
// `external: false` + `href: "#"` mirror the Nav/Breadcrumb routing
// convention (host routes via onActivate(index)).

layout Navbar {
  Row [ navbar ] {
    Text [ navbar-brand ] (
      content : slot: brand
    )
    For ( each: slot: items , as: item , index: i ) {
      HostLink [ navbar-link ] (
        href : "#" ,
        external : false ,
        label : item ,
        onActivate : emit: onSelect
      )
    }
  }
}
