- **Stars (J4b).** A *Star* / *Unstar* button above the editor (hidden for a
  new draft), and a *Starred only* toggle under the search field that narrows
  the timeline and search alike. With the filter on and nothing starred, a
  third `EmptyState` says "No starred entries". The toggle is two parts
  (`starred-filter`, `starred-filter-on`), as in Trestle's outline, so the "on"
  state has its own style on every backend. It uses the amber of the `★` badge.
  New slots: `star-label`, `starred-only`, `no-starred`. New events:
  `onToggleStar`, `onToggleStarredFilter`.
