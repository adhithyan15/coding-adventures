//! DT19 hash set implemented in Rust.
//!
//! The implementation is a zero-cost wrapper around the DT18 hash map:
//! `HashSet<T>` is stored as `HashMap<T, ()>`.

use std::fmt::Debug;

use hash_map::HashMap;

#[derive(Clone, Debug)]
pub struct HashSet<T> {
    map: HashMap<T, ()>,
}

impl<T> Default for HashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HashSet<T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
        }
    }

    pub fn with_options(
        capacity: usize,
        strategy: impl AsRef<str>,
        hash_fn: impl AsRef<str>,
    ) -> Self {
        Self {
            map: HashMap::with_options(capacity, strategy, hash_fn),
        }
    }
}

impl<T: Eq + Debug> HashSet<T> {
    pub fn from_list<I>(elements: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut set = Self::new();
        for element in elements {
            set = set.add(element);
        }
        set
    }

    pub fn from_list_with_options<I>(
        elements: I,
        capacity: usize,
        strategy: impl AsRef<str>,
        hash_fn: impl AsRef<str>,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut set = Self::with_options(capacity, strategy, hash_fn);
        for element in elements {
            set = set.add(element);
        }
        set
    }

    pub fn add(self, element: T) -> Self {
        Self {
            map: self.map.set(element, ()),
        }
    }

    pub fn remove(self, element: &T) -> Self {
        Self {
            map: self.map.delete(element),
        }
    }

    pub fn discard(self, element: &T) -> Self {
        self.remove(element)
    }

    pub fn contains(&self, element: &T) -> bool {
        self.map.has(element)
    }

    pub fn size(&self) -> usize {
        self.map.size()
    }

    pub fn len(&self) -> usize {
        self.size()
    }

    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    pub fn to_list(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.map.keys()
    }

    pub fn union(self, other: Self) -> Self {
        let capacity = self.size() + other.size();
        let mut result = HashSet {
            map: HashMap::with_hash_fn(
                capacity.max(1),
                self.map.strategy(),
                self.map.hash_algorithm(),
            ),
        };
        for (element, ()) in self.map.into_entries() {
            result = result.add(element);
        }
        for (element, ()) in other.map.into_entries() {
            result = result.add(element);
        }
        result
    }

    pub fn intersection(self, other: Self) -> Self
    where
        T: Clone,
    {
        let capacity = self.size().min(other.size()).max(1);
        let mut result = HashSet {
            map: HashMap::with_hash_fn(
                capacity,
                self.map.strategy(),
                self.map.hash_algorithm(),
            ),
        };
        let (smaller, larger) = if self.size() <= other.size() {
            (self, other)
        } else {
            (other, self)
        };
        for (element, ()) in smaller.map.into_entries() {
            if larger.contains(&element) {
                result = result.add(element);
            }
        }
        result
    }

    pub fn difference(self, other: Self) -> Self {
        let mut result = HashSet {
            map: HashMap::with_hash_fn(
                self.size().max(1),
                self.map.strategy(),
                self.map.hash_algorithm(),
            ),
        };
        for (element, ()) in self.map.into_entries() {
            if !other.contains(&element) {
                result = result.add(element);
            }
        }
        result
    }

    pub fn symmetric_difference(self, other: Self) -> Self
    where
        T: Clone,
    {
        let capacity = self.size() + other.size();
        let mut result = HashSet {
            map: HashMap::with_hash_fn(
                capacity.max(1),
                self.map.strategy(),
                self.map.hash_algorithm(),
            ),
        };
        let left_entries = self.to_list();
        let right_entries = other.to_list();
        for element in left_entries.iter().cloned() {
            if !right_entries.contains(&element) {
                result = result.add(element);
            }
        }
        for element in right_entries {
            if !left_entries.contains(&element) {
                result = result.add(element);
            }
        }
        result
    }

    pub fn is_subset(&self, other: &Self) -> bool
    where
        T: Clone,
    {
        if self.size() > other.size() {
            return false;
        }
        for element in self.to_list() {
            if !other.contains(&element) {
                return false;
            }
        }
        true
    }

