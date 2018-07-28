//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies for generating `std::collections` of values.

use core::cmp::Ord;
use core::hash::Hash;
use core::ops::{Add, Range, RangeTo, RangeInclusive, RangeToInclusive};
use core::usize;

use std_facade::{fmt, Vec, VecDeque, BinaryHeap, BTreeMap, BTreeSet, LinkedList};

#[cfg(feature = "std")]
use std_facade::{HashMap, HashSet};

use bits::VarBitSet;
use num::sample_uniform_incl;
use strategy::*;
use tuple::TupleValueTree;
use test_runner::*;

//==============================================================================
// SizeRange
//==============================================================================

/// Creates a `SizeRange` from some value that is convertible into it.
pub fn size_range(from: impl Into<SizeRange>) -> SizeRange {
    from.into()
}

impl Default for SizeRange {
    /// Constructs a `SizeRange` equivalent to `size_range(0..100)`.
    fn default() -> Self {
        size_range(0..100)
    }
}

impl SizeRange {
    /// Creates a `SizeBounds` from a `RangeInclusive<usize>`.
    pub fn new(range: RangeInclusive<usize>) -> Self {
        SizeRange(range)
    }

    // Don't rely on these existing internally:

    /// Merges self together with some other argument producing a product
    /// type expected by some impelementations of `A: Arbitrary` in
    /// `A::Parameters`. This can be more ergonomic to work with and may
    /// help type inference.
    pub fn with<X>(self, and: X) -> product_type![Self, X] {
        product_pack![self, and]
    }

    /// Merges self together with some other argument generated with a
    /// default value producing a product type expected by some
    /// impelementations of `A: Arbitrary` in `A::Parameters`.
    /// This can be more ergonomic to work with and may help type inference.
    pub fn lift<X: Default>(self) -> product_type![Self, X] {
        self.with(Default::default())
    }

    /// Extract the ends `[low, high]` of a `SizeRange`.
    pub(crate) fn extract(&self) -> (usize, usize) {
        (self.start(), self.end())
    }

    pub(crate) fn start(&self) -> usize {
        *self.0.start()
    }

    pub(crate) fn end(&self) -> usize {
        *self.0.end()
    }

    pub(crate) fn end_excl(&self) -> usize {
        let end = self.end();
        // Quietly clamp to usize::MAX to allow RangeFrom to still be used
        // ergonomically in APIs that can't actually handle usize::MAX itself.
        if usize::MAX == end { end } else { end + 1}
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = usize> {
        self.0.clone().into_iter()
    }
}

/// Given `(low: usize, high: usize)`,
/// then a size range of `[low..high)` is the result.
impl From<(usize, usize)> for SizeRange {
    fn from((low, high): (usize, usize)) -> Self { size_range(low..high) }
}

/// Given `exact`, then a size range of `[exact, exact]` is the result.
impl From<usize> for SizeRange {
    fn from(exact: usize) -> Self { size_range(exact..=exact) }
}

/// Given `..high`, then a size range `[0, high)` is the result.
impl From<RangeTo<usize>> for SizeRange {
    fn from(high: RangeTo<usize>) -> Self { size_range(0..high.end) }
}

/// Given `low .. high`, then a size range `[low, high)` is the result.
impl From<Range<usize>> for SizeRange {
    fn from(r: Range<usize>) -> Self { size_range(r.start..=r.end - 1) }
}

/// Given `low ..= high`, then a size range `[low, high]` is the result.
impl From<RangeInclusive<usize>> for SizeRange {
    fn from(r: RangeInclusive<usize>) -> Self { Self::new(r) }
}

/// Given `..=high`, then a size range `[0, high]` is the result.
impl From<RangeToInclusive<usize>> for SizeRange {
    fn from(high: RangeToInclusive<usize>) -> Self { size_range(0..=high.end) }
}

/// Given a size range `[low, high]`, then a range`low..(high + 1)` is returned.
/// This will panic if `high == usize::MAX`.
impl From<SizeRange> for Range<usize> {
    fn from(sr: SizeRange) -> Self {
        let (start, end) = sr.extract();
        start..end + 1
    }
}

/// Given a size range `[low, high]`, then a range `low..=high` is returned.
impl From<SizeRange> for RangeInclusive<usize> {
    fn from(sr: SizeRange) -> Self { sr.0 }
}

#[cfg(feature = "frunk")]
impl Generic for SizeRange {
    type Repr = RangeInclusive<usize>;

    /// Converts the `SizeRange` into `Range<usize>`.
    fn into(self) -> Self::Repr { self.0 }

