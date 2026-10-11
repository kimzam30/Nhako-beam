//! File helpers: safe names, unique paths, part files and free space.

use std::path::{Path, PathBuf};

pub const PART_SUFFIX: &str = ".nhakopart";

/// Makes a file name from another device safe to save here: base name only, no control
/// characters, nothing Windows refuses, and never empty.
pub fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let mut out: String = base
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    // Windows drops trailing dots and spaces; leading dots would hide the file on Linux.
    out = out.trim_end_matches(['.', ' ']).trim_start_matches(['.', ' ']).to_string();
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
        "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out = "file".into();
    }
    // Keep names a sensible length (255 bytes is the usual limit, leave room for " (99)" and the part suffix).
    while out.len() > 200 {
        let cut = out.char_indices().nth_back(0).map(|(i, _)| i).unwrap_or(0);
        out.truncate(cut);
    }
    out
}

/// `dir/name`, or `dir/name (1).ext`, `dir/name (2).ext`... if it (or its part file, or anything in
/// `taken`) already exists.
pub fn unique_path(dir: &Path, name: &str, taken: &[PathBuf]) -> PathBuf {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for n in 0.. {
        let candidate = if n == 0 { dir.join(name) } else { dir.join(format!("{stem} ({n}){ext}")) };
        if !candidate.exists() && !part_path(&candidate).exists() && !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

pub fn part_path(final_path: &Path) -> PathBuf {
    let mut s = final_path.as_os_str().to_owned();
    s.push(PART_SUFFIX);
    PathBuf::from(s)
}

/// Free bytes on the disk holding `dir` (or its nearest existing parent).
pub fn free_space(dir: &Path) -> Option<u64> {
    let mut p = Some(dir);
    while let Some(d) = p {
        if d.exists() {
            return fs4::available_space(d).ok();
        }
        p = d.parent();
    }
    None
}

pub fn mime_for(name: &str) -> String {
    mime_guess::from_path(name).first_or_octet_stream().essence_str().to_string()
}

pub fn is_disk_full(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::StorageFull || e.raw_os_error() == Some(28) /* ENOSPC */ || e.raw_os_error() == Some(112) /* ERROR_DISK_FULL */
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_made_safe() {
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("C:\\Users\\x\\photo.jpg"), "photo.jpg");
        assert_eq!(safe_name("a<b>c?.txt"), "a_b_c_.txt");
        assert_eq!(safe_name("CON.txt"), "_CON.txt");
        assert_eq!(safe_name(".bashrc"), "bashrc");
        assert_eq!(safe_name("..."), "file");
        assert_eq!(safe_name("name. "), "name");
        assert_eq!(safe_name("bell\u{7}.txt"), "bell_.txt");
        assert!(safe_name(&"é".repeat(300)).len() <= 200);
    }

    #[test]
    fn picks_free_names() {
        let dir = std::env::temp_dir().join(format!("nhako-files-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_path(&dir, "a.jpg", &[]), dir.join("a.jpg"));
        std::fs::write(dir.join("a.jpg"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "a.jpg", &[]), dir.join("a (1).jpg"));
        std::fs::write(part_path(&dir.join("a (1).jpg")), b"x").unwrap();
        assert_eq!(unique_path(&dir, "a.jpg", &[dir.join("a (2).jpg")]), dir.join("a (3).jpg"));
        assert_eq!(unique_path(&dir, "README", &[]), dir.join("README"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
