//! Prefix tree (trie) for string keys with prefix operations.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrieNode<V> {
    children: BTreeMap<char, TrieNode<V>>,
    is_end: bool,
    value: Option<V>,
}

impl<V> TrieNode<V> {
    fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            is_end: false,
            value: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trie<V> {
    root: TrieNode<V>,
    size: usize,
}

impl<V> Default for Trie<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> Trie<V> {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            size: 0,
        }
    }

    pub fn insert(&mut self, key: &str, value: V) {
        let mut node = &mut self.root;
        for ch in key.chars() {
            node = node.children.entry(ch).or_insert_with(TrieNode::new);
        }
        if !node.is_end {
            self.size += 1;
        }
        node.is_end = true;
        node.value = Some(value);
    }

    pub fn search(&self, key: &str) -> Option<&V> {
        let node = self.find_node(key)?;
        if node.is_end {
            node.value.as_ref()
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.search(key).is_some() || self.key_exists(key)
    }

    pub fn delete(&mut self, key: &str) -> bool {
        if !self.key_exists(key) {
            return false;
        }
        let chars: Vec<char> = key.chars().collect();
        Self::delete_recursive(&mut self.root, &chars, 0);
        self.size -= 1;
        true
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        if prefix.is_empty() {
            return self.size > 0;
        }
        self.find_node(prefix).is_some()
    }

    pub fn words_with_prefix(&self, prefix: &str) -> Vec<(String, V)>
    where
        V: Clone,
    {
        let Some(node) = self.find_node(prefix) else {
            return Vec::new();
        };
        let mut results = Vec::new();
        Self::collect(node, prefix.to_string(), &mut results);
        results
    }

    pub fn all_words(&self) -> Vec<(String, V)>
    where
        V: Clone,
    {
        let mut results = Vec::new();
        Self::collect(&self.root, String::new(), &mut results);
        results
    }

    pub fn keys(&self) -> Vec<String>
    where
        V: Clone,
    {
        self.all_words().into_iter().map(|(key, _)| key).collect()
    }

    pub fn longest_prefix_match(&self, string: &str) -> Option<(String, V)>
    where
        V: Clone,
    {
        let mut node = &self.root;
        let mut best: Option<(String, V)> = if node.is_end {
            node.value.clone().map(|value| (String::new(), value))
        } else {
            None
        };
        let mut current = String::new();

        for ch in string.chars() {
            let Some(next) = node.children.get(&ch) else {
                break;
            };
            current.push(ch);
            node = next;
            if node.is_end {
                if let Some(value) = node.value.clone() {
                    best = Some((current.clone(), value));
                }
            }
        }

        best
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn is_valid(&self) -> bool {
        Self::count_endpoints(&self.root) == self.size
    }

    fn find_node(&self, key: &str) -> Option<&TrieNode<V>> {
        let mut node = &self.root;
        for ch in key.chars() {
            node = node.children.get(&ch)?;
        }
        Some(node)
    }

    fn key_exists(&self, key: &str) -> bool {
        self.find_node(key).map(|node| node.is_end).unwrap_or(false)
    }

    fn collect(node: &TrieNode<V>, current: String, results: &mut Vec<(String, V)>)
    where
        V: Clone,
    {
        if node.is_end {
            if let Some(value) = node.value.clone() {
                results.push((current.clone(), value));
            }
        }
        for (ch, child) in &node.children {
            let mut next = current.clone();
            next.push(*ch);
            Self::collect(child, next, results);
        }
    }

    fn delete_recursive(node: &mut TrieNode<V>, chars: &[char], depth: usize) -> bool {
        if depth == chars.len() {
            node.is_end = false;
            node.value = None;
            return node.children.is_empty();
        }

        let ch = chars[depth];
        let should_remove_child = match node.children.get_mut(&ch) {
            Some(child) => Self::delete_recursive(child, chars, depth + 1),
            None => false,
        };
        if should_remove_child {
            node.children.remove(&ch);
        }

        node.children.is_empty() && !node.is_end
    }

    fn count_endpoints(node: &TrieNode<V>) -> usize {
        let mut count = usize::from(node.is_end);
        for child in node.children.values() {
            count += Self::count_endpoints(child);
        }
        count
    }
}

impl<V: Clone + fmt::Debug> fmt::Display for Trie<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let words = self.all_words();
        let preview: Vec<_> = words.into_iter().take(5).collect();
        write!(f, "Trie({} keys: {:?})", self.size, preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trie(words: &[&str]) -> Trie<bool> {
        let mut trie = Trie::new();
        for word in words {
            trie.insert(word, true);
        }
        trie
    }

    #[test]
    fn empty_trie_has_no_keys() {
        let trie: Trie<i32> = Trie::new();
        assert_eq!(trie.len(), 0);
        assert!(trie.is_empty());
        assert_eq!(trie.search("anything"), None);
        assert!(!trie.starts_with("a"));
        assert!(trie.is_valid());
    }

    #[test]
    fn insert_and_search_exact_keys() {
        let mut trie = Trie::new();
        trie.insert("hello", 42);
        assert_eq!(trie.search("hello"), Some(&42));
        assert_eq!(trie.search("hell"), None);
        assert_eq!(trie.search("hellos"), None);
    }

    #[test]
    fn insert_updates_existing_key_without_growing_size() {
        let mut trie = Trie::new();
        trie.insert("hello", 1);
        trie.insert("hello", 99);
        assert_eq!(trie.search("hello"), Some(&99));
        assert_eq!(trie.len(), 1);
    }

    #[test]
    fn words_with_prefix_are_lexicographic() {
        let trie = make_trie(&["app", "apple", "apply", "apt"]);
        let results: Vec<_> = trie
            .words_with_prefix("app")
            .into_iter()
            .map(|(word, _)| word)
            .collect();
        assert_eq!(results, vec!["app", "apple", "apply"]);
    }

    #[test]
    fn delete_leaf_and_shared_prefix_cases_work() {
        let mut trie = make_trie(&["app", "apple"]);
        assert!(trie.delete("app"));
        assert!(!trie.contains_key("app"));
        assert!(trie.contains_key("apple"));
        assert_eq!(trie.len(), 1);

        assert!(trie.delete("apple"));
        assert_eq!(trie.len(), 0);
        assert!(trie.root.children.is_empty());
    }

    #[test]
    fn delete_nonexistent_key_returns_false() {
        let mut trie = make_trie(&["apple"]);
        assert!(!trie.delete("xyz"));
        assert!(!trie.delete("app"));
    }

    #[test]
    fn longest_prefix_match_tracks_most_specific_key() {
        let mut trie = Trie::new();
        trie.insert("a", 1);
        trie.insert("ab", 2);
        trie.insert("abc", 3);
        trie.insert("abcd", 4);
        assert_eq!(
            trie.longest_prefix_match("abcde"),
            Some(("abcd".to_string(), 4))
        );
        assert_eq!(trie.longest_prefix_match("xyz"), None);
        assert_eq!(trie.longest_prefix_match("a"), Some(("a".to_string(), 1)));
    }

    #[test]
    fn unicode_and_empty_string_keys_are_supported() {
        let mut trie = Trie::new();
        trie.insert("", "root");
        trie.insert("cafe", "plain");
        trie.insert("cafe\u{301}", "accent-combining");
        trie.insert("caf\u{e9}", "accent-single");
        assert_eq!(trie.search(""), Some(&"root"));
        assert!(trie.starts_with("caf"));
        assert_eq!(trie.search("caf\u{e9}"), Some(&"accent-single"));
    }

    #[test]
    fn keys_and_all_words_are_sorted() {
        let trie = make_trie(&["banana", "app", "apple", "apt"]);
        assert_eq!(trie.keys(), vec!["app", "apple", "apt", "banana"]);
        assert_eq!(trie.all_words().len(), 4);
    }

    #[test]
    fn display_mentions_size() {
        let trie = make_trie(&["app", "apple"]);
        assert!(trie.to_string().contains("2 keys"));
    }
}
