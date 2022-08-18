// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! proptest that picks one using the prop_oneof! macro.

proptest::proptest! {
    fn possible_values_are_even(_ in proptest::strategy::Just(())) {
        let x =
            proptest::prop_oneof![
                1 => 0,
                2 => 2,
                0 => 3, // cannot be picked
            ];
        assert_eq!(x % 2, 0, "Only possible choice is 0");
    }
}

