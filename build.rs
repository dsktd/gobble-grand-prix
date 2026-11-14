//! Embeds the build's version line, e.g. `v0.1.0-23 · 93dbd4e · Nov 14 15:02`, as `GOBBLE_VERSION`.

use std::process::Command;

fn run(
    program: &str,
    args: &[&str],
) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !text.is_empty()).then_some(text)
}

fn main() {
    // v0.1.0-23-g93dbd4e: the tag, commits since it, the short sha
    let version = run("git", &["describe", "--tags", "--long", "--match", "v*"])
        .and_then(|d| {
            let mut parts = d.rsplitn(3, '-');
            let sha = parts.next()?.trim_start_matches('g').to_string();
            let since = parts.next()?.to_string();
            let tag = parts.next()?.to_string();
            Some(format!("{tag}-{since} \u{b7} {sha}"))
        })
        .unwrap_or_else(|| format!("v{} \u{b7} dev", env!("CARGO_PKG_VERSION")));
    let when = run("date", &["-u", "+%b %d %H:%M"]).unwrap_or_default();
    let line = if when.is_empty() {
        version
    } else {
        format!("{version} \u{b7} {when}")
    };
    println!("cargo:rustc-env=GOBBLE_VERSION={line}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
    println!("cargo:rerun-if-changed=build.rs");
}
