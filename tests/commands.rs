use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn missing_config_reports_error() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("vetis-server").unwrap();
    cmd.arg("start")
        .assert()
        .success()
        .stderr(contains("Failed to start server: No config file specified"));
}

#[test]
fn missing_config_file_reports_error() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("vetis-server").unwrap();
    cmd.args(["start", "--config", "missing-vetis-test-config.yaml"])
        .assert()
        .success()
        .stderr(contains(
            "Failed to start server: Config file does not exist: missing-vetis-test-config.yaml",
        ));
}
