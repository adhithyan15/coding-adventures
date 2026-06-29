// DeckOptionsPanel.mll - target-neutral deck option settings layout.

layout DeckOptionsPanel {
  Column [ deck-options-panel ] {
    Text [ settings-title ] (
      content : slot: settings-label
    )
    Row [ learning-step-options ] {
      Column [ learning-steps-field ] {
        Text [ learning-steps-label ] (
          content : slot: learning-steps-label
        )
        HostInput [ learning-steps-input ] (
          value : slot: learning-steps-value ,
          placeholder : "1, 10" ,
          disabled : false ,
          onChange : emit: onLearningStepsChange
        )
      }
      Column [ relearning-steps-field ] {
        Text [ relearning-steps-label ] (
          content : slot: relearning-steps-label
        )
        HostInput [ relearning-steps-input ] (
          value : slot: relearning-steps-value ,
          placeholder : "10" ,
          disabled : false ,
          onChange : emit: onRelearningStepsChange
        )
      }
    }
    Row [ deck-option-limits ] {
      Column [ new-cards-field ] {
        Text [ new-cards-label ] (
          content : slot: new-cards-label
        )
        HostNumberInput [ new-cards-input ] (
          value : slot: new-cards-value ,
          placeholder : "20" ,
          disabled : false ,
          onChange : emit: onNewCardsChange
        )
      }
      Column [ reviews-field ] {
        Text [ reviews-label ] (
          content : slot: reviews-label
        )
        HostNumberInput [ reviews-input ] (
          value : slot: reviews-value ,
          placeholder : "200" ,
          disabled : false ,
          onChange : emit: onReviewsChange
        )
      }
    }
    Row [ graduation-options ] {
      Column [ graduating-interval-field ] {
        Text [ graduating-interval-label ] (
          content : slot: graduating-interval-label
        )
        HostNumberInput [ graduating-interval-input ] (
          value : slot: graduating-interval-value ,
          placeholder : "1" ,
          disabled : false ,
          onChange : emit: onGraduatingIntervalChange
        )
      }
      Column [ easy-interval-field ] {
        Text [ easy-interval-label ] (
          content : slot: easy-interval-label
        )
        HostNumberInput [ easy-interval-input ] (
          value : slot: easy-interval-value ,
          placeholder : "4" ,
          disabled : false ,
          onChange : emit: onEasyIntervalChange
        )
      }
      Column [ maximum-interval-field ] {
        Text [ maximum-interval-label ] (
          content : slot: maximum-interval-label
        )
        HostNumberInput [ maximum-interval-input ] (
          value : slot: maximum-interval-value ,
          placeholder : "36500" ,
          disabled : false ,
          onChange : emit: onMaximumIntervalChange
        )
      }
    }
    Row [ review-factor-options ] {
      Column [ initial-ease-field ] {
        Text [ initial-ease-label ] (
          content : slot: initial-ease-label
        )
        HostNumberInput [ initial-ease-input ] (
          value : slot: initial-ease-value ,
          placeholder : "2.5" ,
          disabled : false ,
          onChange : emit: onInitialEaseChange
        )
      }
      Column [ interval-modifier-field ] {
        Text [ interval-modifier-label ] (
          content : slot: interval-modifier-label
        )
        HostNumberInput [ interval-modifier-input ] (
          value : slot: interval-modifier-value ,
          placeholder : "1.0" ,
          disabled : false ,
          onChange : emit: onIntervalModifierChange
        )
      }
      Column [ hard-multiplier-field ] {
        Text [ hard-multiplier-label ] (
          content : slot: hard-multiplier-label
        )
        HostNumberInput [ hard-multiplier-input ] (
          value : slot: hard-multiplier-value ,
          placeholder : "1.2" ,
          disabled : false ,
          onChange : emit: onHardMultiplierChange
        )
      }
      Column [ easy-bonus-field ] {
        Text [ easy-bonus-label ] (
          content : slot: easy-bonus-label
        )
        HostNumberInput [ easy-bonus-input ] (
          value : slot: easy-bonus-value ,
          placeholder : "1.3" ,
          disabled : false ,
          onChange : emit: onEasyBonusChange
        )
      }
      Column [ lapse-multiplier-field ] {
        Text [ lapse-multiplier-label ] (
          content : slot: lapse-multiplier-label
        )
        HostNumberInput [ lapse-multiplier-input ] (
          value : slot: lapse-multiplier-value ,
          placeholder : "0.0" ,
          disabled : false ,
          onChange : emit: onLapseMultiplierChange
        )
      }
    }
    Row [ fsrs-options ] {
      Column [ desired-retention-field ] {
        Text [ desired-retention-label ] (
          content : slot: desired-retention-label
        )
        HostNumberInput [ desired-retention-input ] (
          value : slot: desired-retention-value ,
          placeholder : "0.90" ,
          disabled : false ,
          onChange : emit: onDesiredRetentionChange
        )
      }
      Column [ historical-retention-field ] {
        Text [ historical-retention-label ] (
          content : slot: historical-retention-label
        )
        HostNumberInput [ historical-retention-input ] (
          value : slot: historical-retention-value ,
          placeholder : "0.90" ,
          disabled : false ,
          onChange : emit: onHistoricalRetentionChange
        )
      }
      Column [ fsrs-parameters-field ] {
        Text [ fsrs-parameters-label ] (
          content : slot: fsrs-parameters-label
        )
        HostInput [ fsrs-parameters-input ] (
          value : slot: fsrs-parameters-value ,
          placeholder : "0.1, 1.2, 2.3" ,
          disabled : false ,
          onChange : emit: onFsrsParametersChange
        )
      }
    }
    Row [ fsrs-filter-options ] {
      Column [ fsrs-search-field ] {
        Text [ fsrs-search-label ] (
          content : slot: fsrs-search-label
        )
        HostInput [ fsrs-search-input ] (
          value : slot: fsrs-search-value ,
          placeholder : "preset:\"Default\" -is:suspended" ,
          disabled : false ,
          onChange : emit: onFsrsSearchChange
        )
      }
      Column [ ignore-review-history-before-field ] {
        Text [ ignore-review-history-before-label ] (
          content : slot: ignore-review-history-before-label
        )
        HostInput [ ignore-review-history-before-input ] (
          value : slot: ignore-review-history-before-value ,
          placeholder : "2024-01-02" ,
          disabled : false ,
          onChange : emit: onIgnoreReviewHistoryBeforeChange
        )
      }
      Column [ easy-days-percentages-field ] {
        Text [ easy-days-percentages-label ] (
          content : slot: easy-days-percentages-label
        )
        HostInput [ easy-days-percentages-input ] (
          value : slot: easy-days-percentages-value ,
          placeholder : "1, 1, 1, 1, 1, 1, 1" ,
          disabled : false ,
          onChange : emit: onEasyDaysPercentagesChange
        )
      }
    }
    Row [ leech-options ] {
      Column [ leech-threshold-field ] {
        Text [ leech-threshold-label ] (
          content : slot: leech-threshold-label
        )
        HostNumberInput [ leech-threshold-input ] (
          value : slot: leech-threshold-value ,
          placeholder : "8" ,
          disabled : false ,
          onChange : emit: onLeechThresholdChange
        )
      }
      Column [ leech-action-field ] {
        Text [ leech-action-label ] (
          content : slot: leech-action-label
        )
        Row [ leech-action-choices ] {
          HostRadio [ leech-action-suspend-radio ] (
            label : slot: leech-action-suspend-label ,
            checked : slot: leech-action-suspend-value ,
            value : "suspend" ,
            group : "deck-options-leech-action" ,
            disabled : false ,
            onSelect : emit: onLeechActionChange
          )
          HostRadio [ leech-action-tag-only-radio ] (
            label : slot: leech-action-tag-only-label ,
            checked : slot: leech-action-tag-only-value ,
            value : "tag-only" ,
            group : "deck-options-leech-action" ,
            disabled : false ,
            onSelect : emit: onLeechActionChange
          )
        }
      }
    }
    Row [ sibling-bury-options ] {
      Column [ bury-new-siblings-field ] {
        HostCheckbox [ bury-new-siblings-checkbox ] (
          label : slot: bury-new-siblings-label ,
          checked : slot: bury-new-siblings-value ,
          disabled : false ,
          indeterminate : false ,
          onToggle : emit: onBuryNewSiblingsChange
        )
      }
      Column [ bury-review-siblings-field ] {
        HostCheckbox [ bury-review-siblings-checkbox ] (
          label : slot: bury-review-siblings-label ,
          checked : slot: bury-review-siblings-value ,
          disabled : false ,
          indeterminate : false ,
          onToggle : emit: onBuryReviewSiblingsChange
        )
      }
      Column [ bury-interday-learning-siblings-field ] {
        HostCheckbox [ bury-interday-learning-siblings-checkbox ] (
          label : slot: bury-interday-learning-siblings-label ,
          checked : slot: bury-interday-learning-siblings-value ,
          disabled : false ,
          indeterminate : false ,
          onToggle : emit: onBuryInterdayLearningSiblingsChange
        )
      }
    }
  }
}
