use clap::Parser as _;

use super::TuiArgs;

#[test]
fn cloud_session_flags_are_not_accepted() {
    assert!(TuiArgs::try_parse_from(["term4u-tui", "--resume", "conversation"]).is_err());
    assert!(TuiArgs::try_parse_from(["term4u-tui", "--api-key", "secret"]).is_err());
    assert!(TuiArgs::try_parse_from(["term4u-tui", "--auto-approve"]).is_err());
}
