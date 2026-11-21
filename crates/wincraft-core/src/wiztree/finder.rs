use std::path::{Path, PathBuf};

const KNOWN_PATHS: &[&str] = &[
    r"C:\Program Files\WizTree\WizTree64.exe",
    r"C:\Program Files\WizTree\WizTree.exe",
    r"C:\Program Files (x86)\WizTree\WizTree64.exe",
    r"C:\Program Files (x86)\WizTree\WizTree.exe",
];

pub fn find_wiztree() -> Result<PathBuf, String> {
    for candidate in KNOWN_PATHS {
        let p = Path::new(candidate);
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let p64 = dir.join("WizTree64.exe");
            if p64.is_file() {
                return Ok(p64);
            }
            let p32 = dir.join("WizTree.exe");
            if p32.is_file() {
                return Ok(p32);
            }
        }
    }

    Err("WizTree executable (WizTree64.exe) not found on system. Ensure WizTree is installed in Program Files or available on PATH.".to_string())
}
