//! #422 – Orphan-file regression lint.
//!
//! Every `.rs` file under `src/` must be declared as a module in `src/lib.rs`.
//! A file that exists on disk but is never declared silently stops compiling
//! (and running) — its tests, docs and safety guarantees become dead weight
//! that CI cannot see. CI compiles only what a module graph declares, so the
//! absence is invisible to `cargo check`/`clippy`/`test`.
//!
//! This lint closes that gap: when `cargo test --lib` runs, it diffs the files
//! on disk against the `mod` declarations in `lib.rs` and fails if any `.rs`
//! file is not declared. Adding a new source file means declaring it here —
//! exactly one line in `lib.rs`.

#![cfg(test)]

extern crate std;

use std::collections::HashSet;
use std::path::PathBuf;
use std::string::{String, ToString};
use std::vec::Vec;

fn declared_modules(lib_rs: &str) -> HashSet<String> {
    let mut declared = HashSet::new();
    for line in lib_rs.lines() {
        let mut t = line.trim();
        // Strip a leading visibility qualifier ("pub" / "pub(crate)" / "pub(super)").
        if let Some(rest) = t
            .strip_prefix("pub(crate)")
            .or_else(|| t.strip_prefix("pub(super)"))
        {
            t = rest.trim_start();
        } else if t.starts_with("pub ") {
            t = t.strip_prefix("pub ").unwrap();
        }
        if let Some(rest) = t.strip_prefix("mod ") {
            let name = rest.trim_end().trim_end_matches(';').trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                declared.insert(name.to_string());
            }
        }
    }
    declared
}

#[test]
fn test_no_orphan_source_files_under_src() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib_rs = std::fs::read_to_string(src_dir.join("lib.rs")).expect("src/lib.rs must be readable");

    let declared = declared_modules(&lib_rs);

    let mut orphans: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&src_dir).expect("src/ must be readable");
    for entry in entries {
        let entry = entry.expect("read_dir entry");
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !file_name.ends_with(".rs") || file_name == "lib.rs" || file_name == "main.rs" {
            continue;
        }
        let stem = file_name.trim_end_matches(".rs").to_string();
        if !declared.contains(&stem) {
            orphans.push(file_name);
        }
    }

    // The lint must not itself be an orphan.
    assert!(declared.contains("orphan_lint_tests"));
    assert!(
        orphans.is_empty(),
        "orphan source files (present under src/ but never declared in lib.rs): {}",
        orphans.join(", ")
    );
}

#[test]
fn test_orphan_lint_detects_a_declared_module() {
    // Sanity check the parser: it must recognise a plain "mod foo;" line so a
    // future parser regression cannot silently disable the lint.
    let declared =
        declared_modules("pub mod alpha;\n#[cfg(test)]\nmod beta;\npub(crate) mod gamma;\nmod delta {\n}\n");
    assert!(declared.contains("alpha"));
    assert!(declared.contains("beta"));
    assert!(declared.contains("gamma"));
    assert!(
        !declared.contains("delta"),
        "an inline module block must not be treated as a file module"
    );
}
