#ifndef ENGRAM_CAPI_H
#define ENGRAM_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct EgSession EgSession;

EgSession *eg_session_new(void);
void eg_session_free(EgSession *session);
void eg_string_free(char *value);

char *eg_snapshot(EgSession *session);
char *eg_load_snapshot(EgSession *session, const char *snapshot_json);
char *eg_export_backup(EgSession *session, uint64_t exported_at);
char *eg_import_backup(EgSession *session, const char *backup_json);
char *eg_dispatch(EgSession *session, const char *command_json);
char *eg_build_queue(EgSession *session, const char *deck_id, uint64_t now);
char *eg_daily_limit_usage(
    EgSession *session,
    const char *deck_id,
    uint64_t day_start,
    uint64_t day_end,
    const char *deck_options_json
);
char *eg_build_queue_with_daily_limits(
    EgSession *session,
    const char *deck_id,
    uint64_t now,
    uint64_t day_start,
    uint64_t day_end,
    const char *deck_options_json
);
char *eg_deck_stats(EgSession *session, const char *deck_id, uint64_t now);
char *eg_session_progress(EgSession *session);
char *eg_engram_app_props(EgSession *session, const char *deck_id, uint64_t now);
char *eg_review_history(
    EgSession *session,
    const char *deck_id,
    uint64_t reviewed_after,
    uint64_t reviewed_before
);
char *eg_generated_cards(EgSession *session, const char *note_type_id, const char *note_id);
char *eg_materialized_cards(
    EgSession *session,
    const char *note_type_id,
    const char *note_id,
    uint64_t created_at
);
char *eg_search_cards(EgSession *session, const char *query, uint64_t now);
char *eg_export_cards_csv(EgSession *session, const char *deck_id);
char *eg_export_anki_basic_tsv(
    EgSession *session,
    const char *deck_id,
    const char *deck_name,
    const char *note_type_name,
    uint8_t html
);
char *eg_export_anki_notes_tsv(
    EgSession *session,
    const char *note_type_id,
    const char *deck_id,
    const char *deck_name,
    const char *note_type_name,
    uint8_t html
);
char *eg_parse_cards_csv(EgSession *session, const char *csv);
char *eg_parse_basic_cards_csv(
    EgSession *session,
    const char *csv,
    const char *deck_id,
    const char *id_prefix,
    uint64_t created_at
);
char *eg_parse_anki_basic_tsv(
    EgSession *session,
    const char *tsv,
    const char *deck_id,
    const char *id_prefix,
    uint64_t created_at
);
char *eg_parse_anki_notes_tsv(
    EgSession *session,
    const char *tsv,
    const char *deck_id,
    const char *note_type_id,
    const char *note_type_name,
    const char *note_id_prefix,
    uint64_t created_at
);
char *eg_export_anki_apkg(EgSession *session);
char *eg_analyze_media_references(EgSession *session);
char *eg_inspect_anki_apkg(EgSession *session, const uint8_t *data, size_t data_len);
char *eg_read_anki_apkg_media(
    EgSession *session,
    const uint8_t *data,
    size_t data_len,
    const char *archive_name
);
char *eg_parse_anki_apkg(EgSession *session, const uint8_t *data, size_t data_len);
char *eg_import_anki_apkg(EgSession *session, const uint8_t *data, size_t data_len);

#ifdef __cplusplus
}
#endif

#endif
