//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::std_facade::{fmt, Arc};

use crate::strategy::fuse::Fuse;
use crate::strategy::traits::*;
use crate::test_runner::*;

/// Adaptor that flattens a `Strategy` which produces other `Strategy`s into a
/// `Strategy` that picks one of those strategies and then picks values from
/// it.
#[derive(Debug, Clone, Copy)]
#[must_use = "strategies do nothing unless used"]
pub struct Flatten<S> {
    source: S,
}

impl<S: Strategy> Flatten<S> {
    /// Wrap `source` to flatten it.
    pub fn new(source: S) -> Self {
        Flatten { source }
    }
}

impl<S: Strategy> Strategy for Flatten<S>
where
    S::Value: Strategy,
{
    type Tree = FlattenValueTree<S::Tree>;
    type Value = <S::Value as Strategy>::Value;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        let meta = self.source.new_tree(runner)?;
        FlattenValueTree::new(runner, meta)
    }
}

/// The `ValueTree` produced by `Flatten`.
pub struct FlattenValueTree<S: ValueTree>
where
    S::Value: Strategy,
{
    meta: Fuse<S>,
    current: Fuse<<S::Value as Strategy>::Tree>,
    // The final value to produce after successive calls to complicate() on the
    // underlying objects return false.
    final_complication: Option<Fuse<<S::Value as Strategy>::Tree>>,
    // When `simplify()` or `complicate()` causes a new `Strategy` to be
    // chosen, we need to find a new failing input for that case. To do this,
    // we implement `complicate()` by regenerating values up to a number of
    // times corresponding to the maximum number of test cases. A `simplify()`
    // which does not cause a new strategy to be chosen always resets
    // `complicate_regen_remaining` to 0.
    //
    // This does unfortunately depart from the direct interpretation of
    // simplify/complicate as binary search, but is still easier to think about
    // than other implementations of higher-order strategies.
    runner: TestRunner,
    complicate_regen_remaining: u32,
}

impl<S: ValueTree> Clone for FlattenValueTree<S>
where
    S::Value: Strategy + Clone,
    S: Clone,
    <S::Value as Strategy>::Tree: Clone,
{
    fn clone(&self) -> Self {
        FlattenValueTree {
            meta: self.meta.clone(),
            current: self.current.clone(),
            final_complication: self.final_complication.clone(),
            runner: self.runner.clone(),
            complicate_regen_remaining: self.complicate_regen_remaining,
        }
    }
}

impl<S: ValueTree> fmt::Debug for FlattenValueTree<S>
where
    S::Value: Strategy,
    S: fmt::Debug,
    <S::Value as Strategy>::Tree: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FlattenValueTree")
            .field("meta", &self.meta)
            .field("current", &self.current)
            .field("final_complication", &self.final_complication)
            .field("complicate_regen_remaining", &self.complicate_regen_remaining)
            .finish()
    }
}

impl<S: ValueTree> FlattenValueTree<S>
where
    S::Value: Strategy,
{
    fn new(runner: &mut TestRunner, meta: S) -> Result<Self, Reason> {
        let current = meta.current().new_tree(runner)?;
        Ok(FlattenValueTree {
            meta: Fuse::new(meta),
            current: Fuse::new(current),
            final_complication: None,
            runner: runner.partial_clone(),
            complicate_regen_remaining: 0,
        })
    }
}

impl<S: ValueTree> ValueTree for FlattenValueTree<S>
where
    S::Value: Strategy,
{
    type Value = <S::Value as Strategy>::Value;

    fn current(&self) -> Self::Value {
        self.current.current()
    }

    fn simplify(&mut self) -> bool {
        false
    }

    fn complicate(&mut self) -> bool {
        false
    }
}

/// Similar to `Flatten`, but does not shrink the input strategy.
///
/// See `Strategy::prop_ind_flat_map()` fore more details.
#[derive(Clone, Copy, Debug)]
pub struct IndFlatten<S>(pub(super) S);

impl<S: Strategy> Strategy for IndFlatten<S>
where
    S::Value: Strategy,
{
    type Tree = <S::Value as Strategy>::Tree;
    type Value = <S::Value as Strategy>::Value;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        let inner = self.0.new_tree(runner)?;
        inner.current().new_tree(runner)
    }
}

/// Similar to `Map` plus `Flatten`, but does not shrink the input strategy and
/// passes the original input through.
///
/// See `Strategy::prop_ind_flat_map2()` for more details.
pub struct IndFlattenMap<S, F> {
    pub(super) source: S,
    pub(super) fun: Arc<F>,
}

impl<S: fmt::Debug, F> fmt::Debug for IndFlattenMap<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IndFlattenMap")
            .field("source", &self.source)
            .field("fun", &"<function>")
            .finish()
    }
}

impl<S: Clone, F> Clone for IndFlattenMap<S, F> {
    fn clone(&self) -> Self {
        IndFlattenMap { source: self.source.clone(), fun: Arc::clone(&self.fun) }
    }
}

impl<S: Strategy, R: Strategy, F: Fn(S::Value) -> R> Strategy for IndFlattenMap<S, F> {
    type Tree = crate::tuple::TupleValueTree<(S::Tree, R::Tree)>;
    type Value = (S::Value, R::Value);

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        let left = self.source.new_tree(runner)?;
        let right_source = (self.fun)(left.current());
        let right = right_source.new_tree(runner)?;

        Ok(crate::tuple::TupleValueTree::new((left, right)))
    }
}

// TODO: Support perturbations.
