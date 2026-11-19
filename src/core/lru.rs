//! Generic zero-dependency bounded LRU Cache using an index-based doubly-linked list.
//!
//! Provides O(1) `get`, `peek`, `insert` (with automatic LRU eviction), and `remove`.
//! Evicted items are dropped immediately upon eviction, releasing held resources (such as GPU textures or memory).

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone)]
struct Node<K, V> {
    key: K,
    value: V,
    prev: Option<usize>,
    next: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    capacity: usize,
    nodes: Vec<Option<Node<K, V>>>,
    map: HashMap<K, usize>,
    head: Option<usize>, // MRU
    tail: Option<usize>, // LRU
    free_slots: Vec<usize>,
}

impl<K: Hash + Eq + Clone, V> Default for LruCache<K, V> {
    fn default() -> Self {
        Self::new(256)
    }
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    /// Creates a new `LruCache` with the specified fixed capacity.
    ///
    /// # Panics
    /// Panics if `capacity == 0`.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "LruCache capacity must be greater than zero");
        Self {
            capacity,
            nodes: Vec::with_capacity(capacity.min(1024)),
            map: HashMap::with_capacity(capacity.min(1024)),
            head: None,
            tail: None,
            free_slots: Vec::new(),
        }
    }

    /// Number of items currently stored in the cache.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True if the cache contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Maximum capacity of the cache.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Retrieves an immutable reference to the value and promotes the entry to Most-Recently-Used (MRU).
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.map.get(key) {
            self.detach(idx);
            self.attach_head(idx);
            Some(&self.nodes[idx].as_ref().unwrap().value)
        } else {
            None
        }
    }

    /// Retrieves a mutable reference to the value and promotes the entry to Most-Recently-Used (MRU).
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.map.get(key) {
            self.detach(idx);
            self.attach_head(idx);
            Some(&mut self.nodes[idx].as_mut().unwrap().value)
        } else {
            None
        }
    }

    /// Inspects the value without altering its LRU recency.
    /// Allows concurrent read-only queries with `&self`.
    pub fn peek<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map
            .get(key)
            .and_then(|&idx| self.nodes[idx].as_ref().map(|n| &n.value))
    }

    /// Inspects the mutable value without altering its LRU recency.
    pub fn peek_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.map.get(key) {
            self.nodes[idx].as_mut().map(|n| &mut n.value)
        } else {
            None
        }
    }

    /// Checks whether the key is present in the cache.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }

    /// Inserts a key-value pair.
    ///
    /// If the key is already present, its value is updated, moved to the MRU position,
    /// and the old value is returned.
    ///
    /// If the cache is at capacity, the Least-Recently-Used (LRU) entry is evicted and
    /// immediately dropped before the new entry is inserted.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&idx) = self.map.get(&key) {
            self.detach(idx);
            self.attach_head(idx);
            let node = self.nodes[idx].as_mut().unwrap();
            let old = std::mem::replace(&mut node.value, value);
            return Some(old);
        }

        if self.map.len() >= self.capacity {
            if let Some(tail_idx) = self.tail {
                self.detach(tail_idx);
                let evicted = self.nodes[tail_idx].take().unwrap();
                self.map.remove(&evicted.key);
                drop(evicted); // Evicted value dropped immediately!

                self.nodes[tail_idx] = Some(Node {
                    key: key.clone(),
                    value,
                    prev: None,
                    next: None,
                });
                self.map.insert(key, tail_idx);
                self.attach_head(tail_idx);
                return None;
            }
        }

        let idx = if let Some(free_idx) = self.free_slots.pop() {
            self.nodes[free_idx] = Some(Node {
                key: key.clone(),
                value,
                prev: None,
                next: None,
            });
            free_idx
        } else {
            let new_idx = self.nodes.len();
            self.nodes.push(Some(Node {
                key: key.clone(),
                value,
                prev: None,
                next: None,
            }));
            new_idx
        };

        self.map.insert(key, idx);
        self.attach_head(idx);
        None
    }

    /// Removes a key from the cache, returning its value if present.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(idx) = self.map.remove(key) {
            self.detach(idx);
            let node = self.nodes[idx].take().unwrap();
            self.free_slots.push(idx);
            Some(node.value)
        } else {
            None
        }
    }

    /// Clears all entries from the cache.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.map.clear();
        self.head = None;
        self.tail = None;
        self.free_slots.clear();
    }

    /// Iterates through elements in order from Most-Recently-Used (MRU) to Least-Recently-Used (LRU).
    pub fn iter(&self) -> LruIter<'_, K, V> {
        LruIter {
            cache: self,
            cur: self.head,
        }
    }

    fn detach(&mut self, idx: usize) {
        let (prev, next) = {
            let n = self.nodes[idx].as_ref().unwrap();
            (n.prev, n.next)
        };

        if let Some(p) = prev {
            self.nodes[p].as_mut().unwrap().next = next;
        } else {
            self.head = next;
        }

        if let Some(n) = next {
            self.nodes[n].as_mut().unwrap().prev = prev;
        } else {
            self.tail = prev;
        }

        let n = self.nodes[idx].as_mut().unwrap();
        n.prev = None;
        n.next = None;
    }

    fn attach_head(&mut self, idx: usize) {
        let old_head = self.head;
        if let Some(h) = old_head {
            self.nodes[h].as_mut().unwrap().prev = Some(idx);
        }
        let n = self.nodes[idx].as_mut().unwrap();
        n.prev = None;
        n.next = old_head;
        self.head = Some(idx);
        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }
}

