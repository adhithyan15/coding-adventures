layout Shell {
  HostNavigationSplit [ app-shell ] (
    pane-title: slot: pane-title,
    pane-width: 236
  ) {
    Column [ pane ] {
      Text [ pane-label ] ( content: "Projects" )
    }
    Column [ detail ] {
      Text [ detail-label ] ( content: "Selected project" )
    }
  }
}
