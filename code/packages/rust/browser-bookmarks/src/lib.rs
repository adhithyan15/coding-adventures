//! Storage-neutral bookmark state and repository semantics for browsers.

use std::{collections::BTreeSet, fmt};

use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";

/// Canonical bookmark identity.
///
/// Unlike visited-link identity, fragments remain significant because a user
/// may intentionally bookmark two anchors in the same document.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BookmarkUrl(String);

impl BookmarkUrl {
    pub fn parse(url: &str) -> Result<Self, UrlError> {
        Ok(Self(Url::parse(url)?.canonicalize()?.to_url_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BookmarkUrl {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for BookmarkUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One user-visible bookmark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    url: BookmarkUrl,
    title: String,
}

impl Bookmark {
    pub fn new(url: &str, title: impl Into<String>) -> Result<Self, UrlError> {
        let url = BookmarkUrl::parse(url)?;
        Ok(Self::from_canonical(url, title))
    }

    pub fn from_canonical(url: BookmarkUrl, title: impl Into<String>) -> Self {
        let mut title = title.into().trim().to_string();
        if title.is_empty() {
            title = url.as_str().to_string();
        }
        Self { url, title }
    }

    pub fn url(&self) -> &BookmarkUrl {
        &self.url
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

/// Invalid persisted catalog structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BookmarkCatalogError {
    DuplicateUrl(BookmarkUrl),
}

impl fmt::Display for BookmarkCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateUrl(url) => write!(formatter, "duplicate bookmark URL: {url}"),
        }
    }
}

impl std::error::Error for BookmarkCatalogError {}

/// Result of a catalog mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookmarkChange {
    Added,
    Updated,
    Removed,
    Unchanged,
}

impl BookmarkChange {
    pub const fn changed(self) -> bool {
        !matches!(self, Self::Unchanged)
    }
}

/// Ordered bookmark collection with unique canonical URLs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookmarkCatalog {
    entries: Vec<Bookmark>,
}

impl BookmarkCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: Vec<Bookmark>) -> Result<Self, BookmarkCatalogError> {
        let mut urls = BTreeSet::new();
        for entry in &entries {
            if !urls.insert(entry.url.clone()) {
                return Err(BookmarkCatalogError::DuplicateUrl(entry.url.clone()));
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[Bookmark] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, url: &str) -> Option<&Bookmark> {
        let url = BookmarkUrl::parse(url).ok()?;
        self.entries.iter().find(|entry| entry.url == url)
    }

    pub fn contains(&self, url: &str) -> bool {
        self.get(url).is_some()
    }

    /// Add a bookmark or update its title without changing its list position.
    pub fn upsert(
        &mut self,
        url: &str,
        title: impl Into<String>,
    ) -> Result<BookmarkChange, UrlError> {
        let bookmark = Bookmark::new(url, title)?;
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|entry| entry.url == bookmark.url)
        {
            if existing.title == bookmark.title {
                return Ok(BookmarkChange::Unchanged);
            }
            existing.title = bookmark.title;
            return Ok(BookmarkChange::Updated);
        }
        self.entries.push(bookmark);
        Ok(BookmarkChange::Added)
    }

    pub fn remove(&mut self, url: &str) -> BookmarkChange {
        let Ok(url) = BookmarkUrl::parse(url) else {
            return BookmarkChange::Unchanged;
        };
        let Some(index) = self.entries.iter().position(|entry| entry.url == url) else {
            return BookmarkChange::Unchanged;
        };
        self.entries.remove(index);
        BookmarkChange::Removed
    }

    pub fn toggle(
        &mut self,
        url: &str,
        title: impl Into<String>,
    ) -> Result<BookmarkChange, UrlError> {
        if self.contains(url) {
            Ok(self.remove(url))
        } else {
            self.upsert(url, title)
        }
    }
}

/// Stable repository failure shared by storage adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BookmarkRepositoryError {
    Unavailable(String),
    Corrupt(String),
    UnsupportedSchema(u32),
}

impl fmt::Display for BookmarkRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(message) => {
                write!(formatter, "bookmark storage unavailable: {message}")
            }
            Self::Corrupt(message) => write!(formatter, "bookmark storage is corrupt: {message}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported bookmark schema version: {version}")
            }
        }
    }
}

impl std::error::Error for BookmarkRepositoryError {}

/// Object-safe durable bookmark boundary.
pub trait BookmarkRepository {
    fn load(&mut self) -> Result<BookmarkCatalog, BookmarkRepositoryError>;
    fn save(&mut self, catalog: &BookmarkCatalog) -> Result<(), BookmarkRepositoryError>;
}

impl<T> BookmarkRepository for Box<T>
where
    T: BookmarkRepository + ?Sized,
{
    fn load(&mut self) -> Result<BookmarkCatalog, BookmarkRepositoryError> {
        (**self).load()
    }

    fn save(&mut self, catalog: &BookmarkCatalog) -> Result<(), BookmarkRepositoryError> {
        (**self).save(catalog)
    }
}

