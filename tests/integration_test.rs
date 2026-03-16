use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn envsafe() -> Command {
    Command::cargo_bin("envsafe").unwrap()
}

fn init_project(dir: &TempDir) {
    envsafe()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("envsafe initialized!"));
}

#[test]
fn test_help() {
    envsafe()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("envsafe"));
}

#[test]
fn test_version() {
    envsafe()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("envsafe"));
}

#[test]
fn test_init_creates_project() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    assert!(dir.path().join(".envsafe").exists());
    assert!(dir.path().join(".envsafe/config.json").exists());
    assert!(dir.path().join(".envsafe/vault.enc").exists());
}

#[test]
fn test_init_fails_if_already_initialized() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Already initialized"));
}

#[test]
fn test_set_and_get() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "MY_VAR", "hello123"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Set MY_VAR in [dev]"));

    envsafe()
        .args(["get", "MY_VAR"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hello123"));
}

#[test]
fn test_set_with_env() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "DB", "staging-db", "--env", "staging"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Set DB in [staging]"));

    envsafe()
        .args(["get", "DB", "--env", "staging"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("staging-db"));
}

#[test]
fn test_get_nonexistent_var() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["get", "NOPE"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_set_secret_and_ls_masked() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "TOKEN", "supersecret", "--secret"])
        .current_dir(dir.path())
        .assert()
        .success();

    // ls should mask secret values
    envsafe()
        .args(["ls"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("********"))
        .stdout(predicate::str::contains("supersecret").not());

    // ls --show should reveal values
    envsafe()
        .args(["ls", "--show"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("supersecret"));
}

#[test]
fn test_rm() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "TO_DELETE", "value"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["rm", "TO_DELETE"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed TO_DELETE"));

    envsafe()
        .args(["get", "TO_DELETE"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_envs_list() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "A", "1", "--env", "dev"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["set", "B", "2", "--env", "prod"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["envs"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dev"))
        .stdout(predicate::str::contains("prod"));
}

#[test]
fn test_diff() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "SHARED", "val1", "--env", "dev"])
        .current_dir(dir.path())
        .assert()
        .success();
    envsafe()
        .args(["set", "SHARED", "val2", "--env", "staging"])
        .current_dir(dir.path())
        .assert()
        .success();
    envsafe()
        .args(["set", "DEV_ONLY", "x", "--env", "dev"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["diff", "dev", "staging"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("SHARED"))
        .stdout(predicate::str::contains("DEV_ONLY"));
}

#[test]
fn test_export_json() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "KEY1", "value1"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["export", "--format", "json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""KEY1": "value1""#));
}

#[test]
fn test_export_shell() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "MY_KEY", "my_value"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["export", "--format", "shell"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("export MY_KEY='my_value'"));
}

#[test]
fn test_export_dotenv() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "DB", "postgres://localhost"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["export", "--format", "dotenv"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("DB=postgres://localhost"));
}

#[test]
fn test_lock_and_unlock() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "SECRET", "locked_value"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Lock
    envsafe()
        .args(["lock"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Vault locked"));

    assert!(dir.path().join(".env.vault").exists());

    // Unlock
    envsafe()
        .args(["unlock"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Vault unlocked"));

    // Value should still be accessible
    envsafe()
        .args(["get", "SECRET"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("locked_value"));
}

#[test]
fn test_key_export() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["key", "export"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Project key"));
}

#[test]
fn test_run_command() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "TEST_ENVSAFE_VAR", "injected_value"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Use printenv on Unix-like, or echo on Windows
    #[cfg(unix)]
    {
        envsafe()
            .args(["run", "--", "printenv", "TEST_ENVSAFE_VAR"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("injected_value"));
    }

    #[cfg(windows)]
    {
        envsafe()
            .args(["run", "--", "cmd", "/c", "echo", "%TEST_ENVSAFE_VAR%"])
            .current_dir(dir.path())
            .assert()
            .success();
    }
}

#[test]
fn test_ls_empty() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["ls"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No variables"));
}

#[test]
fn test_not_initialized_error() {
    let dir = TempDir::new().unwrap();

    envsafe()
        .args(["set", "KEY", "VALUE"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("envsafe init"));
}

#[test]
fn test_scan_clean_directory() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["scan"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No secrets detected"));
}

#[test]
fn test_export_unknown_format() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["export", "--format", "xml"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown format"));
}

#[test]
fn test_multiple_values_same_env() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "A", "1"])
        .current_dir(dir.path())
        .assert()
        .success();
    envsafe()
        .args(["set", "B", "2"])
        .current_dir(dir.path())
        .assert()
        .success();
    envsafe()
        .args(["set", "C", "3"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["ls"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("A"))
        .stdout(predicate::str::contains("B"))
        .stdout(predicate::str::contains("C"));
}

#[test]
fn test_overwrite_value() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    envsafe()
        .args(["set", "KEY", "old_value"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["set", "KEY", "new_value"])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["get", "KEY"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("new_value"));
}

#[test]
fn test_special_characters_in_value() {
    let dir = TempDir::new().unwrap();
    init_project(&dir);

    let value = "postgres://user:p@ss w0rd!#$%&@host:5432/db?sslmode=require";
    envsafe()
        .args(["set", "DB_URL", value])
        .current_dir(dir.path())
        .assert()
        .success();

    envsafe()
        .args(["get", "DB_URL"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(value));
}
