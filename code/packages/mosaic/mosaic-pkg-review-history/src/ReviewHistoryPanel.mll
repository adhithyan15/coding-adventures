// ReviewHistoryPanel.mll - target-neutral review-history summary layout.

layout ReviewHistoryPanel {
  Column [ review-history-panel ] {
    Text [ history-title ] (
      content : slot: history-label
    )
    Text [ history-window ] (
      content : slot: window-label
    )
    Row [ history-summary-grid ] {
      Column [ history-total ] {
        Text [ history-total-value ] (
          content : slot: total-value
        )
        Text [ history-total-label ] (
          content : slot: total-label
        )
      }
      Column [ history-correct ] {
        Text [ history-correct-value ] (
          content : slot: correct-value
        )
        Text [ history-correct-label ] (
          content : slot: correct-label
        )
      }
      Column [ history-unique ] {
        Text [ history-unique-value ] (
          content : slot: unique-value
        )
        Text [ history-unique-label ] (
          content : slot: unique-label
        )
      }
      Column [ history-accuracy ] {
        Text [ history-accuracy-value ] (
          content : slot: accuracy-value
        )
        Text [ history-accuracy-label ] (
          content : slot: accuracy-label
        )
      }
    }
    Row [ rating-summary-grid ] {
      Column [ history-rating-again ] {
        Text [ history-again-value ] (
          content : slot: again-value
        )
        Text [ history-again-label ] (
          content : slot: again-label
        )
      }
      Column [ history-rating-hard ] {
        Text [ history-hard-value ] (
          content : slot: hard-value
        )
        Text [ history-hard-label ] (
          content : slot: hard-label
        )
      }
      Column [ history-rating-good ] {
        Text [ history-good-value ] (
          content : slot: good-value
        )
        Text [ history-good-label ] (
          content : slot: good-label
        )
      }
      Column [ history-rating-easy ] {
        Text [ history-easy-value ] (
          content : slot: easy-value
        )
        Text [ history-easy-label ] (
          content : slot: easy-label
        )
      }
    }
    Row [ history-time-range ] {
      Column [ history-first ] {
        Text [ history-first-label ] (
          content : slot: first-label
        )
        Text [ history-first-value ] (
          content : slot: first-value
        )
      }
      Column [ history-last ] {
        Text [ history-last-label ] (
          content : slot: last-label
        )
        Text [ history-last-value ] (
          content : slot: last-value
        )
      }
    }
  }
}
