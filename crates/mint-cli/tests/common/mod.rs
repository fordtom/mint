#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static UNIQUE_FILE_ID: AtomicU64 = AtomicU64::new(0);

fn test_out_dir() -> PathBuf {
    std::env::temp_dir()
        .join("mint-cli-tests")
        .join(std::process::id().to_string())
}

pub fn ensure_out_dir() {
    fs::create_dir_all(test_out_dir()).unwrap();
}

pub fn write_layout_file(file_stem: &str, contents: &str) -> String {
    ensure_out_dir();
    let unique_id = UNIQUE_FILE_ID.fetch_add(1, Ordering::Relaxed);
    let path = test_out_dir().join(format!("{file_stem}-{unique_id}.toml"));
    std::fs::write(&path, contents).expect("write layout file");
    path.to_string_lossy().into_owned()
}

pub fn unique_out_path(stem: &str, ext: &str) -> PathBuf {
    ensure_out_dir();
    let unique_id = UNIQUE_FILE_ID.fetch_add(1, Ordering::Relaxed);
    test_out_dir().join(format!("{stem}-{unique_id}.{ext}"))
}

pub fn assert_out_file_exists(out_path: &Path) {
    assert!(
        out_path.exists(),
        "expected output file to exist: {}",
        out_path.display()
    );
}
