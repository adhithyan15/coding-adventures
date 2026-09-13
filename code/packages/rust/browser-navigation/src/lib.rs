//! Host-neutral browser navigation and visited-link state.

use std::collections::BTreeSet;

use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";

/// Stable identity for one session-history entry.
///
/// URLs are not entry identities: the same resource can occur more than once
/// with different scroll and form state. The identifier survives traversal
/// and redirects, but a fresh navigation always allocates a new value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NavigationEntryId(u64);

impl NavigationEntryId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// In-memory browser navigation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationHistory {
    home_url: String,
    back_stack: Vec<String>,
    back_entry_ids: Vec<NavigationEntryId>,
    current_url: Option<String>,
    current_entry_id: Option<NavigationEntryId>,
    forward_stack: Vec<String>,
    forward_entry_ids: Vec<NavigationEntryId>,
    next_entry_id: u64,
}

impl NavigationHistory {
    pub fn new(home_url: impl Into<String>) -> Self {
        Self {
            home_url: home_url.into(),
            back_stack: Vec::new(),
            back_entry_ids: Vec::new(),
            current_url: None,
            current_entry_id: None,
            forward_stack: Vec::new(),
            forward_entry_ids: Vec::new(),
            next_entry_id: 1,
        }
    }

    pub fn with_current(home_url: impl Into<String>, current_url: impl Into<String>) -> Self {
        let mut history = Self::new(home_url);
        history.current_url = Some(current_url.into());
        history.current_entry_id = Some(history.allocate_entry_id());
        history
    }

    pub fn home_url(&self) -> &str {
        &self.home_url
    }

    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    pub const fn current_entry_id(&self) -> Option<NavigationEntryId> {
        self.current_entry_id
    }

    pub fn back_stack(&self) -> &[String] {
        &self.back_stack
    }

