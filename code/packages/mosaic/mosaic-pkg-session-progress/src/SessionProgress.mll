// SessionProgress.mll - Mosaic layout for review-session counters.

layout SessionProgress {
  Row [ progress-strip ] {
    Column [ metric-current ] {
      Text [ current-label ] (
        content : slot: current-label
      )
      Text [ current-value ] (
        content : slot: current-value
      )
    }
    Column [ metric-remaining ] {
      Text [ remaining-label ] (
        content : slot: remaining-label
      )
      Text [ remaining-value ] (
        content : slot: remaining-value
      )
    }
    Column [ metric-correct ] {
      Text [ correct-label ] (
        content : slot: correct-label
      )
      Text [ correct-value ] (
        content : slot: correct-value
      )
    }
    Column [ metric-total ] {
      Text [ total-label ] (
        content : slot: total-label
      )
      Text [ total-value ] (
        content : slot: total-value
      )
    }
  }
}
