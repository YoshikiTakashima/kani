// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! proptest that picks one using the prop_oneof! macro.

use proptest::strategy::Just;
use proptest::{proptest, prop_oneof};

proptest! {
    fn possible_values_are_even(
        x in
            prop_oneof![
                1 => Just(0 as u32),
                2 => Just(2 as u32),
                0 => Just(3 as u32), // cannot be picked
            ]
    ) {
        assert_eq!(x % 2, 0, "Just(3) cannot be picked b/c weight is 0");
    }
}