    pub fn is_superset(&self, other: &Self) -> bool
    where
        T: Clone,
    {
        other.is_subset(self)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool
    where
        T: Clone,
    {
        let (smaller, larger) = if self.size() <= other.size() {
            (self, other)
        } else {
            (other, self)
        };
        for element in smaller.to_list() {
            if larger.contains(&element) {
                return false;
            }
        }
        true
    }

    pub fn equals(&self, other: &Self) -> bool
    where
        T: Clone,
    {
        self.size() == other.size() && self.is_subset(other)
    }
}

pub fn from_list<T, I>(elements: I) -> HashSet<T>
where
    T: Eq + Debug,
    I: IntoIterator<Item = T>,
{
    HashSet::from_list(elements)
}

pub fn add<T: Eq + Debug>(set: HashSet<T>, element: T) -> HashSet<T> {
    set.add(element)
}

pub fn remove<T: Eq + Debug>(set: HashSet<T>, element: &T) -> HashSet<T> {
    set.remove(element)
}

pub fn discard<T: Eq + Debug>(set: HashSet<T>, element: &T) -> HashSet<T> {
    set.discard(element)
}

pub fn contains<T: Eq + Debug>(set: &HashSet<T>, element: &T) -> bool {
    set.contains(element)
}

pub fn union<T: Eq + Debug>(set: HashSet<T>, other: HashSet<T>) -> HashSet<T> {
    set.union(other)
}

pub fn intersection<T>(set: HashSet<T>, other: HashSet<T>) -> HashSet<T>
where
    T: Eq + Debug + Clone,
{
    set.intersection(other)
}

pub fn difference<T: Eq + Debug>(set: HashSet<T>, other: HashSet<T>) -> HashSet<T> {
    set.difference(other)
}

pub fn symmetric_difference<T>(set: HashSet<T>, other: HashSet<T>) -> HashSet<T>
where
    T: Eq + Debug + Clone,
{
    set.symmetric_difference(other)
}

pub fn is_subset<T>(set: &HashSet<T>, other: &HashSet<T>) -> bool
where
    T: Eq + Debug + Clone,
{
    set.is_subset(other)
}

pub fn is_superset<T>(set: &HashSet<T>, other: &HashSet<T>) -> bool
where
    T: Eq + Debug + Clone,
{
    set.is_superset(other)
}

pub fn is_disjoint<T>(set: &HashSet<T>, other: &HashSet<T>) -> bool
where
    T: Eq + Debug + Clone,
{
    set.is_disjoint(other)
}

pub fn equals<T>(set: &HashSet<T>, other: &HashSet<T>) -> bool
where
    T: Eq + Debug + Clone,
{
    set.equals(other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_membership_works() {
        let set = HashSet::from_list([1, 2, 3]);
        assert!(set.contains(&1));
        assert!(!set.contains(&4));
        assert_eq!(set.size(), 3);
    }

    #[test]
    fn duplicates_are_ignored() {
        let set = HashSet::from_list([1, 1, 2, 2, 3]);
        assert_eq!(set.size(), 3);
    }

    #[test]
    fn add_and_remove_return_new_sets() {
        let set = HashSet::from_list([1, 2]);
        let added = set.add(3);
        assert_eq!(added.size(), 3);
        let removed = added.remove(&2);
        assert_eq!(removed.size(), 2);
        assert!(!removed.contains(&2));
    }

    #[test]
    fn set_algebra_works() {
        let a = HashSet::from_list([1, 2, 3, 4, 5]);
        let b = HashSet::from_list([3, 4, 5, 6, 7]);
        assert_eq!(a.clone().union(b.clone()).to_list().len(), 7);
        assert_eq!(a.clone().intersection(b.clone()).to_list().len(), 3);
        assert_eq!(a.clone().difference(b.clone()).to_list().len(), 2);
        assert_eq!(a.symmetric_difference(b).to_list().len(), 4);
    }

    #[test]
    fn relational_checks_work() {
        let a = HashSet::from_list([1, 2, 3]);
        let b = HashSet::from_list([1, 2, 3, 4, 5]);
        let c = HashSet::from_list([10, 20]);
        assert!(a.is_subset(&b));
        assert!(b.is_superset(&a));
        assert!(a.is_disjoint(&c));
        assert!(!a.is_disjoint(&b));
        assert!(a.equals(&HashSet::from_list([1, 2, 3])));
    }

    #[test]
    fn with_options_matches_hash_map_options() {
        let set = HashSet::with_options(4, "open_addressing", "murmur3").add(1).add(5);
        assert!(set.contains(&1));
        assert!(set.contains(&5));
    }

    #[test]
    fn from_list_with_options_and_free_function_wrappers_work() {
        let set = HashSet::from_list_with_options([1, 2, 2, 3], 2, "open", "djb2");
        assert_eq!(set.size(), 3);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));

        let set = from_list([10, 20]);
        assert!(contains(&set, &10));

        let set = add(set, 30);
        assert!(contains(&set, &30));

        let set = remove(set, &20);
        assert!(!contains(&set, &20));

        let set = discard(set, &99);
        assert!(contains(&set, &10));

        let other = from_list([30, 40]);
        let unioned = union(set.clone(), other.clone());
        let mut unioned_list = unioned.to_list();
        unioned_list.sort();
        assert_eq!(unioned_list, vec![10, 30, 40]);

        let intersected = intersection(set.clone(), other.clone());
        assert_eq!(intersected.to_list().len(), 1);

        let diffed = difference(set.clone(), other.clone());
        assert_eq!(diffed.to_list().len(), 1);

        let sym = symmetric_difference(set.clone(), other.clone());
        let mut sym_list = sym.to_list();
        sym_list.sort();
        assert_eq!(sym_list, vec![10, 40]);

        assert!(is_subset(&set, &unioned));
        assert!(is_superset(&unioned, &set));
        assert!(!is_subset(&unioned, &set));
        assert!(!is_superset(&set, &unioned));
        assert!(is_disjoint(&set, &from_list([999])));
        assert!(!is_disjoint(&set, &other));
        assert!(equals(&set, &set.clone()));
        assert!(!equals(&set, &other));
    }
}
