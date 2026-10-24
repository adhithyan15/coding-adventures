# Human-language generated ownership passes are ordered, not parallel

Do not run `generate:modality`, `generate:books`, and `generate:narration` in
parallel after adding lessons. The modality writer validates narration lesson
identities, so it can observe the old narration ledger while the narration
writer is still replacing it and fail with a misleading missing-lesson list.

Run authored-source validation first, then narration and books, and run the
modality writer only after narration ownership is current. Parallelize only
independent read-only checks after all generated owners exist.
