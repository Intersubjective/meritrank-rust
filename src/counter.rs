use std::collections::{HashMap, HashSet};

use crate::node::{NodeId, Weight};

/// A counter that keeps track of the counts for different nodes.
#[derive(Clone)]
pub struct Counter {
    counter: HashMap<NodeId, Weight>,
}

impl Counter {
    /// Creates a new empty counter.
    pub fn new() -> Self {
        Counter {
            counter: HashMap::new(),
        }
    }

    pub fn keys(&self) -> Vec<NodeId> {
        self.counter.keys().cloned().collect()
    }

    /// Updates the counter by incrementing the counts for the provided values.
    pub fn increment_counts<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = NodeId>,
    {
        for item in items {
            *self.counter.entry(item).or_insert(0.0) += 1.0;
        }
    }

    /// Updates the counter with unique values, incrementing their counts.
    pub fn increment_unique_counts<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = NodeId>,
    {
        let unique_values: HashSet<NodeId> = items.into_iter().collect();
        self.increment_counts(unique_values);
    }

    /// Returns the count value for the given node ID, if it exists.
    pub fn get_count(&self, key: &NodeId) -> Option<&Weight> {
        self.counter.get(key)
    }

    /// Returns a mutable reference to the count value for the given node ID, if it exists.
    pub fn get_mut_count(&mut self, key: &NodeId) -> &mut Weight {
        self.counter.entry(key.clone()).or_insert(0.0)
    }

    /// Increments the count for the specified node and returns a mutable reference to the count.
    pub fn increment_count(&mut self, node: NodeId, default: Weight) -> &mut Weight {
        self.counter.entry(node).or_insert(default)
    }

    /// Returns an iterator over the count values.
    pub fn count_values(&self) -> impl Iterator<Item = &Weight> {
        self.counter.values()
    }

    pub fn get_tree_map(&self) -> &HashMap<NodeId, Weight> {
        &self.counter
    }

    /// Returns the sum of all count values.
    pub fn total_count(&self) -> Weight {
        self.counter.values().sum()
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

use once_cell::sync::Lazy;

impl Default for &Counter {
    fn default() -> Self {
        static DEFAULT_COUNTER: Lazy<Counter> = Lazy::new(|| Counter {
            counter: HashMap::new(),
        });
        &DEFAULT_COUNTER
    }
}

/// Iterator over the entries of the `Counter`.
pub struct CounterIterator<'a> {
    inner: std::collections::hash_map::Iter<'a, NodeId, Weight>,
}

impl<'a> Iterator for CounterIterator<'a> {
    type Item = (&'a NodeId, &'a Weight);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> IntoIterator for &'a Counter {
    type Item = (&'a NodeId, &'a Weight);
    type IntoIter = CounterIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CounterIterator {
            inner: self.counter.iter(),
        }
    }
}
