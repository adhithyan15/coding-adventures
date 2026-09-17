// The outer split is a workbench whose pane IS the content, so it pins
// itself open (`collapse : never`) and takes its name from a slot. The
// inner one is the ordinary adaptive case: a literal title, a preferred
// pane width, and the default `auto` collapse. Nesting them is not
// contrived — it is the shape a workbench with its own list/detail area
// has — and it makes the XAML compiler check a NavigationView living
// inside another NavigationView's content.
layout Shell {
  HostNavigationSplit [ workbench ] (
    pane-title : slot: workspace-name ,
    collapse : never
  ) {
    Column [ workbench-pane ] {
      HostButton [ inbox ] (
        label : slot: inbox-label ,
        onClick : emit: onOpenInbox
      )
    }
    HostNavigationSplit [ app-shell ] (
      pane-title : "Projects" ,
      pane-width : 236 ,
      collapse : auto
    ) {
      Column [ rail ] {
        Text [ rail-heading ] ( content : "Projects" )
      }
      Column [ main ] {
        Text [ detail-heading ] ( content : "Detail" )
      }
    }
  }
}
