use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../.git");
    println!("cargo:rerun-if-changed=build.rs");

    let git_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|output| String::from_utf8(output.stdout));
    if let Ok(Ok(hash)) = git_output {
        println!("cargo:rustc-env=GIT_HASH={}", hash.trim());
    }
}
