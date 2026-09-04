use super::{next_session_index, previous_session_index};

#[test]
fn next_tab_wraps_to_the_first_session() {
    assert_eq!(next_session_index(3, Some(2)), Some(0));
}

#[test]
fn previous_tab_wraps_to_the_last_session() {
    assert_eq!(previous_session_index(3, Some(0)), Some(2));
}

#[test]
fn tab_navigation_is_empty_without_sessions() {
    assert_eq!(next_session_index(0, None), None);
    assert_eq!(previous_session_index(0, None), None);
}
