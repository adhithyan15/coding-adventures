// EngramApp.mll - product app shell assembled from component packages.

layout EngramApp {
  Column [ app-shell ] {
    Row [ app-header ] {
      Text [ app-title ] (
        content : slot: app-title
      )
      Row [ app-nav ] {
        If ( when: slot: show-decks-screen ) {
          HostButton [ nav-decks-active ] (
            label : "Decks" ,
            onClick : emit: onShowDecks
          )
        }
        Else {
          HostButton [ nav-decks-button ] (
            label : "Decks" ,
            onClick : emit: onShowDecks
          )
        }
        If ( when: slot: show-study-screen ) {
          HostButton [ nav-study-active ] (
            label : "Study" ,
            onClick : emit: onShowStudy
          )
        }
        Else {
          HostButton [ nav-study-button ] (
            label : "Study" ,
            onClick : emit: onShowStudy
          )
        }
        If ( when: slot: show-browse-screen ) {
          HostButton [ nav-browse-active ] (
            label : "Browse" ,
            onClick : emit: onShowBrowse
          )
        }
        Else {
          HostButton [ nav-browse-button ] (
            label : "Browse" ,
            onClick : emit: onShowBrowse
          )
        }
        If ( when: slot: show-add-screen ) {
          HostButton [ nav-add-active ] (
            label : "Add" ,
            onClick : emit: onShowAdd
          )
        }
        Else {
          HostButton [ nav-add-button ] (
            label : "Add" ,
            onClick : emit: onShowAdd
          )
        }
        If ( when: slot: show-stats-screen ) {
          HostButton [ nav-stats-active ] (
            label : "Stats" ,
            onClick : emit: onShowStats
          )
        }
        Else {
          HostButton [ nav-stats-button ] (
            label : "Stats" ,
            onClick : emit: onShowStats
          )
        }
        If ( when: slot: show-options-screen ) {
          HostButton [ nav-options-active ] (
            label : "Options" ,
            onClick : emit: onShowOptions
          )
        }
        Else {
          HostButton [ nav-options-button ] (
            label : "Options" ,
            onClick : emit: onShowOptions
          )
        }
      }
    }

    If ( when: slot: show-decks-screen ) {
      Column [ decks-screen ] {
        Box [ stats-region ] {
          pkg::mosaic-pkg-deck-stats::DeckStatsPanel (
            deck-label : slot: deck-stats-label ,
            deck-name : slot: deck-name ,
            deck-list-label : slot: deck-list-label ,
            deck-names : slot: deck-names ,
            total-label : slot: deck-total-label ,
            total-value : slot: deck-total-value ,
            new-label : slot: deck-new-label ,
            new-value : slot: deck-new-value ,
            due-label : slot: deck-due-label ,
            due-value : slot: deck-due-value ,
            learning-label : slot: deck-learning-label ,
            learning-value : slot: deck-learning-value ,
            hidden-label : slot: deck-hidden-label ,
            hidden-value : slot: deck-hidden-value ,
            onSelectDeck : emit: onSelectDeck
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
            referenced-media-label : slot: collection-referenced-media-label ,
            referenced-media-value : slot: collection-referenced-media-value ,
            missing-media-label : slot: collection-missing-media-label ,
            missing-media-value : slot: collection-missing-media-value ,
            missing-media-filenames : slot: collection-missing-media-filenames ,
            unused-media-label : slot: collection-unused-media-label ,
            unused-media-value : slot: collection-unused-media-value ,
            unused-media-asset-ids : slot: collection-unused-media-asset-ids ,
            prune-unused-media-label : slot: collection-prune-unused-media-label ,
            import-label : slot: collection-import-label ,
            export-label : slot: collection-export-label ,
            add-note-label : slot: collection-add-note-label ,
            add-note-type-label : slot: collection-add-note-type-label ,
            delete-note-label : slot: collection-delete-note-label ,
            delete-note-type-label : slot: collection-delete-note-type-label ,
            onImportAnki : emit: onImportAnki ,
            onExportAnki : emit: onExportAnki ,
            onPruneUnusedMedia : emit: onPruneUnusedMedia ,
            onAddNote : emit: onAddNote ,
            onAddNoteType : emit: onAddNoteType ,
            onDeleteNote : emit: onDeleteNote ,
            onDeleteNoteType : emit: onDeleteNoteType
          )
        }
      }
    }

    If ( when: slot: show-study-screen ) {
      Column [ study-screen ] {
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
            type-answer-active : slot: type-answer-active ,
            type-answer-label : slot: type-answer-label ,
            type-answer-value : slot: type-answer-value ,
            type-answer-placeholder : slot: type-answer-placeholder ,
            type-answer-comparison-label : slot: type-answer-comparison-label ,
            type-answer-comparison-value : slot: type-answer-comparison-value ,
            type-answer-correct : slot: type-answer-correct ,
            progress-label : slot: progress-label ,
            onReveal : emit: onReveal ,
            onTypeAnswerChange : emit: onTypeAnswerChange ,
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

    If ( when: slot: show-browse-screen ) {
      Box [ browse-screen ] {
        Box [ browser-region ] {
          pkg::mosaic-pkg-card-browser::CardBrowser (
            browser-label : slot: browser-label ,
            query-label : slot: browser-query-label ,
            query : slot: browser-query ,
            query-placeholder : slot: browser-query-placeholder ,
            filter-label : slot: browser-filter-label ,
            filter-value : slot: browser-filter-value ,
            filter-options : slot: browser-filter-options ,
            filter-placeholder : slot: browser-filter-placeholder ,
            filter-open : slot: browser-filter-open ,
            search-label : slot: browser-search-label ,
            results-label : slot: browser-results-label ,
            results-summary : slot: browser-results-summary ,
            results : slot: browser-results ,
            result-card-ids : slot: browser-result-card-ids ,
            result-note-ids : slot: browser-result-note-ids ,
            result-template-ids : slot: browser-result-template-ids ,
            result-states : slot: browser-result-states ,
            result-flags : slot: browser-result-flags ,
            selected-index : slot: browser-selected-index ,
            selected-card-id : slot: browser-selected-card-id ,
            selected-note-id : slot: browser-selected-note-id ,
            selected-template-id : slot: browser-selected-template-id ,
            selected-state : slot: browser-selected-state ,
            selected-flag : slot: browser-selected-flag ,
            open-label : slot: browser-open-label ,
            edit-label : slot: browser-edit-label ,
            suspend-label : slot: browser-suspend-label ,
            mark-label : slot: browser-mark-label ,
            flag-label : slot: browser-flag-label ,
            flag-value : slot: browser-flag-value ,
            flag-options : slot: browser-flag-options ,
            flag-placeholder : slot: browser-flag-placeholder ,
            flag-open : slot: browser-flag-open ,
            tag-edit-label : slot: browser-tag-edit-label ,
            tag-edit : slot: browser-tag-edit ,
            tag-edit-placeholder : slot: browser-tag-edit-placeholder ,
            add-tag-label : slot: browser-add-tag-label ,
            remove-tag-label : slot: browser-remove-tag-label ,
            custom-study-label : slot: browser-custom-study-label ,
            custom-study-limit-label : slot: browser-custom-study-limit-label ,
            custom-study-limit-value : slot: browser-custom-study-limit-value ,
            custom-study-reschedule-label : slot: browser-custom-study-reschedule-label ,
            custom-study-reschedule-value : slot: browser-custom-study-reschedule-value ,
            custom-study-rebuild-label : slot: browser-custom-study-rebuild-label ,
            custom-study-empty-label : slot: browser-custom-study-empty-label ,
            onQueryChange : emit: onBrowserQueryChange ,
            onToggleFilter : emit: onBrowserToggleFilter ,
            onSetFilter : emit: onBrowserSetFilter ,
            onSearch : emit: onBrowserSearch ,
            onSelectResult : emit: onBrowserSelectResult ,
            onOpenSelected : emit: onBrowserOpenSelected ,
            onEditSelected : emit: onBrowserEditSelected ,
            onToggleSuspendSelected : emit: onBrowserToggleSuspendSelected ,
            onToggleMarkSelected : emit: onBrowserToggleMarkSelected ,
            onToggleFlagPicker : emit: onBrowserToggleFlagPicker ,
            onSetFlagSelected : emit: onBrowserSetFlagSelected ,
            onTagEditChange : emit: onBrowserTagEditChange ,
            onAddTagSelected : emit: onBrowserAddTagSelected ,
            onRemoveTagSelected : emit: onBrowserRemoveTagSelected ,
            onCustomStudyLimitChange : emit: onBrowserCustomStudyLimitChange ,
            onCustomStudyRescheduleChange : emit: onBrowserCustomStudyRescheduleChange ,
            onRebuildFilteredDeck : emit: onBrowserRebuildFilteredDeck ,
            onEmptyFilteredDeck : emit: onBrowserEmptyFilteredDeck
          )
        }
      }
    }

    If ( when: slot: show-add-screen ) {
      Box [ add-screen ] {
        Box [ note-editor-region ] {
          pkg::mosaic-pkg-note-editor::NoteEditor (
            editor-label : slot: note-editor-label ,
            note-id-label : slot: note-editor-note-id-label ,
            note-id-value : slot: note-editor-note-id-value ,
            note-type-label : slot: note-editor-note-type-label ,
            note-type-value : slot: note-editor-note-type-value ,
            note-type-options-label : slot: note-editor-note-type-options-label ,
            note-type-names : slot: note-editor-note-type-names ,
            selected-note-type-index : slot: note-editor-selected-note-type-index ,
            deck-label : slot: note-editor-deck-label ,
            deck-value : slot: note-editor-deck-value ,
            deck-options-label : slot: note-editor-deck-options-label ,
            deck-names : slot: note-editor-deck-names ,
            selected-deck-index : slot: note-editor-selected-deck-index ,
            fields-label : slot: note-editor-fields-label ,
            field-labels : slot: note-editor-field-labels ,
            selected-field-index : slot: note-editor-selected-field-index ,
            selected-field-label : slot: note-editor-selected-field-label ,
            selected-field-value : slot: note-editor-selected-field-value ,
            selected-field-placeholder : slot: note-editor-selected-field-placeholder ,
            tags-label : slot: note-editor-tags-label ,
            tags-value : slot: note-editor-tags-value ,
            tags-placeholder : slot: note-editor-tags-placeholder ,
            save-label : slot: note-editor-save-label ,
            delete-label : slot: note-editor-delete-label ,
            cancel-label : slot: note-editor-cancel-label ,
            onSelectNoteType : emit: onNoteEditorSelectNoteType ,
            onSelectDeck : emit: onNoteEditorSelectDeck ,
            onSelectField : emit: onNoteEditorSelectField ,
            onFieldValueChange : emit: onNoteEditorFieldValueChange ,
            onTagsChange : emit: onNoteEditorTagsChange ,
            onSaveNote : emit: onNoteEditorSaveNote ,
            onDeleteNote : emit: onNoteEditorDeleteNote ,
            onCancel : emit: onNoteEditorCancel
          )
        }
      }
    }

    If ( when: slot: show-stats-screen ) {
      Box [ stats-screen ] {
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
      }
    }

    If ( when: slot: show-options-screen ) {
      Column [ options-screen ] {
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
            initial-ease-label : slot: deck-options-initial-ease-label ,
            initial-ease-value : slot: deck-options-initial-ease-value ,
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
            leech-threshold-label : slot: deck-options-leech-threshold-label ,
            leech-threshold-value : slot: deck-options-leech-threshold-value ,
            desired-retention-label : slot: deck-options-desired-retention-label ,
            desired-retention-value : slot: deck-options-desired-retention-value ,
            fsrs-parameters-label : slot: deck-options-fsrs-parameters-label ,
            fsrs-parameters-value : slot: deck-options-fsrs-parameters-value ,
            fsrs-search-label : slot: deck-options-fsrs-search-label ,
            fsrs-search-value : slot: deck-options-fsrs-search-value ,
            ignore-review-history-before-label : slot: deck-options-ignore-review-history-before-label ,
            ignore-review-history-before-value : slot: deck-options-ignore-review-history-before-value ,
            historical-retention-label : slot: deck-options-historical-retention-label ,
            historical-retention-value : slot: deck-options-historical-retention-value ,
            easy-days-percentages-label : slot: deck-options-easy-days-percentages-label ,
            easy-days-percentages-value : slot: deck-options-easy-days-percentages-value ,
            leech-action-label : slot: deck-options-leech-action-label ,
            leech-action-suspend-label : slot: deck-options-leech-action-suspend-label ,
            leech-action-suspend-value : slot: deck-options-leech-action-suspend-value ,
            leech-action-tag-only-label : slot: deck-options-leech-action-tag-only-label ,
            leech-action-tag-only-value : slot: deck-options-leech-action-tag-only-value ,
            bury-new-siblings-label : slot: deck-options-bury-new-siblings-label ,
            bury-new-siblings-value : slot: deck-options-bury-new-siblings-value ,
            bury-review-siblings-label : slot: deck-options-bury-review-siblings-label ,
            bury-review-siblings-value : slot: deck-options-bury-review-siblings-value ,
            bury-interday-learning-siblings-label : slot: deck-options-bury-interday-learning-siblings-label ,
            bury-interday-learning-siblings-value : slot: deck-options-bury-interday-learning-siblings-value ,
            onLearningStepsChange : emit: onDeckOptionsLearningStepsChange ,
            onRelearningStepsChange : emit: onDeckOptionsRelearningStepsChange ,
            onNewCardsChange : emit: onDeckOptionsNewCardsChange ,
            onReviewsChange : emit: onDeckOptionsReviewsChange ,
            onGraduatingIntervalChange : emit: onDeckOptionsGraduatingIntervalChange ,
            onEasyIntervalChange : emit: onDeckOptionsEasyIntervalChange ,
            onInitialEaseChange : emit: onDeckOptionsInitialEaseChange ,
            onMaximumIntervalChange : emit: onDeckOptionsMaximumIntervalChange ,
            onIntervalModifierChange : emit: onDeckOptionsIntervalModifierChange ,
            onHardMultiplierChange : emit: onDeckOptionsHardMultiplierChange ,
            onEasyBonusChange : emit: onDeckOptionsEasyBonusChange ,
            onLapseMultiplierChange : emit: onDeckOptionsLapseMultiplierChange ,
            onLeechThresholdChange : emit: onDeckOptionsLeechThresholdChange ,
            onDesiredRetentionChange : emit: onDeckOptionsDesiredRetentionChange ,
            onFsrsParametersChange : emit: onDeckOptionsFsrsParametersChange ,
            onFsrsSearchChange : emit: onDeckOptionsFsrsSearchChange ,
            onIgnoreReviewHistoryBeforeChange : emit: onDeckOptionsIgnoreReviewHistoryBeforeChange ,
            onHistoricalRetentionChange : emit: onDeckOptionsHistoricalRetentionChange ,
            onEasyDaysPercentagesChange : emit: onDeckOptionsEasyDaysPercentagesChange ,
            onLeechActionChange : emit: onDeckOptionsLeechActionChange ,
            onBuryNewSiblingsChange : emit: onDeckOptionsBuryNewSiblingsChange ,
            onBuryReviewSiblingsChange : emit: onDeckOptionsBuryReviewSiblingsChange ,
            onBuryInterdayLearningSiblingsChange : emit: onDeckOptionsBuryInterdayLearningSiblingsChange
          )
        }
        Box [ note-type-editor-region ] {
          pkg::mosaic-pkg-note-type-editor::NoteTypeEditor (
            editor-label : slot: note-type-editor-label ,
            note-types-label : slot: note-type-editor-note-types-label ,
            note-type-names : slot: note-type-editor-note-type-names ,
            selected-note-type-index : slot: note-type-editor-selected-note-type-index ,
            note-type-id-label : slot: note-type-editor-note-type-id-label ,
            note-type-id-value : slot: note-type-editor-note-type-id-value ,
            name-label : slot: note-type-editor-name-label ,
            name-value : slot: note-type-editor-name-value ,
            name-placeholder : slot: note-type-editor-name-placeholder ,
            fields-label : slot: note-type-editor-fields-label ,
            field-labels : slot: note-type-editor-field-labels ,
            selected-field-index : slot: note-type-editor-selected-field-index ,
            field-name-label : slot: note-type-editor-field-name-label ,
            field-name-value : slot: note-type-editor-field-name-value ,
            field-name-placeholder : slot: note-type-editor-field-name-placeholder ,
            field-required-label : slot: note-type-editor-field-required-label ,
            field-required-value : slot: note-type-editor-field-required-value ,
            templates-label : slot: note-type-editor-templates-label ,
            template-labels : slot: note-type-editor-template-labels ,
            selected-template-index : slot: note-type-editor-selected-template-index ,
            template-name-label : slot: note-type-editor-template-name-label ,
            template-name-value : slot: note-type-editor-template-name-value ,
            template-name-placeholder : slot: note-type-editor-template-name-placeholder ,
            front-template-label : slot: note-type-editor-front-template-label ,
            front-template-value : slot: note-type-editor-front-template-value ,
            front-template-placeholder : slot: note-type-editor-front-template-placeholder ,
            back-template-label : slot: note-type-editor-back-template-label ,
            back-template-value : slot: note-type-editor-back-template-value ,
            back-template-placeholder : slot: note-type-editor-back-template-placeholder ,
            stylesheet-label : slot: note-type-editor-stylesheet-label ,
            stylesheet-value : slot: note-type-editor-stylesheet-value ,
            stylesheet-placeholder : slot: note-type-editor-stylesheet-placeholder ,
            new-label : slot: note-type-editor-new-label ,
            save-label : slot: note-type-editor-save-label ,
            delete-label : slot: note-type-editor-delete-label ,
            cancel-label : slot: note-type-editor-cancel-label ,
            onSelectNoteType : emit: onNoteTypeEditorSelectNoteType ,
            onSelectField : emit: onNoteTypeEditorSelectField ,
            onSelectTemplate : emit: onNoteTypeEditorSelectTemplate ,
            onNameChange : emit: onNoteTypeEditorNameChange ,
            onFieldNameChange : emit: onNoteTypeEditorFieldNameChange ,
            onFieldRequiredChange : emit: onNoteTypeEditorFieldRequiredChange ,
            onTemplateNameChange : emit: onNoteTypeEditorTemplateNameChange ,
            onFrontTemplateChange : emit: onNoteTypeEditorFrontTemplateChange ,
            onBackTemplateChange : emit: onNoteTypeEditorBackTemplateChange ,
            onStylesheetChange : emit: onNoteTypeEditorStylesheetChange ,
            onNewNoteType : emit: onNoteTypeEditorNewNoteType ,
            onSaveNoteType : emit: onNoteTypeEditorSaveNoteType ,
            onDeleteNoteType : emit: onNoteTypeEditorDeleteNoteType ,
            onCancel : emit: onNoteTypeEditorCancel
          )
        }
      }
    }
  }
}