/// Apply a catalog mutation only after its complete candidate persists.
pub fn transact<R, F>(
    catalog: &mut BookmarkCatalog,
    repository: &mut R,
    mutate: F,
) -> Result<BookmarkChange, BookmarkRepositoryError>
where
    R: BookmarkRepository + ?Sized,
    F: FnOnce(&mut BookmarkCatalog) -> Result<BookmarkChange, UrlError>,
{
    let mut candidate = catalog.clone();
    let change = mutate(&mut candidate)
        .map_err(|error| BookmarkRepositoryError::Corrupt(error.to_string()))?;
    if change.changed() {
        repository.save(&candidate)?;
        *catalog = candidate;
    }
    Ok(change)
}

/// Deterministic repository useful for ephemeral embedders and tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryBookmarkRepository {
    catalog: BookmarkCatalog,
    fail_saves_with: Option<String>,
}

impl MemoryBookmarkRepository {
    pub fn new(catalog: BookmarkCatalog) -> Self {
        Self {
            catalog,
            fail_saves_with: None,
        }
    }

    pub fn fail_saves_with(&mut self, message: impl Into<String>) {
        self.fail_saves_with = Some(message.into());
    }

    pub fn stored(&self) -> &BookmarkCatalog {
        &self.catalog
    }
}

impl BookmarkRepository for MemoryBookmarkRepository {
    fn load(&mut self) -> Result<BookmarkCatalog, BookmarkRepositoryError> {
        Ok(self.catalog.clone())
    }

    fn save(&mut self, catalog: &BookmarkCatalog) -> Result<(), BookmarkRepositoryError> {
        if let Some(message) = &self.fail_saves_with {
            return Err(BookmarkRepositoryError::Unavailable(message.clone()));
        }
        self.catalog = catalog.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_identity_preserves_fragments() {
        let first = BookmarkUrl::parse("HTTP://Example.TEST:80/a/../%7euser#one").unwrap();
        let equivalent = BookmarkUrl::parse("http://example.test/~user#one").unwrap();
        let other_anchor = BookmarkUrl::parse("http://example.test/~user#two").unwrap();
        assert_eq!(first, equivalent);
        assert_ne!(first, other_anchor);
        assert_eq!(first.as_str(), "http://example.test/~user#one");
    }

    #[test]
    fn upsert_preserves_order_and_updates_equivalent_urls() {
        let mut catalog = BookmarkCatalog::new();
        assert_eq!(
            catalog.upsert("http://example.test/one", "One").unwrap(),
            BookmarkChange::Added
        );
        catalog.upsert("http://example.test/two", "Two").unwrap();
        assert_eq!(
            catalog
                .upsert("HTTP://EXAMPLE.TEST:80/one", "Updated")
                .unwrap(),
            BookmarkChange::Updated
        );
        assert_eq!(catalog.entries()[0].title(), "Updated");
        assert_eq!(catalog.entries()[1].title(), "Two");
    }

    #[test]
    fn blank_titles_fall_back_to_canonical_url() {
        let bookmark = Bookmark::new("HTTP://EXAMPLE.TEST:80/", "  ").unwrap();
        assert_eq!(bookmark.title(), "http://example.test/");
    }

    #[test]
    fn persisted_duplicates_are_rejected() {
        let bookmark = Bookmark::new("http://example.test/", "Example").unwrap();
        let error = BookmarkCatalog::from_entries(vec![bookmark.clone(), bookmark]).unwrap_err();
        assert!(matches!(error, BookmarkCatalogError::DuplicateUrl(_)));
    }

    #[test]
    fn toggle_adds_and_removes_the_same_anchor() {
        let mut catalog = BookmarkCatalog::new();
        assert_eq!(
            catalog
                .toggle("http://example.test/#chapter", "Chapter")
                .unwrap(),
            BookmarkChange::Added
        );
        assert!(catalog.contains("HTTP://EXAMPLE.TEST:80/#chapter"));
        assert_eq!(
            catalog
                .toggle("http://example.test/#chapter", "ignored")
                .unwrap(),
            BookmarkChange::Removed
        );
        assert!(catalog.is_empty());
    }

    #[test]
    fn transaction_commits_only_after_repository_save() {
        let mut catalog = BookmarkCatalog::new();
        let mut repository = MemoryBookmarkRepository::default();
        transact(&mut catalog, &mut repository, |candidate| {
            candidate.toggle("http://example.test/", "Example")
        })
        .unwrap();
        assert_eq!(catalog, *repository.stored());

        repository.fail_saves_with("disk full");
        let before = catalog.clone();
        assert!(transact(&mut catalog, &mut repository, |candidate| {
            candidate.toggle("http://example.test/two", "Two")
        })
        .is_err());
        assert_eq!(catalog, before);
        assert_eq!(repository.stored(), &before);
    }
}
