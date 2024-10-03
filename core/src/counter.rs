use integer_hasher::IntMap;
use crate::graph::NodeId;
use tinyset::SetUsize;
use once_cell::sync::Lazy;

type CounterValue = i32;

/// A counter that keeps track of the counts for different nodes.
#[derive(Clone)]
pub struct Counter {
    counter: IntMap<NodeId, CounterValue>,
}

impl Counter {
    /// Creates a new empty counter.
    pub fn new() -> Self {
        Counter {
            counter: IntMap::default(),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &NodeId> {
        self.counter.keys()
    }

    /// Updates the counter by incrementing the counts for the provided values.
    pub fn increment_counts<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = NodeId>,
    {
        items.into_iter().for_each(|item| {
            *self.counter.entry(item).or_insert(0) += 1;
        });
    }

    pub fn decrement_counts<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = NodeId>,
    {
        items.into_iter().for_each(|item| {
            if let Some(count) = self.counter.get_mut(&item) {
                if *count > 0 {
                    *count -= 1;
                }
            }
        });
    }

    /// Updates the counter with unique values, incrementing their counts.
    pub fn increment_unique_counts<'a, I>(&mut self, items: I)
    where
        I: IntoIterator<Item = &'a NodeId>,
    {
        let unique_values: SetUsize = SetUsize::from_iter(items.into_iter().copied());
        self.increment_counts(unique_values);
    }

    pub fn decrement_unique_counts<'a, I>(&mut self, items: I)
    where
        I: IntoIterator<Item = &'a NodeId>,
    {
        let unique_values: SetUsize = SetUsize::from_iter(items.into_iter().copied());
        self.decrement_counts(unique_values);
    }

    /// Returns the count value for the given node ID, if it exists.
    pub fn get_count(&self, key: &NodeId) -> CounterValue {
        *self.counter.get(key).unwrap_or(&0)
    }

    /// Returns the sum of all count values.
    pub fn total_count(&self) -> CounterValue {
        self.counter.values().sum()
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for &Counter {
    fn default() -> Self {
        static DEFAULT_COUNTER: Lazy<Counter> = Lazy::new(|| Counter {
            counter: IntMap::default(),
        });
        &DEFAULT_COUNTER
    }
}

/// Iterator over the entries of the `Counter`.
pub struct CounterIterator<'a> {
  inner: std::collections::hash_map::Iter<'a, NodeId, CounterValue>,
}

impl<'a> Iterator for CounterIterator<'a> {
    type Item = (&'a NodeId, &'a CounterValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> IntoIterator for &'a Counter {
    type Item = (&'a NodeId, &'a CounterValue);
    type IntoIter = CounterIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CounterIterator {
            inner: self.counter.iter(),
        }
    }
}
