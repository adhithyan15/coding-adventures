// ReviewCard.mll - Mosaic layout for one review card.

layout ReviewCard {
  Column [ review-card ] {
    Text [ deck-name ] (
      content : slot: deck-name
    )
    Box [ prompt-panel ] {
      Text [ prompt-label ] (
        content : slot: prompt-label
      )
      Text [ prompt-text ] (
        content : slot: prompt
      )
    }
    If ( when: slot: answer-visible ) {
      Box [ answer-panel ] {
        Text [ answer-label ] (
          content : slot: answer-label
        )
        Text [ answer-text ] (
          content : slot: answer
        )
      }
      Row [ rating-row ] {
        HostButton [ rating-again ] (
          label : "Again" ,
          onClick : emit: onAgain
        )
        HostButton [ rating-hard ] (
          label : "Hard" ,
          onClick : emit: onHard
        )
        HostButton [ rating-good ] (
          label : "Good" ,
          onClick : emit: onGood
        )
        HostButton [ rating-easy ] (
          label : "Easy" ,
          onClick : emit: onEasy
        )
      }
    }
    Else {
      HostButton [ reveal-button ] (
        label : "Reveal answer" ,
        onClick : emit: onReveal
      )
    }
    Text [ progress-label ] (
      content : slot: progress-label
    )
  }
}
