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

use std::fmt;
use std::sync::Arc;

/// A hierarchical name. See the module documentation.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name(Arc<NameNode>);

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
enum NameNode {
    Anonymous,
    Str(Name, String),
    Num(Name, u64),
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
        Name(Arc::new(NameNode::Anonymous))
    }

    /// Append a string component: `Name::str(&Name::str(&anon, "Nat"), "add")` is `Nat.add`.
    pub fn str(prefix: &Name, component: &str) -> Name {
        Name(Arc::new(NameNode::Str(
            prefix.clone(),
            component.to_string(),
        )))
    }

    /// Append a numeric component. Note `Name::num(p, 3) != Name::str(p, "3")`.
    pub fn num(prefix: &Name, component: u64) -> Name {
        Name(Arc::new(NameNode::Num(prefix.clone(), component)))
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
        matches!(*self.0, NameNode::Anonymous)
    }

    /// The components from the root outwards (empty for the anonymous name).
    pub fn parts(&self) -> Vec<NamePart> {
        let mut out = Vec::new();
        let mut cur = self;
        loop {
            match &*cur.0 {
                NameNode::Anonymous => break,
                NameNode::Str(p, s) => {
                    out.push(NamePart::Str(s.clone()));
                    cur = p;
                }
                NameNode::Num(p, n) => {
                    out.push(NamePart::Num(*n));
                    cur = p;
                }
            }
        }
        out.reverse();
        out
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
