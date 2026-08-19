use crate::ApplicationError;
use coding_adventures_vault_pm_domain::{
    CollectionId, ItemCandidate, ItemDocument, ItemId, ItemState, RedactedItemView,
    RedactedRecordView,
};
use coding_adventures_vault_search::{
    DocumentId, SearchError, SearchIndex, SearchableFields, MAX_INDEXED_FIELD_LEN,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalize::UnicodeNormalize;

pub(crate) const MAX_SEARCH_QUERY_BYTES: usize = 256;
pub(crate) const MAX_SEARCH_RESULTS: usize = 10_000;

struct SearchEntryV1 {
    normalized_title: Zeroizing<String>,
    normalized_text: Zeroizing<String>,
    accelerated: bool,
}

pub(crate) struct SearchProjectionV1 {
    index: SearchIndex,
    entries: BTreeMap<ItemId, SearchEntryV1>,
}

impl SearchProjectionV1 {
    pub(crate) fn build(
        items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    ) -> Result<Self, ApplicationError> {
        let index = SearchIndex::new();
        let mut entries = BTreeMap::new();
        for (item_id, candidates) in items {
            let [candidate] = candidates.as_slice() else {
                continue;
            };
            let ItemState::Live(document) = candidate.state() else {
                continue;
            };
            let entry = SearchEntryV1::from_document(document)?;
            if entry.normalized_text.len() <= MAX_INDEXED_FIELD_LEN {
                index_entry(&index, *item_id, &entry.normalized_text)?;
            }
            let accelerated = entry.normalized_text.len() <= MAX_INDEXED_FIELD_LEN;
            entries.insert(
                *item_id,
                SearchEntryV1 {
                    accelerated,
                    ..entry
                },
            );
        }
        Ok(Self { index, entries })
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn search(
        &self,
        query: Zeroizing<String>,
        collection: Option<CollectionId>,
        limit: usize,
        items: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    ) -> Result<Vec<RedactedItemView>, ApplicationError> {
        validate_query(&query, limit)?;
        let normalized_query = normalize(&query);
        let tokens = normalized_query.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let mut candidate_ids = self.candidates_for_token(tokens[0])?;
        for token in &tokens[1..] {
            let token_candidates = self.candidates_for_token(token)?;
            candidate_ids.retain(|item_id| token_candidates.contains(item_id));
        }

        let mut matches = Vec::new();
        for item_id in candidate_ids {
            let entry = self
                .entries
                .get(&item_id)
                .ok_or(ApplicationError::InternalInvariant)?;
            if !tokens
                .iter()
                .all(|token| entry.normalized_text.contains(token))
            {
                continue;
            }
            let document = current_live_document(
                items
                    .get(&item_id)
                    .ok_or(ApplicationError::InternalInvariant)?,
            )?;
            if collection.is_some_and(|id| !document.collection_ids().contains(&id)) {
                continue;
            }
            matches.push(item_id);
        }

        matches.sort_by(|left, right| {
            let left_entry = &self.entries[left];
            let right_entry = &self.entries[right];
            let left_document = live_document_invariant(&items[left]);
            let right_document = live_document_invariant(&items[right]);
            left_entry
                .normalized_title
                .cmp(&right_entry.normalized_title)
                .then_with(|| {
                    left_document
                        .schema()
                        .as_str()
                        .cmp(right_document.schema().as_str())
                })
                .then_with(|| left.as_bytes().cmp(right.as_bytes()))
        });
        matches.truncate(limit);

        matches
            .into_iter()
            .map(|item_id| {
                RedactedItemView::from_document(live_document_invariant(&items[&item_id]))
                    .map_err(|_| ApplicationError::InternalInvariant)
            })
            .collect()
    }

    fn candidates_for_token(
        &self,
        normalized_token: &str,
    ) -> Result<BTreeSet<ItemId>, ApplicationError> {
        if normalized_token.len() < 3 {
            return Ok(self.entries.keys().copied().collect());
        }
        let hits = self
            .index
            .search(normalized_token, self.index.len())
            .map_err(map_query_error)?;
        let mut candidates = BTreeSet::new();
        for hit in hits {
            let item_id = ItemId::from_user_string(hit.id.as_str())
                .map_err(|_| ApplicationError::InternalInvariant)?;
            if !self.entries.contains_key(&item_id) {
                return Err(ApplicationError::InternalInvariant);
            }
            candidates.insert(item_id);
        }
        candidates.extend(
            self.entries
                .iter()
                .filter_map(|(item_id, entry)| (!entry.accelerated).then_some(*item_id)),
        );
        Ok(candidates)
    }
}

impl Drop for SearchProjectionV1 {
    fn drop(&mut self) {
        self.index.clear();
    }
}

impl SearchEntryV1 {
    fn from_document(document: &ItemDocument) -> Result<Self, ApplicationError> {
        let view = RedactedItemView::from_document(document)
            .map_err(|_| ApplicationError::InternalInvariant)?;
        let title = display_title(&view.record).unwrap_or_default();
        let normalized_title = normalize(title);
        let mut normalized_text = Zeroizing::new(String::new());
        push_normalized(&mut normalized_text, title);

        match &view.record {
            RedactedRecordView::Login { username, urls, .. } => {
                push_normalized(&mut normalized_text, username);
                for url in urls {
                    push_normalized(&mut normalized_text, url);
                }
            }
            RedactedRecordView::ApiKey { service, .. } => {
                push_normalized(&mut normalized_text, service);
            }
            RedactedRecordView::DatabaseCredential { host, username, .. } => {
                push_normalized(&mut normalized_text, host);
                push_normalized(&mut normalized_text, username);
            }
            RedactedRecordView::SecureNote { .. }
            | RedactedRecordView::Card { .. }
            | RedactedRecordView::TotpSeed { .. }
            | RedactedRecordView::Opaque { .. }
            | RedactedRecordView::Quarantined { .. } => {}
        }
        for tag in document.tags().values() {
            push_normalized(&mut normalized_text, tag);
        }

        Ok(Self {
            normalized_title,
            normalized_text,
            accelerated: false,
        })
    }
}

fn display_title(record: &RedactedRecordView) -> Option<&str> {
    match record {
        RedactedRecordView::Login { title, .. }
        | RedactedRecordView::SecureNote { title, .. }
        | RedactedRecordView::Card { title, .. } => Some(title),
        RedactedRecordView::TotpSeed { label, .. }
        | RedactedRecordView::ApiKey { label, .. }
        | RedactedRecordView::DatabaseCredential { label, .. } => Some(label),
        RedactedRecordView::Opaque { .. } | RedactedRecordView::Quarantined { .. } => None,
    }
}

fn push_normalized(destination: &mut String, value: &str) {
    if !destination.is_empty() {
        destination.push('\n');
    }
    let normalized = normalize(value);
    destination.push_str(&normalized);
}

fn normalize(value: &str) -> Zeroizing<String> {
    let mut lowered = Zeroizing::new(
        value
            .chars()
            .flat_map(char::to_lowercase)
            .collect::<String>(),
    );
    let normalized = Zeroizing::new(lowered.as_str().nfc().collect::<String>());
    lowered.zeroize();
    normalized
}

fn index_entry(
    index: &SearchIndex,
    item_id: ItemId,
    normalized_text: &str,
) -> Result<(), ApplicationError> {
    let id = DocumentId::new(item_id.to_user_string())
        .map_err(|_| ApplicationError::InternalInvariant)?;
    let fields = BTreeMap::from([("content".to_owned(), normalized_text.to_owned())]);
    let searchable = SearchableFields::new().with("content", 1.0);
    let result = index.index(id, &fields, &searchable);
    for (mut key, mut value) in fields {
        key.zeroize();
        value.zeroize();
    }
    result.map_err(map_index_error)
}

fn validate_query(query: &str, limit: usize) -> Result<(), ApplicationError> {
    if query.is_empty()
        || query.len() > MAX_SEARCH_QUERY_BYTES
        || query.chars().any(char::is_control)
        || limit == 0
    {
        return Err(ApplicationError::InvalidInput);
    }
    if limit > MAX_SEARCH_RESULTS {
        return Err(ApplicationError::BoundExceeded);
    }
    Ok(())
}

fn current_live_document(candidates: &[ItemCandidate]) -> Result<&ItemDocument, ApplicationError> {
    let [candidate] = candidates else {
        return Err(ApplicationError::ConflictRequired);
    };
    match candidate.state() {
        ItemState::Live(document) => Ok(document),
        ItemState::Tombstone(_) => Err(ApplicationError::InternalInvariant),
    }
}

fn live_document_invariant(candidates: &[ItemCandidate]) -> &ItemDocument {
    match candidates {
        [candidate] => match candidate.state() {
            ItemState::Live(document) => document,
            ItemState::Tombstone(_) => unreachable!("search entry cannot reference a tombstone"),
        },
        _ => unreachable!("search entry cannot reference a conflict"),
    }
}

fn map_index_error(error: SearchError) -> ApplicationError {
    match error {
        SearchError::TooLarge(_) => ApplicationError::BoundExceeded,
        SearchError::InvalidParameter(_) | SearchError::Decode(_) => {
            ApplicationError::InternalInvariant
        }
    }
}

fn map_query_error(error: SearchError) -> ApplicationError {
    match error {
        SearchError::InvalidParameter(_) => ApplicationError::InvalidInput,
        SearchError::TooLarge(_) => ApplicationError::BoundExceeded,
        SearchError::Decode(_) => ApplicationError::InternalInvariant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_pm_domain::{
        ContentType, LwwRegister, ObservedSet, OperationId, RevisionId,
    };
    use coding_adventures_vault_records::{
        AnyRecord, ApiKey, Card, DatabaseCredential, Login, SecureNote, TotpSeed, API_KEY_V1,
        CARD_V1, DATABASE_CREDENTIAL_V1, LOGIN_V1, SECURE_NOTE_V1, TOTP_SEED_V1,
    };

    fn live_candidate(item_id: ItemId, record: AnyRecord) -> ItemCandidate {
        let schema = match &record {
            AnyRecord::Login(_) => LOGIN_V1,
            AnyRecord::SecureNote(_) => SECURE_NOTE_V1,
            AnyRecord::Card(_) => CARD_V1,
            AnyRecord::TotpSeed(_) => TOTP_SEED_V1,
            AnyRecord::ApiKey(_) => API_KEY_V1,
            AnyRecord::DatabaseCredential(_) => DATABASE_CREDENTIAL_V1,
            AnyRecord::Opaque { content_type, .. } => content_type,
            AnyRecord::Quarantined { content_type, .. } => content_type,
        };
        let document = ItemDocument::new(
            item_id,
            ContentType::new(schema).unwrap(),
            1,
            2,
            LwwRegister::new(false, 2, OperationId::new([0x91; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            record,
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .unwrap();
        ItemCandidate::new(
            RevisionId::new([item_id.as_bytes()[0].wrapping_add(1); 32]),
            [],
            ItemState::Live(Box::new(document)),
        )
        .unwrap()
    }

    fn login(item_id: ItemId, title: &str) -> (ItemId, Vec<ItemCandidate>) {
        (
            item_id,
            vec![live_candidate(
                item_id,
                AnyRecord::Login(Login {
                    title: title.to_owned(),
                    username: "user@example.test".to_owned(),
                    password: "credential-secret".to_owned(),
                    urls: vec!["https://example.test".to_owned()],
                    notes: Some("note-secret".to_owned()),
                }),
            )],
        )
    }

    #[test]
    fn results_use_title_schema_and_item_id_order_instead_of_rank() {
        let first_login = ItemId::new([0x10; 16]);
        let second_login = ItemId::new([0x20; 16]);
        let note_id = ItemId::new([0x01; 16]);
        let beta_id = ItemId::new([0x00; 16]);
        let items = BTreeMap::from([
            login(second_login, "ALPHA"),
            login(first_login, "Alpha"),
            (
                note_id,
                vec![live_candidate(
                    note_id,
                    AnyRecord::SecureNote(SecureNote {
                        title: "alpha".to_owned(),
                        body: "body-secret".to_owned(),
                    }),
                )],
            ),
            login(beta_id, "beta"),
        ]);
        let search = SearchProjectionV1::build(&items).unwrap();

        let results = search
            .search(Zeroizing::new("a".to_owned()), None, 10, &items)
            .unwrap();
        assert_eq!(
            results.iter().map(|view| view.item_id).collect::<Vec<_>>(),
            vec![first_login, second_login, note_id, beta_id]
        );
    }

    #[test]
    fn oversized_safe_metadata_uses_exact_fallback_without_indexing_secrets() {
        let item_id = ItemId::new([0x30; 16]);
        let mut title = "x".repeat(MAX_INDEXED_FIELD_LEN + 1);
        title.push_str("needle");
        let items = BTreeMap::from([login(item_id, &title)]);
        let search = SearchProjectionV1::build(&items).unwrap();
        assert_eq!(search.len(), 1);
        assert_eq!(search.index.len(), 0);
        assert_eq!(
            search
                .search(Zeroizing::new("needle".to_owned()), None, 10, &items)
                .unwrap()[0]
                .item_id,
            item_id
        );
        for secret in ["credential-secret", "note-secret"] {
            assert!(search
                .search(Zeroizing::new(secret.to_owned()), None, 10, &items)
                .unwrap()
                .is_empty());
        }
    }

    #[test]
    fn every_record_schema_obeys_the_search_allowlist() {
        let login_id = ItemId::new([0x41; 16]);
        let note_id = ItemId::new([0x42; 16]);
        let card_id = ItemId::new([0x43; 16]);
        let totp_id = ItemId::new([0x44; 16]);
        let api_id = ItemId::new([0x45; 16]);
        let database_id = ItemId::new([0x46; 16]);
        let opaque_id = ItemId::new([0x47; 16]);
        let records = [
            (
                login_id,
                AnyRecord::Login(Login {
                    title: "Login Alpha".to_owned(),
                    username: "login-user-visible".to_owned(),
                    password: "login-password-secret".to_owned(),
                    urls: vec!["https://login-visible.example".to_owned()],
                    notes: Some("login-note-secret".to_owned()),
                }),
            ),
            (
                note_id,
                AnyRecord::SecureNote(SecureNote {
                    title: "Note Bravo".to_owned(),
                    body: "note-body-secret".to_owned(),
                }),
            ),
            (
                card_id,
                AnyRecord::Card(Card {
                    title: "Card Charlie".to_owned(),
                    holder: "card-holder-hidden".to_owned(),
                    number: "4111111111111111".to_owned(),
                    expiry_month: 12,
                    expiry_year: 2030,
                    cvv: "card-cvv-secret".to_owned(),
                    billing_zip: Some("card-zip-hidden".to_owned()),
                }),
            ),
            (
                totp_id,
                AnyRecord::TotpSeed(TotpSeed {
                    label: "Totp Delta".to_owned(),
                    issuer: Some("totp-issuer-hidden".to_owned()),
                    secret: b"totp-seed-secret".to_vec(),
                    algorithm: "totp-algorithm-hidden".to_owned(),
                    digits: 6,
                    period: 30,
                }),
            ),
            (
                api_id,
                AnyRecord::ApiKey(ApiKey {
                    label: "Api Echo".to_owned(),
                    service: "api-service-visible".to_owned(),
                    token: "api-token-secret".to_owned(),
                    scopes: vec!["api-scope-hidden".to_owned()],
                    expires_at: None,
                }),
            ),
            (
                database_id,
                AnyRecord::DatabaseCredential(DatabaseCredential {
                    label: "Database Foxtrot".to_owned(),
                    engine: "database-engine-hidden".to_owned(),
                    host: "database-host-visible".to_owned(),
                    port: 5432,
                    database: Some("database-name-hidden".to_owned()),
                    username: "database-user-visible".to_owned(),
                    password: "database-password-secret".to_owned(),
                    lease_id: Some("database-lease-secret".to_owned()),
                    expires_at: None,
                }),
            ),
            (
                opaque_id,
                AnyRecord::Opaque {
                    content_type: "vendor/opaque-hidden/v1".to_owned(),
                    payload_bytes: b"opaque-payload-secret".to_vec(),
                },
            ),
        ];
        let items = records
            .into_iter()
            .map(|(item_id, record)| (item_id, vec![live_candidate(item_id, record)]))
            .collect::<BTreeMap<_, _>>();
        let search = SearchProjectionV1::build(&items).unwrap();

        for (query, expected_id) in [
            ("login alpha", login_id),
            ("login-user-visible", login_id),
            ("login-visible.example", login_id),
            ("note bravo", note_id),
            ("card charlie", card_id),
            ("totp delta", totp_id),
            ("api echo", api_id),
            ("api-service-visible", api_id),
            ("database foxtrot", database_id),
            ("database-host-visible", database_id),
            ("database-user-visible", database_id),
        ] {
            let results = search
                .search(Zeroizing::new(query.to_owned()), None, 10, &items)
                .unwrap();
            assert_eq!(results.len(), 1, "query {query:?}");
            assert_eq!(results[0].item_id, expected_id, "query {query:?}");
        }

        for forbidden in [
            "login-password-secret",
            "login-note-secret",
            "note-body-secret",
            "card-holder-hidden",
            "4111111111111111",
            "card-cvv-secret",
            "card-zip-hidden",
            "totp-issuer-hidden",
            "totp-seed-secret",
            "totp-algorithm-hidden",
            "api-token-secret",
            "api-scope-hidden",
            "database-engine-hidden",
            "database-name-hidden",
            "database-password-secret",
            "database-lease-secret",
            "opaque-hidden",
            "opaque-payload-secret",
        ] {
            assert!(
                search
                    .search(Zeroizing::new(forbidden.to_owned()), None, 10, &items)
                    .unwrap()
                    .is_empty(),
                "forbidden query {forbidden:?}"
            );
        }
    }
}
