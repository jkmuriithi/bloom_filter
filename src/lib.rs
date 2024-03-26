use std::{
    hash::{BuildHasher, Hash, RandomState},
    marker::PhantomData,
};

/// Fixed-size set of booleans represented by a vector of bytes.
#[derive(Debug, Clone)]
struct BitSet {
    bits: Vec<u8>,
    // len: usize,
}

impl BitSet {
    fn new(len: usize) -> Self {
        BitSet {
            bits: vec![0u8; (len.saturating_sub(1) / 8) + 1],
            // len,
        }
    }

    // pub fn len(&self) -> usize {
    //     self.len
    // }

    // #[must_use]
    // pub fn is_empty(&self) -> bool {
    //     self.len == 0
    // }

    /// Bit accesses are currently unchecked, because this struct is private.
    fn get(&self, bit: usize) -> bool {
        // assert!(
        //     (0..self.len).contains(&bit),
        //     "out-of-bounds bit index {} for BitSet of length {}",
        //     bit,
        //     self.len
        // );

        let byte_idx = bit / 8;
        let bit_idx = bit % 8;

        (self.bits[byte_idx] >> bit_idx) & 1 == 1
    }

    /// Bit accesses are currently unchecked, because this struct is private.
    fn set(&mut self, bit: usize, val: bool) {
        // assert!(
        //     (0..self.len).contains(&bit),
        //     "out-of-bounds bit index {} for BitSet of length {}",
        //     bit,
        //     self.len
        // );

        let byte_idx = bit / 8;
        let bit_idx = bit % 8;

        if (self.bits[byte_idx] >> bit_idx) & 1 != val as u8 {
            self.bits[byte_idx] ^= 1 << bit_idx
        }
    }
}

/// A bloom filter using an array of size `F` and `H` distinct hash functions.
/// The constant parameters of this struct should be tuned for the filter's
/// specific use case; under the defaults, the false positive rate should be <<
/// 1% after 10K items are inserted.
#[derive(Debug, Clone)]
pub struct BloomFilter<T: Hash, const F: usize = 150000, const H: usize = 5> {
    hashers: Vec<RandomState>,
    filter: BitSet,
    _element_type: PhantomData<T>,
}

impl<T: Hash, const F: usize, const H: usize> BloomFilter<T, F, H> {
    pub fn new() -> Self {
        BloomFilter {
            hashers: (0..H).map(|_| RandomState::new()).collect(),
            filter: BitSet::new(F),
            _element_type: PhantomData,
        }
    }

    pub fn insert(&mut self, item: &T) {
        self.hashers
            .iter()
            .map(|rs| rs.hash_one(item))
            .for_each(|hash| self.filter.set(hash as usize % F, true))
    }

    pub fn contains(&self, item: &T) -> bool {
        self.hashers
            .iter()
            .map(|rs| rs.hash_one(item))
            .all(|hash| self.filter.get(hash as usize % F))
    }
}

impl<T: Hash, const F: usize, const H: usize> Default for BloomFilter<T, F, H> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BITSET_SIZE: usize = 1000;
    const BLOOM_FILTER_MAX: usize = 10000;

    // Should be loose to prevent non-deterministic failures
    const FALSE_POS_UPPER_BOUND: f64 = 0.01;

    #[test]
    #[should_panic]
    fn bitset_bounds_get() {
        let bs = BitSet::new(BITSET_SIZE);
        bs.get(BITSET_SIZE);
    }

    #[test]
    #[should_panic]
    fn bitset_bounds_set() {
        let mut bs = BitSet::new(BITSET_SIZE);
        bs.set(BITSET_SIZE, true);
    }

    #[test]
    fn bitset_correctness() {
        let mut bs = BitSet::new(BITSET_SIZE);

        for i in 0..BITSET_SIZE {
            assert!(!bs.get(i), "bitset index {} is not initially false", i);

            bs.set(i, true);
            assert!(bs.get(i), "bitset index {} is not true after set true", i);

            bs.set(i, false);
            assert!(
                !bs.get(i),
                "bitset index {} is not false after set false",
                i
            );
        }

        assert!(
            (0..BITSET_SIZE).all(|i| !bs.get(i)),
            "bitset contains true value after all set false"
        )
    }

    #[test]
    fn bloom_filter_accepts_one() {
        let num = 1234;

        let mut bf: BloomFilter<i32> = BloomFilter::new();
        bf.insert(&num);

        assert!(
            bf.contains(&num),
            "bloom filter doesn't contain inserted item"
        )
    }

    #[test]
    fn bloom_filter_rejects_one() {
        let num = 1234;

        let bf: BloomFilter<i32> = BloomFilter::new();

        assert!(
            !bf.contains(&num),
            "empty bloom filter claims to contain item"
        )
    }

    #[test]
    fn bloom_filter_accepts_many() {
        let mut bf: BloomFilter<usize> = BloomFilter::new();

        (0..BLOOM_FILTER_MAX).for_each(|i| bf.insert(&i));

        assert!(
            (0..BLOOM_FILTER_MAX).all(|i| bf.contains(&i)),
            "bloom filter doesn't contain inserted items"
        )
    }

    #[test]
    fn bloom_filter_rejects_many() {
        let bf: BloomFilter<usize> = BloomFilter::new();

        assert!(
            (0..BLOOM_FILTER_MAX).all(|i| !bf.contains(&i)),
            "empty bloom filter claims to contain items"
        )
    }

    #[test]
    fn bloom_filter_false_positives() {
        let mut false_pos = 0;

        let mut bf: BloomFilter<usize> = BloomFilter::new();
        for i in 0..BLOOM_FILTER_MAX {
            bf.insert(&i);
        }

        for i in BLOOM_FILTER_MAX..BLOOM_FILTER_MAX * 2 {
            false_pos += bf.contains(&i) as usize;
        }

        let rate = false_pos as f64 / BLOOM_FILTER_MAX as f64;
        assert!(
            rate < FALSE_POS_UPPER_BOUND,
            "false positive rate {} higher than set bound {}",
            rate,
            FALSE_POS_UPPER_BOUND
        )
    }
}
