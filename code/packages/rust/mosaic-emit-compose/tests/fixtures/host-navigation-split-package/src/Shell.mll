layout Shell {
  HostNavigationSplit [ app-shell ] (
    pane-title: "Projects",
    pane-width: 236,
    collapse: auto
  ) {
    Column [ pane ] {
      Text ( content: "Projects" )
    }
    Column [ detail ] {
      Text ( content: "Select a project" )
    }
  }
}
