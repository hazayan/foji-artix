use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run_git(repo_path: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    output
}

/// Helper to create a test git repository
fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    // Initialize git repo
    run_git(path, &["init"]);

    // Configure git
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);

    dir
}

/// Helper to create a simple PKGBUILD file
fn create_pkgbuild(dir: &Path, pkgver: &str, pkgrel: &str) {
    let pkgname = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("test-package");

    let content = format!(
        r#"# Maintainer: Test <test@example.com>
pkgname={}
pkgver={}
pkgrel={}
pkgdesc="Test package"
arch=('x86_64')
license=('MIT')

package() {{
    echo "test"
}}
"#,
        pkgname, pkgver, pkgrel
    );

    fs::write(dir.join("PKGBUILD"), content).unwrap();
}

#[test]
fn test_list_packages_empty_repo() {
    let repo = create_test_repo();

    // Create initial commit
    fs::write(repo.path().join("README.md"), "# Test").unwrap();
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "Initial commit"]);

    // Build foji (assumes it's built)
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "list-packages",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_list_packages_with_package() {
    let repo = create_test_repo();

    // Create a package directory
    let pkg_dir = repo.path().join("packages").join("test-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    // Commit
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "Add test package"]);

    // List packages
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "list-packages",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test-pkg"));
}

#[test]
fn test_detect_changes_first_commit() {
    let repo = create_test_repo();

    // Create a package
    let pkg_dir = repo.path().join("test-pkg");
    fs::create_dir(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    // Commit
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "First commit"]);

    // Detect changes (should return all packages on first commit)
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "detect-changes",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test-pkg"));
}

#[test]
fn test_package_version() {
    let dir = TempDir::new().unwrap();
    create_pkgbuild(dir.path(), "2.5.1", "3");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "package-version",
            dir.path().join("PKGBUILD").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "2.5.1-3");
}

#[test]
fn test_detect_changes_json_format() {
    let repo = create_test_repo();

    // Create initial package
    let pkg_dir = repo.path().join("pkg1");
    fs::create_dir(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "Add pkg1"]);

    // Test JSON output format
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "detect-changes",
            "-r",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok());
}

#[test]
fn test_removed_package_not_reported_as_changed() {
    let repo = create_test_repo();

    // Create a package under packages/
    let pkg_dir = repo.path().join("packages").join("to-remove");
    fs::create_dir_all(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    // Commit with the package present
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "Add removable package"]);

    // Record this commit as the base ref
    let base_output = run_git(repo.path(), &["rev-parse", "HEAD"]);
    let base_ref = String::from_utf8(base_output.stdout).unwrap();
    let base_ref = base_ref.trim();

    // Now remove the package directory and commit the removal
    fs::remove_dir_all(&pkg_dir).unwrap();
    run_git(repo.path(), &["add", "-A"]);
    run_git(repo.path(), &["commit", "-m", "Remove package"]);

    // list-packages at HEAD should *not* include the removed package
    let list_output = Command::new("cargo")
        .args([
            "run",
            "--",
            "list-packages",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        !list_stdout.contains("to-remove"),
        "removed package should not appear in list-packages output"
    );

    // detect-changes from the recorded base_ref should *not* report the removed package,
    // documenting the current limitation that deletions are not surfaced as 'changed'
    let detect_output = Command::new("cargo")
        .args([
            "run",
            "--",
            "detect-changes",
            "-r",
            repo.path().to_str().unwrap(),
            "--base-ref",
            base_ref,
        ])
        .output()
        .unwrap();
    assert!(detect_output.status.success());
    let detect_stdout = String::from_utf8(detect_output.stdout).unwrap();
    assert!(
        !detect_stdout.contains("to-remove"),
        "removed package should not be reported as changed by detect-changes according to current semantics"
    );
}
