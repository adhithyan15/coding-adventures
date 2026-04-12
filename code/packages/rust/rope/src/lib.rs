//! DT16 Rope.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafNode {
    pub chunk: String,
}

impl LeafNode {
    pub fn new(chunk: impl Into<String>) -> Self {
        Self {
            chunk: chunk.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InternalNode {
    pub weight: usize,
    pub left: Box<RopeNode>,
    pub right: Box<RopeNode>,
}

impl InternalNode {
    pub fn new(weight: usize, left: RopeNode, right: RopeNode) -> Self {
        Self {
            weight,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RopeNode {
    Leaf(LeafNode),
    Internal(InternalNode),
}

impl RopeNode {
    fn depth(&self) -> usize {
        match self {
            Self::Leaf(_) => 0,
            Self::Internal(internal) => 1 + internal.left.depth().max(internal.right.depth()),
        }
    }

    fn is_balanced(&self) -> bool {
        match self {
            Self::Leaf(_) => true,
            Self::Internal(internal) => {
                let left_depth = internal.left.depth();
                let right_depth = internal.right.depth();
                (left_depth as isize - right_depth as isize).abs() <= 1
                    && internal.left.is_balanced()
                    && internal.right.is_balanced()
            }
        }
    }

    fn to_string(&self, out: &mut String) {
        match self {
            Self::Leaf(leaf) => out.push_str(&leaf.chunk),
            Self::Internal(internal) => {
                internal.left.to_string(out);
                internal.right.to_string(out);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rope {
    root: Option<Box<RopeNode>>,
    len: usize,
}

impl Default for Rope {
    fn default() -> Self {
        Self::empty()
    }
}

impl Rope {
    pub fn empty() -> Self {
        Self { root: None, len: 0 }
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        let s = s.into();
        if s.is_empty() {
            Self::empty()
        } else {
            let len = s.chars().count();
            Self {
                root: Some(Box::new(RopeNode::Leaf(LeafNode::new(s)))),
                len,
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn to_string(&self) -> String {
        to_string(self)
    }

    pub fn index(&self, i: usize) -> Option<char> {
        index(self, i)
    }

    pub fn substring(&self, start: usize, end: usize) -> String {
        substring(self, start, end)
    }

    pub fn depth(&self) -> usize {
        depth(self)
    }

    pub fn is_balanced(&self) -> bool {
        is_balanced(self)
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub fn rope_from_string(s: impl Into<String>) -> Rope {
    Rope::from_string(s)
}

pub fn rope_empty() -> Rope {
    Rope::empty()
}

pub fn length(rope: &Rope) -> usize {
    rope.len()
}

pub fn index(rope: &Rope, i: usize) -> Option<char> {
    let chars: Vec<char> = rope.to_string().chars().collect();
    chars.get(i).copied()
}

pub fn rope_index(rope: &Rope, i: usize) -> Option<char> {
    index(rope, i)
}

pub fn to_string(rope: &Rope) -> String {
    let mut out = String::new();
    if let Some(root) = rope.root.as_deref() {
        root.to_string(&mut out);
    }
    out
}

pub fn concat(left: Rope, right: Rope) -> Rope {
    match (left.root, right.root) {
        (None, other) => Rope {
            root: other,
            len: right.len,
        },
        (other, None) => Rope {
            root: other,
            len: left.len,
        },
        (Some(left_root), Some(right_root)) => Rope {
            root: Some(Box::new(RopeNode::Internal(InternalNode::new(
                left.len,
                *left_root,
                *right_root,
            )))),
            len: left.len + right.len,
        },
    }
}

pub fn split(rope: Rope, i: usize) -> (Rope, Rope) {
    let text = to_string(&rope);
    let chars: Vec<char> = text.chars().collect();
    let split_at = i.min(chars.len());
    let left: String = chars[..split_at].iter().collect();
    let right: String = chars[split_at..].iter().collect();
    (Rope::from_string(left), Rope::from_string(right))
}

pub fn insert(rope: Rope, i: usize, s: &str) -> Rope {
    let (left, right) = split(rope, i);
    concat(concat(left, Rope::from_string(s)), right)
}

pub fn delete(rope: Rope, start: usize, length: usize) -> Rope {
    let text = to_string(&rope);
    let chars: Vec<char> = text.chars().collect();
    let start = start.min(chars.len());
    let end = (start + length).min(chars.len());
    let left: String = chars[..start].iter().collect();
    let right: String = chars[end..].iter().collect();
    concat(Rope::from_string(left), Rope::from_string(right))
}

pub fn substring(rope: &Rope, start: usize, end: usize) -> String {
    let chars: Vec<char> = rope.to_string().chars().collect();
    let start = start.min(chars.len());
    let end = end.min(chars.len());
    if start >= end {
        return String::new();
    }
    chars[start..end].iter().collect()
}

pub fn depth(rope: &Rope) -> usize {
    rope.root.as_deref().map(|node| node.depth()).unwrap_or(0)
}

pub fn is_balanced(rope: &Rope) -> bool {
    rope.root.as_deref().map(|node| node.is_balanced()).unwrap_or(true)
}

pub fn rebalance(rope: Rope) -> Rope {
    let text = to_string(&rope);
    build_balanced(text)
}

fn build_balanced(text: String) -> Rope {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Rope::empty();
    }
    build_balanced_from_chars(&chars)
}

fn build_balanced_from_chars(chars: &[char]) -> Rope {
    if chars.is_empty() {
        return Rope::empty();
    }
    if chars.len() <= 1 {
        return Rope::from_string(chars.iter().collect::<String>());
    }
    let mid = chars.len() / 2;
    let left = build_balanced_from_chars(&chars[..mid]);
    let right = build_balanced_from_chars(&chars[mid..]);
    concat(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_split_and_index_work() {
        let rope = concat(rope_from_string("hello"), rope_from_string(" world"));
        assert_eq!(length(&rope), 11);
        assert_eq!(rope_index(&rope, 1), Some('e'));
        let (left, right) = split(rope.clone(), 5);
        assert_eq!(to_string(&left), "hello");
        assert_eq!(to_string(&right), " world");
    }

    #[test]
    fn editing_and_rebalance_work() {
        let rope = insert(rope_from_string("ace"), 1, "b");
        let rope = insert(rope, 3, "d");
        assert_eq!(to_string(&rope), "abcde");
        let rope = delete(rope, 1, 2);
        assert_eq!(to_string(&rope), "ade");
        let rope = rebalance(concat(rope_from_string("ab"), rope_from_string("cdef")));
        assert!(is_balanced(&rope));
        assert!(depth(&rope) <= 3);
        assert_eq!(substring(&rope, 1, 4), "bcd");
    }
}
