//-
// Copyright 2017, 2018 The proptest developers
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

//! Arbitrary implementations for `std::boxed`.

use crate::std_facade::Box;
use crate::arbitrary::Arbitrary;
use crate::strategy::{Strategy, MapInto};
// wrap_from!(Box);

impl<A: Arbitrary> Arbitrary for Box<A> {
    type Parameters = A::Parameters;
    type Strategy = MapInto<A::Strategy, Self>;
    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        A::arbitrary_with(args)
            .prop_map_into()
    }
}

#[cfg(test)]
mod test {
    no_panic_test!(boxed => Box<u8>);
}
