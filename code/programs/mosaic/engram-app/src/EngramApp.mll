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
    Box [ deck-options-region ] {
      pkg::mosaic-pkg-deck-options::DeckOptionsPanel (
        settings-label : slot: deck-options-settings-label ,
        learning-steps-label : slot: deck-options-learning-steps-label ,
        learning-steps-value : slot: deck-options-learning-steps-value ,
        relearning-steps-label : slot: deck-options-relearning-steps-label ,
        relearning-steps-value : slot: deck-options-relearning-steps-value ,
        new-cards-label : slot: deck-options-new-cards-label ,
        new-cards-value : slot: deck-options-new-cards-value ,
        reviews-label : slot: deck-options-reviews-label ,
        reviews-value : slot: deck-options-reviews-value ,
        graduating-interval-label : slot: deck-options-graduating-interval-label ,
        graduating-interval-value : slot: deck-options-graduating-interval-value ,
        easy-interval-label : slot: deck-options-easy-interval-label ,
        easy-interval-value : slot: deck-options-easy-interval-value ,
        maximum-interval-label : slot: deck-options-maximum-interval-label ,
        maximum-interval-value : slot: deck-options-maximum-interval-value ,
        interval-modifier-label : slot: deck-options-interval-modifier-label ,
        interval-modifier-value : slot: deck-options-interval-modifier-value ,
        hard-multiplier-label : slot: deck-options-hard-multiplier-label ,
        hard-multiplier-value : slot: deck-options-hard-multiplier-value ,
        easy-bonus-label : slot: deck-options-easy-bonus-label ,
        easy-bonus-value : slot: deck-options-easy-bonus-value ,
        lapse-multiplier-label : slot: deck-options-lapse-multiplier-label ,
        lapse-multiplier-value : slot: deck-options-lapse-multiplier-value ,
        onLearningStepsChange : emit: onDeckOptionsLearningStepsChange ,
        onRelearningStepsChange : emit: onDeckOptionsRelearningStepsChange ,
        onNewCardsChange : emit: onDeckOptionsNewCardsChange ,
        onReviewsChange : emit: onDeckOptionsReviewsChange ,
        onGraduatingIntervalChange : emit: onDeckOptionsGraduatingIntervalChange ,
        onEasyIntervalChange : emit: onDeckOptionsEasyIntervalChange ,
        onMaximumIntervalChange : emit: onDeckOptionsMaximumIntervalChange ,
        onIntervalModifierChange : emit: onDeckOptionsIntervalModifierChange ,
        onHardMultiplierChange : emit: onDeckOptionsHardMultiplierChange ,
        onEasyBonusChange : emit: onDeckOptionsEasyBonusChange ,
        onLapseMultiplierChange : emit: onDeckOptionsLapseMultiplierChange
      )
    }
    Box [ review-history-region ] {
      pkg::mosaic-pkg-review-history::ReviewHistoryPanel (
        history-label : slot: history-label ,
        window-label : slot: history-window-label ,
        total-label : slot: history-total-label ,
        total-value : slot: history-total-value ,
        correct-label : slot: history-correct-label ,
        correct-value : slot: history-correct-value ,
        unique-label : slot: history-unique-label ,
        unique-value : slot: history-unique-value ,
        accuracy-label : slot: history-accuracy-label ,
        accuracy-value : slot: history-accuracy-value ,
        again-label : slot: history-again-label ,
        again-value : slot: history-again-value ,
        hard-label : slot: history-hard-label ,
        hard-value : slot: history-hard-value ,
        good-label : slot: history-good-label ,
        good-value : slot: history-good-value ,
        easy-label : slot: history-easy-label ,
        easy-value : slot: history-easy-value ,
        first-label : slot: history-first-label ,
        first-value : slot: history-first-value ,
        last-label : slot: history-last-label ,
        last-value : slot: history-last-value
      )
    }
    Box [ collection-region ] {
      pkg::mosaic-pkg-collection-actions::CollectionActions (
        collection-label : slot: collection-label ,
        note-count-label : slot: collection-note-count-label ,
        note-count-value : slot: collection-note-count-value ,
        note-type-count-label : slot: collection-note-type-count-label ,
        note-type-count-value : slot: collection-note-type-count-value ,
        media-count-label : slot: collection-media-count-label ,
        media-count-value : slot: collection-media-count-value ,
        import-label : slot: collection-import-label ,
        export-label : slot: collection-export-label ,
        add-note-label : slot: collection-add-note-label ,
        add-note-type-label : slot: collection-add-note-type-label ,
        delete-note-label : slot: collection-delete-note-label ,
        delete-note-type-label : slot: collection-delete-note-type-label ,
        onImportAnki : emit: onImportAnki ,
        onExportAnki : emit: onExportAnki ,
        onAddNote : emit: onAddNote ,
        onAddNoteType : emit: onAddNoteType ,
        onDeleteNote : emit: onDeleteNote ,
        onDeleteNoteType : emit: onDeleteNoteType
      )
    }
    Box [ browser-region ] {
      pkg::mosaic-pkg-card-browser::CardBrowser (
        browser-label : slot: browser-label ,
        query-label : slot: browser-query-label ,
        query : slot: browser-query ,
        query-placeholder : slot: browser-query-placeholder ,
        search-label : slot: browser-search-label ,
        results-label : slot: browser-results-label ,
        results-summary : slot: browser-results-summary ,
        results : slot: browser-results ,
        result-card-ids : slot: browser-result-card-ids ,
        result-note-ids : slot: browser-result-note-ids ,
        result-template-ids : slot: browser-result-template-ids ,
        result-states : slot: browser-result-states ,
        selected-index : slot: browser-selected-index ,
        selected-card-id : slot: browser-selected-card-id ,
        selected-note-id : slot: browser-selected-note-id ,
        selected-template-id : slot: browser-selected-template-id ,
        selected-state : slot: browser-selected-state ,
        open-label : slot: browser-open-label ,
        edit-label : slot: browser-edit-label ,
        suspend-label : slot: browser-suspend-label ,
        mark-label : slot: browser-mark-label ,
        onQueryChange : emit: onBrowserQueryChange ,
        onSearch : emit: onBrowserSearch ,
        onSelectResult : emit: onBrowserSelectResult ,
        onOpenSelected : emit: onBrowserOpenSelected ,
        onEditSelected : emit: onBrowserEditSelected ,
        onToggleSuspendSelected : emit: onBrowserToggleSuspendSelected ,
        onToggleMarkSelected : emit: onBrowserToggleMarkSelected
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
