layout NavigationShell {
  HostNavigationSplit [ shell ] (
    pane-title: "Projects",
    pane-width: 236,
    collapse: auto
  ) {
    Column [ pane ] {
      Text ( content: "Project navigation" )
    }
    Column [ detail ] {
      Text ( content: "Project detail" )
    }
  }
}
