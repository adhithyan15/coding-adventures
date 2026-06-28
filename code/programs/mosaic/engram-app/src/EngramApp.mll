// EngramApp.mll - product app shell assembled from component packages.

layout EngramApp {
  Column [ app-shell ] {
    Text [ app-title ] (
      content : slot: app-title
    )
    Box [ stats-region ] {
      pkg::mosaic-pkg-deck-stats::DeckStatsPanel (
        deck-label : slot: deck-stats-label ,
        deck-name : slot: deck-name ,
        total-label : slot: deck-total-label ,
        total-value : slot: deck-total-value ,
        new-label : slot: deck-new-label ,
        new-value : slot: deck-new-value ,
        due-label : slot: deck-due-label ,
        due-value : slot: deck-due-value ,
        learning-label : slot: deck-learning-label ,
        learning-value : slot: deck-learning-value ,
        hidden-label : slot: deck-hidden-label ,
        hidden-value : slot: deck-hidden-value
      )
    }
    pkg::mosaic-pkg-session-progress::SessionProgress (
      current-label : slot: current-label ,
      current-value : slot: current-value ,
      remaining-label : slot: remaining-label ,
      remaining-value : slot: remaining-value ,
      correct-label : slot: correct-label ,
      correct-value : slot: correct-value ,
      total-label : slot: total-label ,
      total-value : slot: total-value
    )
    Box [ review-region ] {
      pkg::mosaic-pkg-review-card::ReviewCard (
        deck-name : slot: deck-name ,
        prompt-label : slot: prompt-label ,
        prompt : slot: prompt ,
        answer-label : slot: answer-label ,
        answer : slot: answer ,
        answer-visible : slot: answer-visible ,
        progress-label : slot: progress-label ,
        onReveal : emit: onReveal ,
        onAgain : emit: onAgain ,
        onHard : emit: onHard ,
        onGood : emit: onGood ,
        onEasy : emit: onEasy
      )
    }
    pkg::mosaic-pkg-review-actions::ReviewActions (
      undo-label : slot: action-undo-label ,
      bury-card-label : slot: action-bury-card-label ,
      bury-siblings-label : slot: action-bury-siblings-label ,
      suspend-card-label : slot: action-suspend-card-label ,
      mark-label : slot: action-mark-label ,
      onUndo : emit: onUndo ,
      onBuryCard : emit: onBuryCard ,
      onBurySiblings : emit: onBurySiblings ,
      onSuspendCard : emit: onSuspendCard ,
      onToggleMark : emit: onToggleMark
    )
  }
}
