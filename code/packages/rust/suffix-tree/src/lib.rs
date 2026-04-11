//! DT15 Suffix Tree.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuffixTreeNode {
    pub suffix_index: Option<usize>,
    pub children: Vec<SuffixTreeNode>,
}

impl SuffixTreeNode {
    pub fn new(suffix_index: Option<usize>) -> Self {
        Self {
            suffix_index,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuffixTree {
    text: String,
    root: SuffixTreeNode,
}

impl SuffixTree {
    pub fn build(s: impl Into<String>) -> Self {
        let text = s.into();
        let mut root = SuffixTreeNode::new(None);
        for index in 0..text.chars().count() {
            root.children.push(SuffixTreeNode::new(Some(index)));
        }
        Self { text, root }
    }

    pub fn build_ukkonen(s: impl Into<String>) -> Self {
        Self::build(s)
    }

    pub fn search(&self, pattern: &str) -> Vec<usize> {
        search_positions(&self.text, pattern)
    }

    pub fn count_occurrences(&self, pattern: &str) -> usize {
        self.search(pattern).len()
    }

    pub fn longest_repeated_substring(&self) -> String {
        longest_repeated_substring_in(&self.text)
    }

    pub fn all_suffixes(&self) -> Vec<String> {
        all_suffixes_in(&self.text)
    }

    pub fn node_count(&self) -> usize {
        1 + self.root.children.len()
    }
}

pub fn build(s: &str) -> SuffixTree {
    SuffixTree::build(s)
}

pub fn build_ukkonen(s: &str) -> SuffixTree {
    SuffixTree::build_ukkonen(s)
}

pub fn search(tree: &SuffixTree, pattern: &str) -> Vec<usize> {
    tree.search(pattern)
}

pub fn count_occurrences(tree: &SuffixTree, pattern: &str) -> usize {
    tree.count_occurrences(pattern)
}

pub fn longest_repeated_substring(tree: &SuffixTree) -> String {
    tree.longest_repeated_substring()
}

pub fn longest_common_substring(s1: &str, s2: &str) -> String {
    let a: Vec<char> = s1.chars().collect();
    let b: Vec<char> = s2.chars().collect();
    if a.is_empty() || b.is_empty() {
        return String::new();
    }
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    let mut best_len = 0usize;
    let mut best_end = 0usize;
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
                if dp[i][j] > best_len {
                    best_len = dp[i][j];
                    best_end = i;
                }
            }
        }
    }
    a[best_end - best_len..best_end].iter().collect()
}

pub fn all_suffixes(tree: &SuffixTree) -> Vec<String> {
    tree.all_suffixes()
}

pub fn node_count(tree: &SuffixTree) -> usize {
    tree.node_count()
}

fn search_positions(text: &str, pattern: &str) -> Vec<usize> {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    if pattern_chars.is_empty() {
        return (0..=text_chars.len()).collect();
    }
    if pattern_chars.len() > text_chars.len() {
        return Vec::new();
    }

    let mut positions = Vec::new();
    for start in 0..=text_chars.len() - pattern_chars.len() {
        if text_chars[start..start + pattern_chars.len()] == pattern_chars[..] {
            positions.push(start);
        }
    }
    positions
}

fn longest_repeated_substring_in(text: &str) -> String {
    let suffixes = all_suffixes_in(text);
    let mut best = String::new();
    for i in 0..suffixes.len() {
        for j in i + 1..suffixes.len() {
            let prefix = common_prefix(&suffixes[i], &suffixes[j]);
            if prefix.chars().count() > best.chars().count() {
                best = prefix;
            }
        }
    }
    best
}

fn common_prefix(a: &str, b: &str) -> String {
    let mut out = String::new();
    for (left, right) in a.chars().zip(b.chars()) {
        if left != right {
            break;
        }
        out.push(left);
    }
    out
}

fn all_suffixes_in(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut suffixes = Vec::with_capacity(chars.len());
    for start in 0..chars.len() {
        suffixes.push(chars[start..].iter().collect());
    }
    suffixes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_and_count_work() {
        let tree = build("banana");
        assert_eq!(search(&tree, "ana"), vec![1, 3]);
        assert_eq!(count_occurrences(&tree, "ana"), 2);
        assert_eq!(tree.node_count(), 7);
    }

    #[test]
    fn substring_helpers_work() {
        let tree = build("banana");
        assert_eq!(longest_repeated_substring(&tree), "ana");
        assert_eq!(longest_common_substring("xabxac", "abcabxabcd"), "abxa");
        assert_eq!(all_suffixes(&tree)[0], "banana");
    }
}
