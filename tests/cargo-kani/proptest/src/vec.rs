// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Testing the vector feature

use core::ops::Range;
use proptest::strategy::*;
use proptest::collection::vec;

proptest::proptest! {
    #[cfg_attr(kani, kani::unwind(5))]
    fn vector_even_sums(
        vec_even in vec(
            (0..10).prop_map(|x: i32| x << 1),
            0..2
        ),
    ) {
        let sum: i32 = vec_even.into_iter().sum();
        assert!(sum  < 40, "each element is < 20, at most 2 elements");
        assert_eq!(sum % 2, 0, "Sum is even due to << 1.");
    }
}

