//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! Support for combining strategies into tuples.
//!
//! There is no explicit "tuple strategy"; simply make a tuple containing the
//! strategy and that tuple is itself a strategy.

use crate::strategy::*;
use crate::test_runner::*;

/// Common `ValueTree` implementation for all tuple strategies.
#[derive(Clone, Copy, Debug)]
pub struct TupleValueTree<T> {
    tree: T,
}

impl<T> TupleValueTree<T> {
    /// Create a new `TupleValueTree` wrapping `inner`.
    ///
    /// It only makes sense for `inner` to be a tuple of an arity for which the
    /// type implements `ValueTree`.
    pub fn new(inner: T) -> Self {
        TupleValueTree { tree: inner }
    }
}

macro_rules! tuple {
    ($($fld:tt : $typ:ident),*) => {
        impl<$($typ : Strategy),*> Strategy for ($($typ,)*) {
            type Tree = TupleValueTree<($($typ::Tree,)*)>;
            type Value = ($($typ::Value,)*);

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                let values = ($(self.$fld.new_tree(runner)?,)*);
                Ok(TupleValueTree::new(values))
            }
        }

        impl<$($typ : ValueTree),*> ValueTree
        for TupleValueTree<($($typ,)*)> {
            type Value = ($($typ::Value,)*);

            fn current(&self) -> Self::Value {
                ($(self.tree.$fld.current(),)*)
            }

            fn simplify(&mut self) -> bool {
                false
            }

            fn complicate(&mut self) -> bool {
                false
            }
        }
    }
}

tuple!(0: A);
tuple!(0: A, 1: B);
tuple!(0: A, 1: B, 2: C);
tuple!(0: A, 1: B, 2: C, 3: D);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);
tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L);