    /// Converts `RangeInclusive<usize>` into `SizeRange`.
    fn from(r: Self::Repr) -> Self { r.into() }
}

/// Adds `usize` to both start and end of the bounds.
///
/// Panics if adding to either end overflows `usize`.
impl Add<usize> for SizeRange {
    type Output = SizeRange;

    fn add(self, rhs: usize) -> Self::Output {
        let (start, end) = self.extract();
        size_range((start + rhs)..=(end + rhs))
    }
}

/// The minimum and maximum range/bounds on the size of a collection.
/// The interval must form a subset of `[0, std::usize::MAX]`.
///
/// The `Default` is `0..100`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SizeRange(RangeInclusive<usize>);

//==============================================================================
// Strategies
//==============================================================================

/// Strategy to create `Vec`s with a length in a certain range.
///
/// Created by the `vec()` function in the same module.
#[must_use = "strategies do nothing unless used"]
#[derive(Clone, Debug)]
pub struct VecStrategy<T : Strategy> {
    element: T,
    size: SizeRange,
}

/// Create a strategy to generate `Vec`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn vec<T: Strategy>(element: T, size: impl Into<SizeRange>)
                        -> VecStrategy<T> {
    VecStrategy { element, size: size.into() }
}

mapfn! {
    [] fn VecToDeque[<T : fmt::Debug>](vec: Vec<T>) -> VecDeque<T> {
        vec.into()
    }
}

opaque_strategy_wrapper! {
    /// Strategy to create `VecDeque`s with a length in a certain range.
    ///
    /// Created by the `vec_deque()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct VecDequeStrategy[<T>][where T : Strategy](
        statics::Map<VecStrategy<T>, VecToDeque>)
        -> VecDequeValueTree<T::Tree>;
    /// `ValueTree` corresponding to `VecDequeStrategy`.
    #[derive(Clone, Debug)]
    pub struct VecDequeValueTree[<T>][where T : ValueTree](
        statics::Map<VecValueTree<T>, VecToDeque>)
        -> VecDeque<T::Value>;
}

/// Create a strategy to generate `VecDeque`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn vec_deque<T: Strategy>(element: T, size: impl Into<SizeRange>)
    -> VecDequeStrategy<T>
{
    VecDequeStrategy(statics::Map::new(vec(element, size), VecToDeque))
}

mapfn! {
    [] fn VecToLl[<T : fmt::Debug>](vec: Vec<T>) -> LinkedList<T> {
        vec.into_iter().collect()
    }
}

opaque_strategy_wrapper! {
    /// Strategy to create `LinkedList`s with a length in a certain range.
    ///
    /// Created by the `linkedlist()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct LinkedListStrategy[<T>][where T : Strategy](
        statics::Map<VecStrategy<T>, VecToLl>)
        -> LinkedListValueTree<T::Tree>;
    /// `ValueTree` corresponding to `LinkedListStrategy`.
    #[derive(Clone, Debug)]
    pub struct LinkedListValueTree[<T>][where T : ValueTree](
        statics::Map<VecValueTree<T>, VecToLl>)
        -> LinkedList<T::Value>;
}

/// Create a strategy to generate `LinkedList`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn linked_list<T : Strategy>(element: T, size: impl Into<SizeRange>)
     -> LinkedListStrategy<T>
{
    LinkedListStrategy(statics::Map::new(vec(element, size), VecToLl))
}

mapfn! {
    [] fn VecToBinHeap[<T : fmt::Debug + Ord>](vec: Vec<T>) -> BinaryHeap<T> {
        vec.into()
    }
}

opaque_strategy_wrapper! {
    /// Strategy to create `BinaryHeap`s with a length in a certain range.
    ///
    /// Created by the `binary_heap()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct BinaryHeapStrategy[<T>][where T : Strategy, T::Value : Ord](
        statics::Map<VecStrategy<T>, VecToBinHeap>)
        -> BinaryHeapValueTree<T::Tree>;
    /// `ValueTree` corresponding to `BinaryHeapStrategy`.
    #[derive(Clone, Debug)]
    pub struct BinaryHeapValueTree[<T>][where T : ValueTree, T::Value : Ord](
        statics::Map<VecValueTree<T>, VecToBinHeap>)
        -> BinaryHeap<T::Value>;
}

/// Create a strategy to generate `BinaryHeap`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn binary_heap<T : Strategy>(element: T, size: impl Into<SizeRange>)
    -> BinaryHeapStrategy<T>
where T::Value : Ord {
    BinaryHeapStrategy(statics::Map::new(vec(element, size), VecToBinHeap))
}

