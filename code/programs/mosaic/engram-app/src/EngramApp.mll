// EngramApp.mll - product app shell assembled from component packages.

layout EngramApp {
  Column [ app-shell ] {
    Text [ app-title ] (
      content : slot: app-title
    )
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
  }
}
