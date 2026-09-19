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
