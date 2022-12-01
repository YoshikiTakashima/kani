// a.rs -- first harness

#[kani::proof]
fn harness() {
    assert_eq!(2 + 2, 4);
}
