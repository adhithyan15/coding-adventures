// Breadcrumb.mll — layout for the Breadcrumb.
//
//   Row [ breadcrumb ]
//     For (each: slot: crumbs, as: crumb, index: i)
//       HostButton [ breadcrumb-link ] (label: crumb, onClick: emit: onSelect)
//
// v0.1: every crumb renders as a HostButton; the .msl styles them
// uniformly. The "current location" non-clickable distinction
// (typically the last crumb) needs either a `For` body that
// branches on `i == crumbs.length - 1` or a separate `current`
// slot — both are kernel-doable but add complexity. Punting that
// to a follow-up; v0.1 ships the uniform-clickable form.
//
// Separators (the `/` or `>` between crumbs) are intentionally
// NOT in the .mll for v0.1 either — they'd require a similar
// branching-on-index pattern. The .msl can simulate separators
// via per-link right-margin + a `::after` pseudo-element on
// platforms that support it.

layout Breadcrumb {
  Row [ breadcrumb ] {
    For ( each: slot: crumbs , as: crumb , index: i ) {
      HostButton [ breadcrumb-link ] (
        label : crumb ,
        onClick : emit: onSelect
      )
    }
  }
}
