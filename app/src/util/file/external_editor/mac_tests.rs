use super::is_product_bundle;

#[test]
fn product_bundle_recognizes_term4u() {
    assert!(is_product_bundle("dev.term4u.Term4u"));
}

#[test]
fn product_bundle_rejects_other_apps() {
    assert!(!is_product_bundle("com.microsoft.VSCode"));
    assert!(!is_product_bundle("com.apple.TextEdit"));
    assert!(!is_product_bundle("dev.zed.Zed"));
    assert!(!is_product_bundle("invalid"));
    assert!(!is_product_bundle(""));
}
