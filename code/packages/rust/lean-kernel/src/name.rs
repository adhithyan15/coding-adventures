//! Hierarchical names.
//!
//! Every constant, universe parameter and structure in Lean is identified by a
//! *hierarchical name*: a path such as `Nat.add` or `List.rec` or `_private.Foo.0.bar`.
//! A name is a (possibly empty) chain of components, each either a string or a number:
//!
//! ```text
//!   Name ::= anonymous            -- the empty path
//!          | Name . "text"        -- a string component
//!          | Name . 42            -- a numeric component
//! ```
//!
//! Numeric components exist because Lean generates names like `_uniq.1234` and
//! `Foo._proof_3`; they are *not* the same as the string `"3"`, and the kernel must keep
//! the two apart or two different constants could be confused. That is why the
//! constructor is an enum rather than a joined string.
//!
//! Names are immutable and shared (`Arc`), so cloning is a pointer copy; equality,
//! ordering and hashing are structural.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A hierarchical name. See the module documentation.
///
/// # Why the hash is cached (security review of PIF-002)
///
/// A name is a chain of components, and a hostile producer can make one legal name megabytes
/// long. The derived `Hash`/`Eq` walked the whole chain and every byte on *every* call, and
/// the expression store, the environment and every `HashMap<Name, _>` call them for each
/// lookup: millions of references to one long name cost millions of megabyte-sized hashes.
/// So each node stores, once at construction, a structural hash of the whole chain
/// (computed from its prefix's cached hash plus its own component, so construction costs
/// only the new component). `Hash` writes that one `u64`; `Eq` is pointer equality, then a
/// hash comparison, and only a genuine hash match walks the components.
#[derive(Clone)]
pub struct Name(Arc<NameNode>);

struct NameNode {
    kind: Kind,
    /// Structural hash of the whole chain, fixed at construction.
    hash: u64,
}

enum Kind {
    Anonymous,
    Str(Name, String),
    Num(Name, u64),
}

fn mix(tag: u8, prefix: u64, bytes: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (tag, prefix, bytes).hash(&mut h);
    h.finish()
}

impl Name {
    fn node(kind: Kind, hash: u64) -> Name {
        Name(Arc::new(NameNode { kind, hash }))
    }
    /// The chain from this node down to the anonymous root, outermost first.
    fn chain(&self) -> Vec<&Kind> {
        let mut out = vec![];
        let mut cur = self;
        loop {
            out.push(&cur.0.kind);
            match &cur.0.kind {
                Kind::Anonymous => return out,
                Kind::Str(p, _) | Kind::Num(p, _) => cur = p,
            }
        }
    }
}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash);
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        if Arc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        if self.0.hash != other.0.hash {
            return false;
        }
        // equal hashes: confirm by walking (iterative: no recursion on depth)
        let (a, b) = (self.chain(), other.chain());
        a.len() == b.len()
            && a.iter().zip(&b).all(|(x, y)| match (x, y) {
                (Kind::Anonymous, Kind::Anonymous) => true,
                (Kind::Str(_, s), Kind::Str(_, t)) => s == t,
                (Kind::Num(_, m), Kind::Num(_, n)) => m == n,
                _ => false,
            })
    }
}
impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Name) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The order the derived implementation gave: the variant of the last component
/// (anonymous, string, number), then the prefix, then the component. Unrolled: ranks from
/// the outside in, then components from the root out. Consistent with `Eq`.
impl Ord for Name {
    fn cmp(&self, other: &Name) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }
        let rank = |k: &Kind| match k {
            Kind::Anonymous => 0,
            Kind::Str(..) => 1,
            Kind::Num(..) => 2,
        };
        let (a, b) = (self.chain(), other.chain());
        for (x, y) in a.iter().zip(&b) {
            let o = rank(x).cmp(&rank(y));
            if o != Ordering::Equal {
                return o;
            }
        }
        for (x, y) in a.iter().rev().zip(b.iter().rev()) {
            let o = match (x, y) {
                (Kind::Str(_, s), Kind::Str(_, t)) => s.cmp(t),
                (Kind::Num(_, m), Kind::Num(_, n)) => m.cmp(n),
                _ => Ordering::Equal,
            };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    }
}

/// One component of a [`Name`], as returned by [`Name::parts`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamePart {
    /// A string component.
    Str(String),
    /// A numeric component.
    Num(u64),
}

impl Name {
    /// The empty name.
    pub fn anonymous() -> Name {
        Name::node(Kind::Anonymous, mix(0, 0, &[]))
    }

    /// Append a string component: `Name::str(&Name::str(&anon, "Nat"), "add")` is `Nat.add`.
    pub fn str(prefix: &Name, component: &str) -> Name {
        let h = mix(1, prefix.0.hash, component.as_bytes());
        Name::node(Kind::Str(prefix.clone(), component.to_string()), h)
    }

    /// Append a numeric component. Note `Name::num(p, 3) != Name::str(p, "3")`.
    pub fn num(prefix: &Name, component: u64) -> Name {
        let h = mix(2, prefix.0.hash, &component.to_le_bytes());
        Name::node(Kind::Num(prefix.clone(), component), h)
    }

    /// Build a name from dot-separated *string* components (`"Nat.add"`). Every component
    /// becomes a string component, including all-digit ones; use [`Name::num`] to build
    /// numeric components. The empty string is the anonymous name.
    pub fn from_dotted(s: &str) -> Name {
        if s.is_empty() {
            return Name::anonymous();
        }
        s.split('.')
            .fold(Name::anonymous(), |acc, c| Name::str(&acc, c))
    }

    /// True for the empty name.
    pub fn is_anonymous(&self) -> bool {
        matches!(self.0.kind, Kind::Anonymous)
    }

    /// The components from the root outwards (empty for the anonymous name).
    pub fn parts(&self) -> Vec<NamePart> {
        let mut out = Vec::new();
        let mut cur = self;
        loop {
            match &cur.0.kind {
                Kind::Anonymous => break,
                Kind::Str(p, s) => {
                    out.push(NamePart::Str(s.clone()));
                    cur = p;
                }
                Kind::Num(p, n) => {
                    out.push(NamePart::Num(*n));
                    cur = p;
                }
            }
        }
        out.reverse();
        out
    }
}

/// Drop iteratively so a very long name cannot overflow the stack.
impl Drop for Name {
    fn drop(&mut self) {
        fn take_prefix(a: &mut Arc<NameNode>) -> Option<Name> {
            match std::mem::replace(&mut Arc::get_mut(a)?.kind, Kind::Anonymous) {
                Kind::Str(p, _) | Kind::Num(p, _) => Some(p),
                Kind::Anonymous => None,
            }
        }
        let mut cur = take_prefix(&mut self.0);
        while let Some(mut n) = cur {
            cur = take_prefix(&mut n.0);
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_anonymous() {
            return write!(f, "[anonymous]");
        }
        for (i, part) in self.parts().iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            match part {
                NamePart::Str(s) => write!(f, "{s}")?,
                NamePart::Num(n) => write!(f, "{n}")?,
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{self}")
    }
}

#[cfg(test)]
#[path = "name_tests.rs"]
mod tests;