pub struct LruIter<'a, K, V> {
    cache: &'a LruCache<K, V>,
    cur: Option<usize>,
}

impl<'a, K, V> Iterator for LruIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.cur?;
        let node = self.cache.nodes[idx].as_ref()?;
        self.cur = node.next;
        Some((&node.key, &node.value))
    }
}

impl<'a, K: Hash + Eq + Clone, V> IntoIterator for &'a LruCache<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = LruIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct DropTracker(#[allow(dead_code)] usize, Arc<AtomicUsize>);
    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.1.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_basic_crud() {
        let mut cache = LruCache::new(3);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        assert_eq!(cache.insert("a", 1), None);
        assert_eq!(cache.insert("b", 2), None);
        assert_eq!(cache.insert("c", 3), None);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.peek(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));

        // Updating "a"
        assert_eq!(cache.insert("a", 10), Some(1));
        assert_eq!(cache.get(&"a"), Some(&10));

        // Remove
        assert_eq!(cache.remove(&"b"), Some(2));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"b"), None);
    }

    #[test]
    fn test_lru_eviction_order() {
        let mut cache = LruCache::new(3);
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");

        // Access 1 so MRU order becomes: 1, 3, 2 (2 is LRU)
        assert_eq!(cache.get(&1), Some(&"one"));

        // Insert 4: 2 should be evicted
        cache.insert(4, "four");
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&3), Some(&"three"));
        assert_eq!(cache.get(&4), Some(&"four"));

        // Order is now: 4, 3, 1 (peek doesn't change order)
        assert_eq!(cache.peek(&1), Some(&"one"));
        let items: Vec<(&i32, &&str)> = cache.iter().collect();
        assert_eq!(items, vec![(&4, &"four"), (&3, &"three"), (&1, &"one")]);
    }

    #[test]
    fn test_immediate_drop_on_eviction() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        let mut cache = LruCache::new(2);

        cache.insert(1, DropTracker(1, Arc::clone(&drop_count)));
        cache.insert(2, DropTracker(2, Arc::clone(&drop_count)));
        assert_eq!(drop_count.load(Ordering::SeqCst), 0);

        // Insert 3 -> 1 is evicted and must be dropped immediately!
        cache.insert(3, DropTracker(3, Arc::clone(&drop_count)));
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);

        // Remove 2 -> dropped
        cache.remove(&2);
        assert_eq!(drop_count.load(Ordering::SeqCst), 2);

        // Clear remaining
        cache.clear();
        assert_eq!(drop_count.load(Ordering::SeqCst), 3);
    }
}
