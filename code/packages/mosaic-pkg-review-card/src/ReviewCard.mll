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
    If ( when: slot: type-answer-active ) {
      Box [ type-answer-panel ] {
        Text [ type-answer-label ] (
          content : slot: type-answer-label
        )
        HostInput [ type-answer-input ] (
          value : slot: type-answer-value ,
          placeholder : slot: type-answer-placeholder ,
          onChange : emit: onTypeAnswerChange
        )
        If ( when: slot: answer-visible ) {
          Text [ type-answer-comparison-label ] (
            content : slot: type-answer-comparison-label
          )
          Text [ type-answer-comparison-value ] (
            content : slot: type-answer-comparison-value
          )
        }
      }
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
      pkg::mosaic-pkg-rating-controls::RatingControls (
        again-label : "Again" ,
        hard-label : "Hard" ,
        good-label : "Good" ,
        easy-label : "Easy" ,
        onAgain : emit: onAgain ,
        onHard : emit: onHard ,
        onGood : emit: onGood ,
        onEasy : emit: onEasy
      )
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
