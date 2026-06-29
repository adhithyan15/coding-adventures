// ReviewActions.mll - Mosaic layout for secondary review actions.

layout ReviewActions {
  Row [ review-actions-row ] {
    HostButton [ action-undo ] (
      label : slot: undo-label ,
      onClick : emit: onUndo
    )
    HostButton [ action-bury-card ] (
      label : slot: bury-card-label ,
      onClick : emit: onBuryCard
    )
    HostButton [ action-bury-siblings ] (
      label : slot: bury-siblings-label ,
      onClick : emit: onBurySiblings
    )
    HostButton [ action-suspend-card ] (
      label : slot: suspend-card-label ,
      onClick : emit: onSuspendCard
    )
    HostButton [ action-toggle-mark ] (
      label : slot: mark-label ,
      onClick : emit: onToggleMark
    )
  }
}
