use super::*;

#[test]
fn l0_07_marked_selection_converts_utf16_to_character_offsets() {
    let text = "a😀中e\u{301}";
    assert_eq!(nsrange_to_rust_range(NSRange::new(1, 2), text), 1..2);
    assert_eq!(nsrange_to_rust_range(NSRange::new(3, 1), text), 2..3);
    assert_eq!(nsrange_to_rust_range(NSRange::new(6, 0), text), 5..5);
    assert_eq!(nsrange_to_rust_range(NSRange::new(0, 6), text), 0..5);
}

#[test]
fn l0_07_marked_selection_clamps_without_splitting_surrogates() {
    assert_eq!(nsrange_to_rust_range(NSRange::new(2, 0), "a😀中"), 1..1);
    assert_eq!(nsrange_to_rust_range(NSRange::new(2, 1), "a😀中"), 1..2);
    assert_eq!(
        nsrange_to_rust_range(NSRange::new(3, usize::MAX), "a😀中"),
        2..3
    );
    assert_eq!(
        nsrange_to_rust_range(NSRange::new(usize::MAX, 0), "a😀中"),
        3..3
    );
    assert_eq!(nsrange_to_rust_range(NSRange::new(3, 1), ""), 0..0);
}
