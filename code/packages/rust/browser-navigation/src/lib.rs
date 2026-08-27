//! Host-neutral browser navigation and visited-link state.

use std::collections::BTreeSet;

use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";

/// In-memory browser navigation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationHistory {
    home_url: String,
    back_stack: Vec<String>,
    current_url: Option<String>,
    forward_stack: Vec<String>,
}

impl NavigationHistory {
    pub fn new(home_url: impl Into<String>) -> Self {
        Self {
            home_url: home_url.into(),
            back_stack: Vec::new(),
            current_url: None,
            forward_stack: Vec::new(),
        }
    }

    pub fn with_current(home_url: impl Into<String>, current_url: impl Into<String>) -> Self {
        let mut history = Self::new(home_url);
        history.current_url = Some(current_url.into());
        history
    }

    pub fn home_url(&self) -> &str {
        &self.home_url
    }

    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
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
        }
        self.current_url = Some(url.into());
        self.forward_stack.clear();
        self.current_url.as_deref().unwrap_or("")
    }

    pub fn back(&mut self) -> Option<&str> {
        let previous = self.back_stack.pop()?;
        if let Some(current) = self.current_url.replace(previous) {
            self.forward_stack.push(current);
        }
        self.current_url()
    }

    pub fn forward(&mut self) -> Option<&str> {
        let next = self.forward_stack.pop()?;
        if let Some(current) = self.current_url.replace(next) {
            self.back_stack.push(current);
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
