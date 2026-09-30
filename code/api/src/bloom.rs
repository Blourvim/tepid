use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Bloom {

    //TODO: This is bad, it stores a whole  byte, but I will add space optimizations later
    contents: Vec<bool>,
}

impl Bloom {
    const HASH_FUNC_QUANT: usize = 4;


    /// create a filter with `size` bits.
    /// `size` here is the number of bits, higher means a better rate of accuracy
    /// https://en.wikipedia.org/wiki/Bloom_filter#Probability_of_false_positives
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "Bloom filter size must be non-zero");
        Bloom {
            contents: vec![false; size],
        }
    }

    fn calculate_index<T: Hash>(&self, item: &T) -> Vec<usize> {
        (0..Self::HASH_FUNC_QUANT as u64)
            .map(|seed| {
                let mut hasher = DefaultHasher::new();
                hasher.write_u64(seed); 
                item.hash(&mut hasher);
                (hasher.finish() as usize) % self.contents.len()
            })
            .collect()
    }

    /// insert into the store.
    pub fn push<T: Hash>(&mut self, item: &T) {
        for i in self.calculate_index(item) {
            self.contents[i] = true;
        }
    }

    ///you get "possibly in set" or "definitely not in set".
    pub fn find<T: Hash>(&self, item: &T) -> bool {
        self.calculate_index(item).into_iter().all(|i| self.contents[i])
    }
    /// NOT IMPLEMENTED
    /// I should be able to implement this without recreating the whole bloom storage with https://en.wikipedia.org/wiki/Bloom_filter#Counting_Bloom_filters
    /// But at this moment I am not sure If I should
    pub fn remove<T: Hash>(&mut self, item: &T) {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- construction ----------

    #[test]
    fn new_creates_all_false_filter() {
        let bloom = Bloom::new(123);
        assert_eq!(bloom.contents.len(), 123);
        assert!(bloom.contents.iter().all(|&b| b == false), "fresh filter must be all zeros");
    }

    #[test]
    #[should_panic(expected = "Bloom filter size must be non-zero")]
    fn new_with_zero_size_panics() {
        let _ = Bloom::new(0);
    }

    // ---------- basic behaviour ----------

    #[test]
    fn push_then_find_returns_true() {
        let mut bloom = Bloom::new(1024);
        bloom.push(&"hello");
        assert!(bloom.find(&"hello"));
    }

    #[test]
    fn find_on_empty_filter_always_returns_false() {
        let bloom = Bloom::new(1024);
        assert!(!bloom.find(&"hello"));
        assert!(!bloom.find(&42u64));
        assert!(!bloom.find(&(1, 'a')));
    }

    #[test]
    fn single_insert_sets_at_most_k_bits() {
        let mut bloom = Bloom::new(100_000);
        bloom.push(&"lonely");
        let set = bloom.contents.iter().filter(|&&b| b ==true).count();
        assert!(set > 0);
        assert!(set <= Bloom::HASH_FUNC_QUANT);
    }

    #[test]
    fn set_bits_never_exceed_k_per_insert() {
        let mut bloom = Bloom::new(100_000);
        let n = 1_000;
        for i in 0..n {
            bloom.push(&i);
        }
        let set = bloom.contents.iter().filter(|&&b| b).count();
        assert!(set <= n * Bloom::HASH_FUNC_QUANT);
    }

    // ---------- the core Bloom-filter guarantees ----------

    #[test]
    fn no_false_negatives_even_when_saturated() {
        // A bloom filter may lie with "yes", but must never say "no"
        // to an inserted item — even with a comically undersized filter.
        let mut bloom = Bloom::new(10);
        let items: Vec<u32> = (0..50).collect();
        for i in &items {
            bloom.push(i);
        }
        for i in &items {
            assert!(bloom.find(i), "false negative for {i} — must never happen");
        }
    }

    #[test]
    fn all_inserted_items_are_found() {
        let mut bloom = Bloom::new(10_000);
        let items: Vec<String> = (0..200).map(|i| format!("item-{i}")).collect();
        for s in &items {
            bloom.push(s);
        }
        for s in &items {
            assert!(bloom.find(s));
        }
    }


    #[test]
    fn calculate_index_stays_in_bounds() {
        let bloom = Bloom::new(97); // prime size, exercises the modulo
        let idx = bloom.calculate_index(&"some item");
        assert_eq!(idx.len(), Bloom::HASH_FUNC_QUANT);
        assert!(idx.iter().all(|&i| i < 97));
    }

    #[test]
    fn different_items_get_different_index_sets() {
        let bloom = Bloom::new(100_000);
        assert_ne!(
            bloom.calculate_index(&"item a"),
            bloom.calculate_index(&"item b")
        );
    }

    // ---------- genericity ----------

    #[test]
    fn works_with_any_hashable_type() {
        let mut bloom = Bloom::new(10_000);
        bloom.push(&'x');
        bloom.push(&(1u8, "tuple"));
        bloom.push(&vec![1u32, 2, 3]);

        assert!(bloom.find(&'x'));
        assert!(bloom.find(&(1u8, "tuple")));
        assert!(bloom.find(&vec![1u32, 2, 3]));
    }

    #[test]
    fn equivalent_values_of_different_types_hash_identically() {
        let mut bloom = Bloom::new(1_000);
        bloom.push(&"hello"); // T = &str
        assert!(bloom.find(&String::from("hello"))); // T = String
    }

}
