use chrono::prelude::*;
use std::env::{
    self,
    consts::{ARCH, OS},
};
use std::fs;
use std::io::ErrorKind;
use std::ops::Add;
use std::path::Path;
use std::process::{Command, exit};

#[cfg(debug_assertions)]
const BUILD_TYPE: &'static str = "debug";
#[cfg(not(debug_assertions))]
const BUILD_TYPE: &'static str = "release";

#[cfg(feature = "db")]
const SHOULD_BUILD_DB: bool = true;
#[cfg(not(feature = "db"))]
const SHOULD_BUILD_DB: bool = false;

fn main() {
    if SHOULD_BUILD_DB {
        create_db();
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let version_path = Path::new(&out_dir).join("version");
    let mut result: Vec<String> = Vec::new();

    if let Some(ver) = option_env!("CARGO_PKG_VERSION") {
        result.push("v".to_string().add(ver));
    }

    if let Some(b_name) = get_branch_name().or(option_env!("CUSTOM_BRANCH").map(|i| i.to_string()))
    {
        result.push("branch:".to_string().add(b_name.as_str()));
    }

    if let Some(hash) =
        get_commit_hash().or(option_env!("CUSTOM_COMMIT_HASH").map(|i| i.to_string()))
    {
        result.push("hash:".to_string().add(hash.as_str()));
    }

    if is_working_tree_clean() {
        result.push("[clean]".to_string());
    }

    result.push("build:".to_string().add(BUILD_TYPE));
    result.push("os:".to_string().add(OS));
    result.push("arch:".to_string().add(ARCH));
    result.push("at ".to_string().add(Local::now().to_string().as_str()));

    fs::write(version_path, result.join(" ")).unwrap();
}

fn get_commit_hash() -> Option<String> {
    Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=format:%h")
        .output()
        .ok()
        .and_then(|output| {
            output
                .status
                .success()
                .then_some(String::from_utf8_lossy(&output.stdout).to_string())
        })
}

fn get_branch_name() -> Option<String> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()
        .and_then(|output| {
            output.status.success().then_some(
                String::from_utf8_lossy(&output.stdout)
                    .trim_end()
                    .to_string(),
            )
        })
}

fn is_working_tree_clean() -> bool {
    Command::new("git")
        .arg("diff")
        .arg("--quiet")
        .arg("--exit-code")
        .status()
        .ok()
        .and_then(|status| status.code())
        .map(|code| code == 0)
        .unwrap_or(false)
}


fn create_db() {
    Command::new("sqlx")
        .arg("db")
        .arg("create")
        .status()
        .unwrap_or_else(|e| {
            if e.kind() == ErrorKind::NotFound {
                println!("No sqlx executable in path!");
            }
            exit(1);
        });
    Command::new("sqlx")
        .arg("migrate")
        .arg("run")
        .status()
        .unwrap_or_else(|_| {
            exit(1);
        });
}
