// DeckStatsPanel.mll - Mosaic layout for deck statistics.

layout DeckStatsPanel {
  Column [ deck-stats-panel ] {
    Text [ deck-stats-label ] (
      content : slot: deck-label
    )
    Text [ deck-stats-name ] (
      content : slot: deck-name
    )
    Text [ deck-list-label ] (
      content : slot: deck-list-label
    )
    Row [ deck-list-row ] {
      For ( each: slot: deck-names , as: deck-option , index: i ) {
        HostButton [ deck-option-button ] (
          label : deck-option ,
          onClick : emit: onSelectDeck
        )
      }
    }
    Row [ deck-stats-grid ] {
      Box [ deck-stat-total ] {
        Text [ deck-total-value ] (
          content : slot: total-value
        )
        Text [ deck-total-label ] (
          content : slot: total-label
        )
      }
      Box [ deck-stat-new ] {
        Text [ deck-new-value ] (
          content : slot: new-value
        )
        Text [ deck-new-label ] (
          content : slot: new-label
        )
      }
      Box [ deck-stat-due ] {
        Text [ deck-due-value ] (
          content : slot: due-value
        )
        Text [ deck-due-label ] (
          content : slot: due-label
        )
      }
      Box [ deck-stat-learning ] {
        Text [ deck-learning-value ] (
          content : slot: learning-value
        )
        Text [ deck-learning-label ] (
          content : slot: learning-label
        )
      }
      Box [ deck-stat-hidden ] {
        Text [ deck-hidden-value ] (
          content : slot: hidden-value
        )
        Text [ deck-hidden-label ] (
          content : slot: hidden-label
        )
      }
    }
  }
}
