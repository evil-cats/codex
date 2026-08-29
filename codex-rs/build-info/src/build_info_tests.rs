use std::fs;

use codex_install_context::InstallContext;
use pretty_assertions::assert_eq;
use semver::Version;
use tempfile::tempdir;

use crate::BUILD_COMMIT_STAMP_LEN;
use crate::BuildInfo;
use crate::decode_build_commit_stamp;
use crate::encode_build_commit_stamp;

const BUILD_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// Фиксированный stamp сохраняет короткое development-значение и полный Git SHA.
#[test]
fn build_commit_stamp_round_trips_supported_revisions() {
    static DEVELOPMENT_STAMP: [u8; BUILD_COMMIT_STAMP_LEN] =
        encode_build_commit_stamp(/*build_commit*/ None);
    static RELEASE_STAMP: [u8; BUILD_COMMIT_STAMP_LEN] =
        encode_build_commit_stamp(Some(BUILD_COMMIT));

    assert_eq!(
        (
            decode_build_commit_stamp(&DEVELOPMENT_STAMP),
            decode_build_commit_stamp(&RELEASE_STAMP),
        ),
        ("dev", BUILD_COMMIT),
    );
}

/// Payload длиннее полного SHA не может незаметно изменить соседние ELF-секции.
#[test]
#[should_panic]
fn build_commit_stamp_rejects_oversized_value() {
    encode_build_commit_stamp(Some("0123456789abcdef0123456789abcdef012345678"));
}

/// A packaged runtime takes its release identity from its package manifest.
#[test]
fn packaged_runtime_uses_manifest_version() {
    let package = tempdir().expect("create runtime package");
    let bin_dir = package.path().join("bin");
    fs::create_dir(&bin_dir).expect("create runtime binary directory");
    let executable = bin_dir.join("codex");
    fs::write(&executable, b"").expect("create runtime binary");
    fs::write(
        package.path().join("codex-package.json"),
        r#"{"version":"1.2.3-alpha.4"}"#,
    )
    .expect("create runtime package manifest");

    let context = InstallContext::from_exe(
        cfg!(target_os = "macos"),
        Some(&executable),
        /*method_override*/ None,
    );

    assert_eq!(
        BuildInfo::resolve(&context, BUILD_COMMIT),
        BuildInfo {
            version: Version::parse("1.2.3-alpha.4").expect("valid release version"),
            build_commit: BUILD_COMMIT.to_string(),
        },
    );
}

/// Unpackaged builds expose their stamped commit and structured source version.
#[test]
fn unpackaged_runtime_uses_build_commit() {
    let context = InstallContext::from_exe(
        cfg!(target_os = "macos"),
        /*current_exe*/ None,
        /*method_override*/ None,
    );

    assert_eq!(
        BuildInfo::resolve(&context, BUILD_COMMIT),
        BuildInfo {
            version: Version::new(0, 0, 0),
            build_commit: BUILD_COMMIT.to_string(),
        },
    );
}

/// Older package layouts without release metadata retain their build identity.
#[test]
fn legacy_package_without_version_uses_build_commit() {
    let package = tempdir().expect("create runtime package");
    let bin_dir = package.path().join("bin");
    fs::create_dir(&bin_dir).expect("create runtime binary directory");
    let executable = bin_dir.join("codex");
    fs::write(&executable, b"").expect("create runtime binary");
    fs::write(package.path().join("codex-package.json"), "{}")
        .expect("create legacy runtime package manifest");

    let context = InstallContext::from_exe(
        cfg!(target_os = "macos"),
        Some(&executable),
        /*method_override*/ None,
    );

    assert_eq!(
        BuildInfo::resolve(&context, BUILD_COMMIT),
        BuildInfo {
            version: Version::new(0, 0, 0),
            build_commit: BUILD_COMMIT.to_string(),
        },
    );
}

/// Invalid package versions cannot override the executable's stamped commit.
#[test]
fn invalid_package_version_uses_build_commit() {
    let package = tempdir().expect("create runtime package");
    let bin_dir = package.path().join("bin");
    fs::create_dir(&bin_dir).expect("create runtime binary directory");
    let executable = bin_dir.join("codex");
    fs::write(&executable, b"").expect("create runtime binary");
    fs::write(
        package.path().join("codex-package.json"),
        r#"{"version":"not-a-release-version"}"#,
    )
    .expect("create runtime package manifest");

    let context = InstallContext::from_exe(
        cfg!(target_os = "macos"),
        Some(&executable),
        /*method_override*/ None,
    );

    assert_eq!(
        BuildInfo::resolve(&context, BUILD_COMMIT),
        BuildInfo {
            version: Version::new(0, 0, 0),
            build_commit: BUILD_COMMIT.to_string(),
        },
    );
}

/// Serializing build information preserves both its release version and commit.
#[test]
fn build_info_serialization_preserves_build_provenance() {
    let build_info = BuildInfo {
        version: Version::parse("1.2.3-alpha.4").expect("valid release version"),
        build_commit: BUILD_COMMIT.to_string(),
    };

    assert_eq!(
        serde_json::to_string(&build_info).expect("serialize build information"),
        format!("{{\"version\":\"1.2.3-alpha.4\",\"build_commit\":\"{BUILD_COMMIT}\"}}"),
    );
    assert_eq!(
        serde_json::from_str::<BuildInfo>(&format!(
            "{{\"version\":\"1.2.3-alpha.4\",\"build_commit\":\"{BUILD_COMMIT}\"}}"
        ))
        .expect("deserialize build information"),
        build_info,
    );
}
