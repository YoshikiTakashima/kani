// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! cargo-kani test crate for proptest.

mod proptest_library_is_linked;

// proptest::prop_compose! {
//   fn nearby_numbers()(centre in -1000..1000)
//                    (a in centre-10..centre+10,
//                     b in centre-10..centre+10)
//                    -> (i32, i32) {
//     (a, b)
//   }
// }

// proptest::proptest! {
//     fn sum_lower_than_2020((a,b) in nearby_numbers()) {
//         assert!(a + b < 2020, "each is < 1010, so sum is < 2020");
//     }
// }
