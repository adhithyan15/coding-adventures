//! Compressed trie (radix tree / Patricia trie) for string-keyed prefix search.

use std::collections::BTreeMap;
use std::fmt;

fn common_prefix_len(a: &str, b: &str) -> usize {
    let mut len = 0;
    let mut a_iter = a.chars();
    let mut b_iter = b.chars();

    loop {
        match (a_iter.next(), b_iter.next()) {
            (Some(left), Some(right)) if left == right => len += left.len_utf8(),
            _ => break,
        }
    }

    len
}

fn first_char(s: &str) -> char {
    s.chars().next().expect("edge labels are never empty")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RadixEdge<V> {
    label: String,
    child: RadixNode<V>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadixNode<V> {
    is_end: bool,
    value: Option<V>,
    children: BTreeMap<char, RadixEdge<V>>,
}

impl<V> RadixNode<V> {
    fn new() -> Self {
        Self {
            is_end: false,
            value: None,
            children: BTreeMap::new(),
        }
    }

    fn leaf(value: V) -> Self {
        Self {
            is_end: true,
            value: Some(value),
            children: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadixTree<V> {
    root: RadixNode<V>,
    size: usize,
}

impl<V> Default for RadixTree<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> RadixTree<V> {
    pub fn new() -> Self {
        Self {
            root: RadixNode::new(),
            size: 0,
        }
    }

    pub fn insert(&mut self, key: &str, value: V) {
        if Self::insert_recursive(&mut self.root, key, value) {
            self.size += 1;
        }
    }

    pub fn search(&self, key: &str) -> Option<&V> {
        let mut node = &self.root;
        let mut remaining = key;

        while !remaining.is_empty() {
            let first = first_char(remaining);
            let edge = node.children.get(&first)?;
            let common_len = common_prefix_len(remaining, &edge.label);
            if common_len < edge.label.len() {
                return None;
            }
            remaining = &remaining[common_len..];
            node = &edge.child;
        }

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
        let (deleted, _) = Self::delete_recursive(&mut self.root, key);
        if deleted {
            self.size -= 1;
        }
        deleted
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        if prefix.is_empty() {
            return self.size > 0;
        }

        let mut node = &self.root;
        let mut remaining = prefix;

        while !remaining.is_empty() {
            let first = first_char(remaining);
            let Some(edge) = node.children.get(&first) else {
                return false;
            };
            let common_len = common_prefix_len(remaining, &edge.label);
            if common_len == remaining.len() {
                return true;
            }
            if common_len < edge.label.len() {
                return false;
            }
            remaining = &remaining[common_len..];
            node = &edge.child;
        }

        node.is_end || !node.children.is_empty()
    }

    pub fn words_with_prefix(&self, prefix: &str) -> Vec<String> {
        let mut node = &self.root;
        let mut remaining = prefix;
        let mut path = String::new();

        if remaining.is_empty() {
            let mut results = Vec::new();
            Self::collect_keys(&self.root, String::new(), &mut results);
            return results;
        }

        while !remaining.is_empty() {
            let first = first_char(remaining);
            let Some(edge) = node.children.get(&first) else {
                return Vec::new();
            };
            let common_len = common_prefix_len(remaining, &edge.label);
            if common_len == remaining.len() {
                if common_len == edge.label.len() {
                    path.push_str(&edge.label);
                    node = &edge.child;
                    remaining = "";
                } else {
                    let mut subtree_path = path.clone();
                    subtree_path.push_str(&edge.label);
                    let mut results = Vec::new();
                    Self::collect_keys(&edge.child, subtree_path, &mut results);
                    return results;
                }
            } else if common_len < edge.label.len() {
                return Vec::new();
            } else {
                path.push_str(&edge.label);
                remaining = &remaining[common_len..];
                node = &edge.child;
            }
        }

        let mut results = Vec::new();
        Self::collect_keys(node, path, &mut results);
        results
    }

    pub fn longest_prefix_match(&self, key: &str) -> Option<String> {
        let mut node = &self.root;
        let mut remaining = key;
        let mut consumed = 0usize;
        let mut best = if node.is_end {
            Some(String::new())
        } else {
            None
        };

        while !remaining.is_empty() {
            let first = first_char(remaining);
            let Some(edge) = node.children.get(&first) else {
                break;
            };
            let common_len = common_prefix_len(remaining, &edge.label);
            if common_len < edge.label.len() {
                break;
            }
            consumed += common_len;
            remaining = &remaining[common_len..];
            node = &edge.child;
            if node.is_end {
                best = Some(key[..consumed].to_string());
            }
        }

        best
    }

    pub fn to_map(&self) -> BTreeMap<String, V>
    where
        V: Clone,
    {
        let mut result = BTreeMap::new();
        Self::collect_values(&self.root, String::new(), &mut result);
        result
    }

    pub fn keys(&self) -> Vec<String> {
        let mut results = Vec::new();
        Self::collect_keys(&self.root, String::new(), &mut results);
        results
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn node_count(&self) -> usize {
        Self::count_nodes(&self.root)
    }

    fn key_exists(&self, key: &str) -> bool {
        let mut node = &self.root;
        let mut remaining = key;

        while !remaining.is_empty() {
            let first = first_char(remaining);
            let Some(edge) = node.children.get(&first) else {
                return false;
            };
            let common_len = common_prefix_len(remaining, &edge.label);
            if common_len < edge.label.len() {
                return false;
            }
            remaining = &remaining[common_len..];
            node = &edge.child;
        }

        node.is_end
    }

    fn insert_recursive(node: &mut RadixNode<V>, key: &str, value: V) -> bool {
        if key.is_empty() {
            let added = !node.is_end;
            node.is_end = true;
            node.value = Some(value);
            return added;
        }

        let first = first_char(key);
        let Some(mut edge) = node.children.remove(&first) else {
            node.children.insert(
                first,
                RadixEdge {
                    label: key.to_string(),
                    child: RadixNode::leaf(value),
                },
            );
            return true;
        };

        let common_len = common_prefix_len(key, &edge.label);
        if common_len == edge.label.len() {
            let added = Self::insert_recursive(&mut edge.child, &key[common_len..], value);
            node.children.insert(first, edge);
            return added;
        }

        let common = edge.label[..common_len].to_string();
        let label_rest = edge.label[common_len..].to_string();
        let key_rest = key[common_len..].to_string();
        let mut split_node = RadixNode::new();
        split_node.children.insert(
            first_char(&label_rest),
            RadixEdge {
                label: label_rest,
                child: edge.child,
            },
        );

        if key_rest.is_empty() {
            split_node.is_end = true;
            split_node.value = Some(value);
        } else {
            split_node.children.insert(
                first_char(&key_rest),
                RadixEdge {
                    label: key_rest,
                    child: RadixNode::leaf(value),
                },
            );
        }

        node.children.insert(
            first_char(&common),
            RadixEdge {
                label: common,
                child: split_node,
            },
        );
        true
    }

    fn delete_recursive(node: &mut RadixNode<V>, key: &str) -> (bool, bool) {
        if key.is_empty() {
            if !node.is_end {
                return (false, false);
            }
            node.is_end = false;
            node.value = None;
            return (true, !node.is_end && node.children.len() == 1);
        }

        let first = first_char(key);
        let Some(mut edge) = node.children.remove(&first) else {
            return (false, false);
        };

        let common_len = common_prefix_len(key, &edge.label);
        if common_len < edge.label.len() {
            node.children.insert(first, edge);
            return (false, false);
        }

        let (deleted, child_mergeable) =
            Self::delete_recursive(&mut edge.child, &key[common_len..]);
        if !deleted {
            node.children.insert(first, edge);
            return (false, false);
        }

        if child_mergeable {
            let (_, grand_edge) = edge
                .child
                .children
                .into_iter()
                .next()
                .expect("mergeable child must have one edge");
            let merged_label = format!("{}{}", edge.label, grand_edge.label);
            node.children.insert(
                first_char(&merged_label),
                RadixEdge {
                    label: merged_label,
                    child: grand_edge.child,
                },
            );
        } else if !edge.child.is_end && edge.child.children.is_empty() {
            // prune dead child
        } else {
            node.children.insert(first, edge);
        }

        (true, !node.is_end && node.children.len() == 1)
    }

    fn collect_keys(node: &RadixNode<V>, current: String, results: &mut Vec<String>) {
        if node.is_end {
            results.push(current.clone());
        }
        for edge in node.children.values() {
            let mut next = current.clone();
            next.push_str(&edge.label);
            Self::collect_keys(&edge.child, next, results);
        }
    }

    fn collect_values(node: &RadixNode<V>, current: String, result: &mut BTreeMap<String, V>)
    where
        V: Clone,
    {
        if node.is_end {
            if let Some(value) = node.value.clone() {
                result.insert(current.clone(), value);
            }
        }
        for edge in node.children.values() {
            let mut next = current.clone();
            next.push_str(&edge.label);
            Self::collect_values(&edge.child, next, result);
        }
    }

    fn count_nodes(node: &RadixNode<V>) -> usize {
        1 + node
            .children
            .values()
            .map(|edge| Self::count_nodes(&edge.child))
            .sum::<usize>()
    }
}

impl<V: Clone + fmt::Debug> fmt::Display for RadixTree<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let preview: Vec<_> = self.to_map().into_iter().take(5).collect();
        write!(f, "RadixTree({} keys: {:?})", self.size, preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_with(keys: &[&str]) -> RadixTree<usize> {
        let mut tree = RadixTree::new();
        for (index, key) in keys.iter().enumerate() {
            tree.insert(key, index + 1);
        }
        tree
    }

    #[test]
    fn insert_and_search_cover_split_cases() {
        let mut tree = RadixTree::new();
        tree.insert("application", 1);
        tree.insert("apple", 2);
        tree.insert("app", 3);
        tree.insert("apt", 4);
        assert_eq!(tree.search("application"), Some(&1));
        assert_eq!(tree.search("apple"), Some(&2));
        assert_eq!(tree.search("app"), Some(&3));
        assert_eq!(tree.search("apt"), Some(&4));
        assert_eq!(tree.search("appl"), None);
    }

    #[test]
    fn delete_prunes_and_merges() {
        let mut tree = RadixTree::new();
        tree.insert("app", 1);
        tree.insert("apple", 2);
        assert_eq!(tree.node_count(), 3);
        assert!(tree.delete("app"));
        assert_eq!(tree.search("app"), None);
        assert_eq!(tree.search("apple"), Some(&2));
        assert_eq!(tree.node_count(), 2);
    }

    #[test]
    fn starts_with_handles_mid_edge_prefixes() {
        let tree = tree_with(&["searching"]);
        assert!(tree.starts_with("sear"));
        assert!(tree.starts_with("search"));
        assert!(tree.starts_with("searchin"));
        assert!(!tree.starts_with("seek"));
    }

    #[test]
    fn words_with_prefix_are_sorted() {
        let tree = tree_with(&["search", "searcher", "searching", "banana"]);
        assert_eq!(
            tree.words_with_prefix("search"),
            vec!["search", "searcher", "searching"]
        );
    }

    #[test]
    fn longest_prefix_match_returns_most_specific_key() {
        let tree = tree_with(&["a", "ab", "abc", "application"]);
        assert_eq!(tree.longest_prefix_match("abcdef"), Some("abc".to_string()));
        assert_eq!(
            tree.longest_prefix_match("application/json"),
            Some("application".to_string())
        );
        assert_eq!(tree.longest_prefix_match("xyz"), None);
    }

    #[test]
    fn empty_string_keys_are_supported() {
        let mut tree = RadixTree::new();
        tree.insert("", 1);
        tree.insert("a", 2);
        assert_eq!(tree.search(""), Some(&1));
        assert_eq!(tree.longest_prefix_match("xyz"), Some(String::new()));
        assert!(tree.delete(""));
        assert_eq!(tree.search(""), None);
    }

    #[test]
    fn to_map_round_trips_values() {
        let tree = tree_with(&["foo", "bar", "baz"]);
        let map = tree.to_map();
        assert_eq!(map.get("foo"), Some(&1));
        assert_eq!(map.get("bar"), Some(&2));
        assert_eq!(map.get("baz"), Some(&3));
    }

    #[test]
    fn compression_example_has_four_nodes() {
        let tree = tree_with(&["search", "searcher", "searching"]);
        assert_eq!(tree.node_count(), 4);
        assert_eq!(tree.root.children.len(), 1);
    }

    #[test]
    fn keys_are_sorted() {
        let tree = tree_with(&["banana", "apple", "apricot", "app"]);
        assert_eq!(tree.keys(), vec!["app", "apple", "apricot", "banana"]);
    }

    #[test]
    fn display_mentions_size() {
        let tree = tree_with(&["alpha", "beta"]);
        assert!(tree.to_string().contains("2 keys"));
    }
}
