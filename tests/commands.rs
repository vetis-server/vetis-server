use assert_cmd::Command;
use predicates::prelude::predicate;

#[test]
fn test_no_arga() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("vetis-server").unwrap();
    cmd.assert()
        .success();
}
