// b.rs -- second harness

#[kani::proof]
fn harness() {
    assert_eq!(5 + 5, 10);
}
