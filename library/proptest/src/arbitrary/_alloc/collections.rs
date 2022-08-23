// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

use crate::arbitrary::Arbitrary;
use crate::collection::{SizeRange, VecStrategy};

impl<A : Arbitrary> Arbitrary for Vec<A> {
    type Parameters = A::Parameters;
    type Strategy = VecStrategy<A::Strategy>;

    fn arbitrary_with(parameters: Self::Parameters) -> Self::Strategy {
        crate::collection::vec(A::arbitrary_with(parameters), SizeRange::default())
    }
}