mapfn! {
    {#[cfg(feature = "std")]}
    [] fn VecToHashSet[<T : fmt::Debug + Hash + Eq>](vec: Vec<T>)
                                                     -> HashSet<T> {
        vec.into_iter().collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct MinSize(usize);

#[cfg(feature = "std")]
impl<T : Eq + Hash> statics::FilterFn<HashSet<T>> for MinSize {
    fn apply(&self, set: &HashSet<T>) -> bool {
        set.len() >= self.0
    }
}

opaque_strategy_wrapper! {
    {#[cfg(feature = "std")]}
    /// Strategy to create `HashSet`s with a length in a certain range.
    ///
    /// Created by the `hash_set()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct HashSetStrategy[<T>][where T : Strategy, T::Value : Hash + Eq](
        statics::Filter<statics::Map<VecStrategy<T>, VecToHashSet>, MinSize>)
        -> HashSetValueTree<T::Tree>;
    /// `ValueTree` corresponding to `HashSetStrategy`.
    #[derive(Clone, Debug)]
    pub struct HashSetValueTree[<T>][where T : ValueTree, T::Value : Hash + Eq](
        statics::Filter<statics::Map<VecValueTree<T>, VecToHashSet>, MinSize>)
        -> HashSet<T::Value>;
}

/// Create a strategy to generate `HashSet`s containing elements drawn from
/// `element` and with a size range given by `size`.
///
/// This strategy will implicitly do local rejects to ensure that the `HashSet`
/// has at least the minimum number of elements, in case `element` should
/// produce duplicate values.
#[cfg(feature = "std")]
pub fn hash_set<T : Strategy>(element: T, size: impl Into<SizeRange>)
                              -> HashSetStrategy<T>
where T::Value : Hash + Eq {
    let size = size.into();
    HashSetStrategy(statics::Filter::new(
        statics::Map::new(vec(element, size.clone()), VecToHashSet),
        "HashSet minimum size".into(),
        MinSize(size.start())))
}

mapfn! {
    [] fn VecToBTreeSet[<T : fmt::Debug + Ord>](vec: Vec<T>)
                                                -> BTreeSet<T> {
        vec.into_iter().collect()
    }
}

impl<T : Ord> statics::FilterFn<BTreeSet<T>> for MinSize {
    fn apply(&self, set: &BTreeSet<T>) -> bool {
        set.len() >= self.0
    }
}

opaque_strategy_wrapper! {
    /// Strategy to create `BTreeSet`s with a length in a certain range.
    ///
    /// Created by the `btree_set()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct BTreeSetStrategy[<T>][where T : Strategy, T::Value : Ord](
        statics::Filter<statics::Map<VecStrategy<T>, VecToBTreeSet>, MinSize>)
        -> BTreeSetValueTree<T::Tree>;
    /// `ValueTree` corresponding to `BTreeSetStrategy`.
    #[derive(Clone, Debug)]
    pub struct BTreeSetValueTree[<T>][where T : ValueTree, T::Value : Ord](
        statics::Filter<statics::Map<VecValueTree<T>, VecToBTreeSet>, MinSize>)
        -> BTreeSet<T::Value>;
}

/// Create a strategy to generate `BTreeSet`s containing elements drawn from
/// `element` and with a size range given by `size`.
///
/// This strategy will implicitly do local rejects to ensure that the
/// `BTreeSet` has at least the minimum number of elements, in case `element`
/// should produce duplicate values.
pub fn btree_set<T : Strategy>(element: T, size: impl Into<SizeRange>)
                               -> BTreeSetStrategy<T>
where T::Value : Ord {
    let size = size.into();
    BTreeSetStrategy(statics::Filter::new(
        statics::Map::new(vec(element, size.clone()), VecToBTreeSet),
        "BTreeSet minimum size".into(),
        MinSize(size.start())))
}

mapfn! {
    {#[cfg(feature = "std")]}
    [] fn VecToHashMap[<K : fmt::Debug + Hash + Eq, V : fmt::Debug>]
        (vec: Vec<(K, V)>) -> HashMap<K, V>
    {
        vec.into_iter().collect()
    }
}

#[cfg(feature = "std")]
impl<K : Hash + Eq, V> statics::FilterFn<HashMap<K, V>> for MinSize {
    fn apply(&self, map: &HashMap<K, V>) -> bool {
        map.len() >= self.0
    }
}

opaque_strategy_wrapper! {
    {#[cfg(feature = "std")]}
    /// Strategy to create `HashMap`s with a length in a certain range.
    ///
    /// Created by the `hash_map()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct HashMapStrategy[<K, V>]
        [where K : Strategy, V : Strategy, K::Value : Hash + Eq](
            statics::Filter<statics::Map<VecStrategy<(K,V)>,
            VecToHashMap>, MinSize>)
        -> HashMapValueTree<K::Tree, V::Tree>;
    /// `ValueTree` corresponding to `HashMapStrategy`.
    #[derive(Clone, Debug)]
    pub struct HashMapValueTree[<K, V>]
        [where K : ValueTree, V : ValueTree, K::Value : Hash + Eq](
            statics::Filter<statics::Map<VecValueTree<TupleValueTree<(K, V)>>,
            VecToHashMap>, MinSize>)
        -> HashMap<K::Value, V::Value>;
}

/// Create a strategy to generate `HashMap`s containing keys and values drawn
/// from `key` and `value` respectively, and with a size within the given
/// range.
///
/// This strategy will implicitly do local rejects to ensure that the `HashMap`
/// has at least the minimum number of elements, in case `key` should produce
/// duplicate values.
#[cfg(feature = "std")]
pub fn hash_map<K : Strategy, V : Strategy>
    (key: K, value: V, size: impl Into<SizeRange>) -> HashMapStrategy<K, V>
where K::Value : Hash + Eq {
    let size = size.into();
    HashMapStrategy(statics::Filter::new(
        statics::Map::new(vec((key, value), size.clone()), VecToHashMap),
        "HashMap minimum size".into(),
        MinSize(size.start())))
}

mapfn! {
    [] fn VecToBTreeMap[<K : fmt::Debug + Ord, V : fmt::Debug>]
        (vec: Vec<(K, V)>) -> BTreeMap<K, V>
    {
        vec.into_iter().collect()
    }
}

impl<K : Ord, V> statics::FilterFn<BTreeMap<K, V>> for MinSize {
    fn apply(&self, map: &BTreeMap<K, V>) -> bool {
        map.len() >= self.0
    }
}

opaque_strategy_wrapper! {
    /// Strategy to create `BTreeMap`s with a length in a certain range.
    ///
    /// Created by the `btree_map()` function in the same module.
    #[derive(Clone, Debug)]
    pub struct BTreeMapStrategy[<K, V>]
        [where K : Strategy, V : Strategy, K::Value : Ord](
            statics::Filter<statics::Map<VecStrategy<(K,V)>,
            VecToBTreeMap>, MinSize>)
        -> BTreeMapValueTree<K::Tree, V::Tree>;
    /// `ValueTree` corresponding to `BTreeMapStrategy`.
    #[derive(Clone, Debug)]
    pub struct BTreeMapValueTree[<K, V>]
        [where K : ValueTree, V : ValueTree, K::Value : Ord](
            statics::Filter<statics::Map<VecValueTree<TupleValueTree<(K, V)>>,
            VecToBTreeMap>, MinSize>)
        -> BTreeMap<K::Value, V::Value>;
}

/// Create a strategy to generate `BTreeMap`s containing keys and values drawn
/// from `key` and `value` respectively, and with a size within the given
/// range.
///
/// This strategy will implicitly do local rejects to ensure that the
/// `BTreeMap` has at least the minimum number of elements, in case `key`
/// should produce duplicate values.
pub fn btree_map<K : Strategy, V : Strategy>
    (key: K, value: V, size: impl Into<SizeRange>) -> BTreeMapStrategy<K, V>
where K::Value : Ord {
    let size = size.into();
    BTreeMapStrategy(statics::Filter::new(
        statics::Map::new(vec((key, value), size.clone()), VecToBTreeMap),
        "BTreeMap minimum size".into(),
        MinSize(size.start())))
}

#[derive(Clone, Copy, Debug)]
enum Shrink {
    DeleteElement(usize),
    ShrinkElement(usize),
}

/// `ValueTree` corresponding to `VecStrategy`.
#[derive(Clone, Debug)]
pub struct VecValueTree<T : ValueTree> {
    elements: Vec<T>,
    included_elements: VarBitSet,
    min_size: usize,
    shrink: Shrink,
    prev_shrink: Option<Shrink>,
}

impl<T : Strategy> Strategy for VecStrategy<T> {
    type Tree = VecValueTree<T::Tree>;
    type Value = Vec<T::Value>;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        let (start, end) = self.size.extract();
        let max_size = sample_uniform_incl(runner, start, end);
        let mut elements = Vec::with_capacity(max_size);
        while elements.len() < max_size {
            elements.push(self.element.new_tree(runner)?);
        }

        Ok(VecValueTree {
            elements,
            included_elements: (0..max_size).collect(),
            min_size: start,
            shrink: Shrink::DeleteElement(0),
            prev_shrink: None,
        })
    }
}

impl<T : ValueTree> ValueTree for VecValueTree<T> {
    type Value = Vec<T::Value>;

    fn current(&self) -> Vec<T::Value> {
        self.elements.iter().enumerate()
            .filter(|&(ix, _)| self.included_elements.contains(ix))
            .map(|(_, element)| element.current())
            .collect()
    }

    fn simplify(&mut self) -> bool {
        // The overall strategy here is to iteratively delete elements from the
        // list until we can do so no further, then to shrink each remaining
        // element in sequence.
        //
        // For `complicate()`, we simply undo the last shrink operation, if
        // there was any.
        if let Shrink::DeleteElement(ix) = self.shrink {
            // Can't delete an element if beyond the end of the vec or if it
            // would put us under the minimum length.
            if ix >= self.elements.len() ||
                self.included_elements.len() == self.min_size
            {
                self.shrink = Shrink::ShrinkElement(0);
            } else {
                self.included_elements.remove(ix);
                self.prev_shrink = Some(self.shrink);
                self.shrink = Shrink::DeleteElement(ix + 1);
                return true;
            }
        }

        while let Shrink::ShrinkElement(ix) = self.shrink {
            if ix >= self.elements.len() {
                // Nothing more we can do
                return false;
            }

            if !self.included_elements.contains(ix) {
                // No use shrinking something we're not including.
                self.shrink = Shrink::ShrinkElement(ix + 1);
                continue;
            }

            if !self.elements[ix].simplify() {
                // Move on to the next element
                self.shrink = Shrink::ShrinkElement(ix + 1);
            } else {
                self.prev_shrink = Some(self.shrink);
                return true;
            }
        }

        panic!("Unexpected shrink state");
    }

    fn complicate(&mut self) -> bool {
        match self.prev_shrink {
            None => false,
            Some(Shrink::DeleteElement(ix)) => {
                // Undo the last item we deleted. Can't complicate any further,
                // so unset prev_shrink.
                self.included_elements.insert(ix);
                self.prev_shrink = None;
                true
            },
            Some(Shrink::ShrinkElement(ix)) => {
                if self.elements[ix].complicate() {
                    // Don't unset prev_shrink; we may be able to complicate
                    // again.
                    true
                } else {
                    // Can't complicate the last element any further.
                    self.prev_shrink = None;
                    false
                }
            }
        }
    }
}

//==============================================================================
// Tests
//==============================================================================

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_vec() {
        let input = vec(1usize..20usize, 5..20);
        let mut num_successes = 0;

        for _ in 0..256 {
            let mut runner = TestRunner::default();
            let case = input.new_tree(&mut runner).unwrap();
            let start = case.current();
            // Has correct length
            assert!(start.len() >= 5 && start.len() < 20);
            // Has at least 2 distinct values
            assert!(start.iter().map(|&v| v).collect::<VarBitSet>().len() >= 2);

            let result = runner.run_one(case, |v| {
                prop_assert!(v.iter().map(|&v| v).sum::<usize>() < 9,
                             "greater than 8");
                Ok(())
            });

            match result {
                Ok(true) => num_successes += 1,
                Err(TestError::Fail(_, value)) => {
                    // The minimal case always has between 5 (due to min
                    // length) and 9 (min element value = 1) elements, and
                    // always sums to exactly 9.
                    assert!(value.len() >= 5 && value.len() <= 9 &&
                            value.iter().map(|&v| v).sum::<usize>() == 9,
                            "Unexpected minimal value: {:?}", value);
                },
                e => panic!("Unexpected result: {:?}", e),
            }
        }

        assert!(num_successes < 256);
    }

    #[test]
    fn test_vec_sanity() {
        check_strategy_sanity(vec(0i32..1000, 5..10), None);
    }

    #[cfg(feature = "std")]
    #[test]
    #[cfg(feature = "std")]
    fn test_map() {
        // Only 8 possible keys
        let input = hash_map("[ab]{3}", "a", 2..3);
        let mut runner = TestRunner::default();

        for _ in 0..256 {
            let v = input.new_tree(&mut runner).unwrap().current();
            assert_eq!(2, v.len());
        }
    }

    #[cfg(feature = "std")]
    #[test]
    #[cfg(feature = "std")]
    fn test_set() {
        // Only 8 possible values
        let input = hash_set("[ab]{3}", 2..3);
        let mut runner = TestRunner::default();

        for _ in 0..256 {
            let v = input.new_tree(&mut runner).unwrap().current();
            assert_eq!(2, v.len());
        }
    }
}
