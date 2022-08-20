// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Testing the vector feature

use proptest::strategy::*;
use proptest::collection::vec;

proptest! {
    fn vector_even_sums(
        vec_even in vec(
            0..10.prop_map(|x| x << 1),
            0..2
        ),
    ) {
        let sum = vec_eve.sum();
        assert!(sum  < 40, "each element is < 20, at most 2 elements");
        assert_rq!(sum % 2, 0, "Sum is even due to << 1.");
    }
}