    pub fn forward_stack(&self) -> &[String] {
        &self.forward_stack
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    /// Record a new navigation and clear stale forward history.
    pub fn navigate(&mut self, url: impl Into<String>) -> &str {
        if let Some(current) = self.current_url.take() {
            self.back_stack.push(current);
            self.back_entry_ids.push(
                self.current_entry_id
                    .take()
                    .expect("current URL must have an entry identifier"),
            );
        }
        self.current_url = Some(url.into());
        self.current_entry_id = Some(self.allocate_entry_id());
        self.forward_stack.clear();
        self.forward_entry_ids.clear();
        self.current_url.as_deref().unwrap_or("")
    }

    pub fn back(&mut self) -> Option<&str> {
        let previous = self.back_stack.pop()?;
        let previous_id = self
            .back_entry_ids
            .pop()
            .expect("history URL and identifier stacks must stay aligned");
        if let Some(current) = self.current_url.replace(previous) {
            self.forward_stack.push(current);
            self.forward_entry_ids.push(
                self.current_entry_id
                    .replace(previous_id)
                    .expect("current URL must have an entry identifier"),
            );
        } else {
            self.current_entry_id = Some(previous_id);
        }
        self.current_url()
    }

    pub fn forward(&mut self) -> Option<&str> {
        let next = self.forward_stack.pop()?;
        let next_id = self
            .forward_entry_ids
            .pop()
            .expect("history URL and identifier stacks must stay aligned");
        if let Some(current) = self.current_url.replace(next) {
            self.back_stack.push(current);
            self.back_entry_ids.push(
                self.current_entry_id
                    .replace(next_id)
                    .expect("current URL must have an entry identifier"),
            );
        } else {
            self.current_entry_id = Some(next_id);
        }
        self.current_url()
    }

    pub fn home(&mut self) -> &str {
        self.navigate(self.home_url.clone())
    }

    /// Return the URL that a host should fetch again without changing history.
    pub fn reload(&self) -> Option<&str> {
        self.current_url()
    }

    /// Replace the current entry after a redirect without creating history.
    pub fn replace_current(&mut self, final_url: impl Into<String>) -> Option<&str> {
        self.current_url.as_ref()?;
        self.current_url = Some(final_url.into());
        self.current_url()
    }

    fn allocate_entry_id(&mut self) -> NavigationEntryId {
        let id = NavigationEntryId(self.next_entry_id);
        self.next_entry_id = self.next_entry_id.wrapping_add(1).max(1);
        id
    }
}

/// Canonical document-resource identity used by visited-link state.
///
/// Fragments are intentionally excluded: navigating between anchors in one
/// document must not create separate visited identities for the same fetched
/// resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisitedUrl(String);

impl VisitedUrl {
    pub fn parse(url: &str) -> Result<Self, UrlError> {
        let mut canonical = Url::parse(url)?.canonicalize()?;
        canonical.fragment = None;
        Ok(Self(canonical.to_url_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for VisitedUrl {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Session-scoped visited-link state with canonical URL membership.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VisitedLinks {
    urls: BTreeSet<VisitedUrl>,
}

impl VisitedLinks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a URL, returning whether it was newly inserted.
    pub fn record(&mut self, url: &str) -> Result<bool, UrlError> {
        Ok(self.urls.insert(VisitedUrl::parse(url)?))
    }

    /// Invalid URLs are never considered visited.
    pub fn contains(&self, url: &str) -> bool {
        VisitedUrl::parse(url)
            .map(|url| self.urls.contains(&url))
            .unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.urls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.urls.is_empty()
    }

    pub fn clear(&mut self) {
        self.urls.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &VisitedUrl> {
        self.urls.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_walks_back_forward_home_reload_and_redirects() {
        let mut history = NavigationHistory::new("http://home.test/");
        history.navigate("http://example.test/a");
        history.navigate("http://example.test/b");
        assert_eq!(history.back(), Some("http://example.test/a"));
        assert_eq!(history.forward(), Some("http://example.test/b"));
        assert_eq!(history.reload(), Some("http://example.test/b"));
        assert_eq!(history.home(), "http://home.test/");
        assert_eq!(
            history.replace_current("http://home.test/final"),
            Some("http://home.test/final")
        );
    }

    #[test]
    fn navigating_after_back_clears_forward_history() {
        let mut history = NavigationHistory::new("http://home.test/");
        history.navigate("http://example.test/a");
        history.navigate("http://example.test/b");
        history.back();
        history.navigate("http://example.test/c");
        assert!(!history.can_go_forward());
        assert_eq!(history.back_stack(), &["http://example.test/a".to_string()]);
    }

    #[test]
    fn duplicate_urls_keep_distinct_stable_entry_identifiers() {
        let mut history = NavigationHistory::new("http://home.test/");
        history.navigate("http://example.test/repeated");
        let first = history.current_entry_id().unwrap();
        history.navigate("http://example.test/repeated");
        let second = history.current_entry_id().unwrap();
        assert_ne!(first, second);

        history.back();
        assert_eq!(history.current_entry_id(), Some(first));
        history.forward();
        assert_eq!(history.current_entry_id(), Some(second));
        history.replace_current("http://example.test/final");
        assert_eq!(history.current_entry_id(), Some(second));
    }

    #[test]
    fn visited_identity_normalizes_resource_urls() {
        let canonical = VisitedUrl::parse("HTTP://Example.TEST:80/a/../%7euser?q=%7b#one")
            .expect("URL should canonicalize");
        assert_eq!(canonical.as_str(), "http://example.test/~user?q=%7B");
    }

    #[test]
    fn visited_membership_ignores_fragments_and_canonical_spelling() {
        let mut visited = VisitedLinks::new();
        assert!(visited
            .record("HTTP://Example.TEST:80/guide/../index.html#intro")
            .expect("URL should record"));
        assert!(visited.contains("http://example.test/index.html#details"));
        assert!(!visited
            .record("http://example.test/index.html")
            .expect("equivalent URL should record"));
        assert_eq!(visited.len(), 1);
    }

    #[test]
    fn visited_membership_preserves_queries() {
        let mut visited = VisitedLinks::new();
        visited
            .record("http://example.test/search?q=one")
            .expect("URL should record");
        assert!(visited.contains("http://example.test/search?q=one#result"));
        assert!(!visited.contains("http://example.test/search?q=two"));
    }

    #[test]
    fn invalid_urls_do_not_mutate_or_match() {
        let mut visited = VisitedLinks::new();
        assert!(visited.record("not a URL").is_err());
        assert!(!visited.contains("not a URL"));
        assert!(visited.is_empty());
    }
}
