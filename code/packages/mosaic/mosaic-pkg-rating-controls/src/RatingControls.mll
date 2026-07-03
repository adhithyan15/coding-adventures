// RatingControls.mll - Mosaic layout for answer grading.

layout RatingControls {
  Row [ rating-row ] {
    HostButton [ rating-again ] (
      label : slot: again-label ,
      onClick : emit: onAgain
    )
    HostButton [ rating-hard ] (
      label : slot: hard-label ,
      onClick : emit: onHard
    )
    HostButton [ rating-good ] (
      label : slot: good-label ,
      onClick : emit: onGood
    )
    HostButton [ rating-easy ] (
      label : slot: easy-label ,
      onClick : emit: onEasy
    )
  }
}
