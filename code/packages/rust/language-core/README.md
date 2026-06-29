# language-core

`language-core` is the headless Rust domain model for the broader language
learning app that can sit beside Engram.

It owns language-learning concepts that should not live inside the flashcard
engine:

- languages, scripts, graphemes, and phonemes
- lexemes, glosses, and tags
- etymology/cognate links between lexemes
- lesson nodes and exercises
- bindings from language items to Engram review cards

The first helpers are intentionally small and pure:

- `etymology_path(collection, lexeme_id)`
- `lexemes_with_shared_ancestor(collection, ancestor_lexeme_id)`
- `review_card_ids_for_lesson(collection, lesson_id)`

This crate does not own UI, storage, sync, generated exercises, or scheduling.
Engram remains the memory/review engine.
