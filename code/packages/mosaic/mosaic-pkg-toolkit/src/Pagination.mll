// Pagination.mll — layout for the Pagination control (v0.5).
//
//   Row [ pagination ]
//     HostLink [ pagination-prev ] (
//       href: "#", external: false,
//       label: slot: prev-label, onActivate: emit: onPrev
//     )
//     For (each: slot: pages, as: page, index: i)
//       HostLink [ pagination-page ] (
//         href: "#", external: false,
//         label: page, onActivate: emit: onPageSelect
//       )
//     HostLink [ pagination-next ] (
//       href: "#", external: false,
//       label: slot: next-label, onActivate: emit: onNext
//     )
//
// All three chip families (prev / pages / next) share the same Row.
// The .msl flat-styles them as connected button-styled anchors with
// shared borders.
//
// `external: false` + `href: "#"`: matches the Nav/Breadcrumb
// convention — toolkit owns layout, host routes via the onActivate
// payload. The inert href keeps right-click → Copy Link and middle-
// click → open-in-new-tab from doing useless things; a future PR
// could thread per-page hrefs through a list<text> slot.

layout Pagination {
  Row [ pagination ] {
    HostLink [ pagination-prev ] (
      href : "#" ,
      external : false ,
      label : slot: prev-label ,
      onActivate : emit: onPrev
    )
    For ( each: slot: pages , as: page , index: i ) {
      HostLink [ pagination-page ] (
        href : "#" ,
        external : false ,
        label : page ,
        onActivate : emit: onPageSelect
      )
    }
    HostLink [ pagination-next ] (
      href : "#" ,
      external : false ,
      label : slot: next-label ,
      onActivate : emit: onNext
    )
  }
}
