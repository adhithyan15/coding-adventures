layout PinnedNavigationShell {
  HostNavigationSplit [ shell ] (
    pane-title: "Workbench",
    pane-width: 236,
    collapse: never
  ) {
    Column [ pane ] {
      Text ( content: "Pinned navigation" )
    }
    Column [ detail ] {
      Text ( content: "Workbench detail" )
    }
  }
}
