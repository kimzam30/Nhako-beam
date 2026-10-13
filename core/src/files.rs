//! File helpers: safe names, unique paths, part files, free space, and opening files to send.

use std::path::{Path, PathBuf};

pub const PART_SUFFIX: &str = ".nhakopart";

/// Opens a `content://` address for reading: set by the Android app, for files in a folder
/// picked with the system picker. See [`set_content_opener`].
pub type ContentOpener = dyn Fn(&str) -> std::io::Result<std::fs::File> + Send + Sync;
static CONTENT_OPENER: std::sync::OnceLock<Box<ContentOpener>> = std::sync::OnceLock::new();

/// Lets [`open_to_send`] open `content://` addresses. Each file is opened when it's needed and
/// closed after, so a folder of thousands never holds thousands of handles. Only the first call counts.
pub fn set_content_opener(opener: Box<ContentOpener>) {
    let _ = CONTENT_OPENER.set(opener);
}

/// Opens a file to send. The Android app hands the engine files that Android opened for it (from
/// the share sheet or the file picker) as `/proc/self/fd/<n>`. Opening that path again goes back
/// through Android's storage layer, which refuses it for files from apps like Files, so the engine
/// reads through the file Android already opened. The app keeps it open until the transfer ends.
pub fn open_to_send(path: &Path) -> std::io::Result<std::fs::File> {
    if let Some(uri) = path.to_str().filter(|p| p.starts_with("content://")) {
        let open = CONTENT_OPENER.get().ok_or_else(|| std::io::Error::other("can't open content addresses here"))?;
        let file = open(uri)?;
        if !file.metadata()?.is_file() {
            return Err(std::io::Error::other("not a file"));
        }
        return Ok(file);
    }
    #[cfg(unix)]
    if let Some(fd) = path.to_str().and_then(|p| p.strip_prefix("/proc/self/fd/")).and_then(|n| n.parse::<u16>().ok()) {
        use std::os::fd::BorrowedFd;
        // SAFETY: a descriptor number the app passed for this transfer and keeps open while it runs;
        // it's duplicated here, so the engine closing its copy never closes the app's.
        let file = std::fs::File::from(unsafe { BorrowedFd::borrow_raw(i32::from(fd)) }.try_clone_to_owned()?);
        if !file.metadata()?.is_file() {
            return Err(std::io::Error::other("not a file"));
        }
        return Ok(file);
    }
    std::fs::File::open(path)
}

/// Makes a file name from another device safe to save here: base name only, no control
/// characters, nothing invisible that could disguise it (`invoice{U+202E}fdp.exe` shows as
/// `invoiceexe.pdf`), nothing Windows refuses, and never empty.
pub fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let mut out: String = base
        .chars()
        .filter(|c| !crate::text::is_invisible(*c))
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

/// The deepest folder an incoming file may sit in, counted from the folder that was sent.
pub const FOLDER_DEPTH_MAX: usize = 32;

/// A folder path from another device (`Photos/2024`, `/` between folders) made safe to save
/// under: each folder name is made safe like a file name, and empty, `.` and `..` parts are
/// dropped, so nothing can climb out of the save folder. `None` when no folder is left, or when
/// it's deeper than [`FOLDER_DEPTH_MAX`].
pub fn safe_folder(path: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = path
        .split(['/', '\\'])
        .map(|p| p.trim())
        .filter(|p| !p.is_empty() && *p != "." && *p != "..")
        .map(safe_name)
        .collect();
    (!parts.is_empty() && parts.len() <= FOLDER_DEPTH_MAX).then_some(parts)
}

/// Makes a new, empty folder for an incoming folder: `dir/name`, or `dir/name (1)`, `dir/name (2)`...
/// when that's taken by anything (a file, a folder, a link). It's only ever made new, so files
/// arriving never mix with what was already there.
pub fn reserve_folder(dir: &Path, name: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    for n in 0..10_000 {
        let candidate = if n == 0 { dir.join(name) } else { dir.join(format!("{name} ({n})")) };
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("no free folder name"))
}

/// Removes the empty folders inside `top`, and `top` itself if that leaves it empty. For a
/// cancelled folder: its part files are gone, and so are the folders made for them. Links aren't
/// followed, and a folder with anything left in it stays.
pub fn prune_empty(top: &Path) {
    fn prune(dir: &Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else { return false };
        let mut empty = true;
        for e in entries.flatten() {
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            if !(is_dir && prune(&e.path())) {
                empty = false;
            }
        }
        empty && std::fs::remove_dir(dir).is_ok()
    }
    prune(top);
}

/// Files the system leaves in folders, never worth sending.
const CLUTTER: [&str; 3] = [".DS_Store", "Thumbs.db", "desktop.ini"];

/// The files inside a folder being sent, each with the folder it's in, starting with the sent
/// folder's own name (`Photos`, `Photos/2024`), in a steady order. Links to folders aren't
/// followed (they can loop), and system clutter (`.DS_Store`, `Thumbs.db`, `desktop.ini`) is left
/// out. Fails with `max` or more files, rather than offering more than the other device takes.
pub fn walk_folder(root: &Path, max: usize) -> std::io::Result<Vec<(PathBuf, String)>> {
    let top = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Folder".into());
    let mut out = vec![];
    let mut stack = vec![(root.to_path_buf(), top)];
    while let Some((dir, folder)) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        let mut subfolders = vec![];
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(kind) = e.file_type() else { continue };
            if kind.is_dir() {
                subfolders.push((e.path(), format!("{folder}/{name}")));
            } else if CLUTTER.contains(&name.as_str()) {
                continue;
            } else if kind.is_file() || (kind.is_symlink() && std::fs::metadata(e.path()).is_ok_and(|m| m.is_file())) {
                if out.len() >= max {
                    return Err(std::io::Error::other(format!("more than {max} files")));
                }
                out.push((e.path(), folder.clone()));
            }
        }
        // Taken off the end, so the first comes next.
        stack.extend(subfolders.into_iter().rev());
    }
    Ok(out)
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

/// Picks a free name for an incoming file and creates its empty part file. The part file is only
/// ever created new, so a file or link someone left under that name is never written through,
/// and the next file wanting the name sees it and picks another.
pub fn reserve(dir: &Path, name: &str) -> std::io::Result<(PathBuf, PathBuf)> {
    let mut taken = vec![];
    loop {
        let final_path = unique_path(dir, name, &taken);
        taken.push(final_path.clone());
        let part = part_path(&final_path);
        match create_new(&part) {
            Ok(()) => return Ok((final_path, part)),
            // Appeared since we looked: try the next name.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

/// Creates an empty file, failing if anything (a file, a folder, a link) is already there.
pub fn create_new(path: &Path) -> std::io::Result<()> {
    std::fs::OpenOptions::new().write(true).create_new(true).open(path).map(drop)
}

/// How a part file is opened to write into: never through a link put in its place.
pub fn part_options() -> tokio::fs::OpenOptions {
    let mut o = tokio::fs::OpenOptions::new();
    o.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    o.custom_flags(libc::O_NOFOLLOW);
    o
}

/// Moves a checked part file to its final name without replacing anything already there. Where
/// the disk can't do that (no hard links, as on some phone storage), it checks first instead.
pub fn finish_part(part: &Path, final_path: &Path) -> std::io::Result<PathBuf> {
    let dir = final_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let name = final_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut target = final_path.to_path_buf();
    for _ in 0..100 {
        match std::fs::hard_link(part, &target) {
            Ok(()) => {
                std::fs::remove_file(part)?;
                mark_from_elsewhere(&target);
                return Ok(target);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => target = unique_path(&dir, &name, &[]),
            Err(_) => break,
        }
    }
    if target.exists() || target.symlink_metadata().is_ok() {
        target = unique_path(&dir, &name, &[]);
    }
    std::fs::rename(part, &target)?;
    mark_from_elsewhere(&target);
    Ok(target)
}

/// Windows: marks a received file as coming from another computer (the "Mark of the Web" that
/// browsers add to downloads), so SmartScreen and Office Protected View check it before it runs
/// or opens with editing on. Elsewhere it does nothing.
pub fn mark_from_elsewhere(path: &Path) {
    #[cfg(windows)]
    {
        let mut stream = path.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        // Zone 3 is "Internet", the zone browsers use for downloads.
        if let Err(e) = std::fs::write(&stream, "[ZoneTransfer]\r\nZoneId=3\r\n") {
            tracing::debug!(%e, "couldn't mark {} as received", path.display());
        }
    }
    #[cfg(not(windows))]
    let _ = path;
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

    #[cfg(unix)]
    #[test]
    fn reads_a_file_android_opened_even_when_its_path_is_off_limits() {
        use std::{io::Read, os::fd::AsRawFd};
        let dir = crate::store::tests::tempdir("open-to-send");
        let path = dir.join("photo.jpg");
        std::fs::write(&path, b"pixels").unwrap();
        let held = std::fs::File::open(&path).unwrap();
        // The app can't reach the file by its path any more, only through the open descriptor.
        std::fs::remove_file(&path).unwrap();
        let link = PathBuf::from(format!("/proc/self/fd/{}", held.as_raw_fd()));
        let mut text = String::new();
        open_to_send(&link).unwrap().read_to_string(&mut text).unwrap();
        assert_eq!(text, "pixels");
        assert_eq!(open_to_send(&link).unwrap().metadata().unwrap().len(), 6);
    }

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
        assert_eq!(safe_name("invoice\u{202E}fdp.exe"), "invoicefdp.exe");
        assert_eq!(safe_name("photo\u{200B}.jpg"), "photo.jpg");
        assert!(safe_name(&"é".repeat(300)).len() <= 200);
    }

    #[test]
    fn folder_paths_stay_inside() {
        let f = |p: &str| safe_folder(p);
        assert_eq!(f("Photos/2024"), Some(vec!["Photos".into(), "2024".into()]));
        assert_eq!(f("../../etc"), Some(vec!["etc".into()]));
        assert_eq!(f("/abs//./x\\y"), Some(vec!["abs".into(), "x".into(), "y".into()]));
        assert_eq!(f("C:\\Users"), Some(vec!["C_".into(), "Users".into()]));
        assert_eq!(f("a\u{202E}b/CON"), Some(vec!["ab".into(), "_CON".into()]));
        assert_eq!(f(".."), None);
        assert_eq!(f(""), None);
        assert_eq!(f(&vec!["d"; FOLDER_DEPTH_MAX + 1].join("/")), None);
        assert!(f(&vec!["d"; FOLDER_DEPTH_MAX].join("/")).is_some());
    }

    #[test]
    fn incoming_folders_never_mix_with_existing_ones() {
        let dir = crate::store::tests::tempdir("reserve-folder");
        std::fs::create_dir(dir.join("Photos")).unwrap();
        std::fs::write(dir.join("Photos (1)"), b"a file").unwrap();
        assert_eq!(reserve_folder(&dir, "Photos").unwrap(), dir.join("Photos (2)"));
        assert_eq!(reserve_folder(&dir, "Photos").unwrap(), dir.join("Photos (3)"));
        let deep = dir.join("Photos (3)/a/b");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir_all(dir.join("Photos (3)/c")).unwrap();
        std::fs::write(dir.join("Photos (3)/c/keep.txt"), b"x").unwrap();
        prune_empty(&dir.join("Photos (3)"));
        assert!(!dir.join("Photos (3)/a").exists());
        assert!(dir.join("Photos (3)/c/keep.txt").exists(), "a folder with something in it stays");
        std::fs::remove_file(dir.join("Photos (3)/c/keep.txt")).unwrap();
        prune_empty(&dir.join("Photos (3)"));
        assert!(!dir.join("Photos (3)").exists());
        assert!(dir.join("Photos").exists(), "never goes above the top");
    }

    #[test]
    fn walks_a_folder_in_order() {
        let dir = crate::store::tests::tempdir("walk");
        let root = dir.join("Trip");
        std::fs::create_dir_all(root.join("day 2/raw")).unwrap();
        std::fs::create_dir_all(root.join("empty")).unwrap();
        std::fs::write(root.join("b.jpg"), b"b").unwrap();
        std::fs::write(root.join("a.jpg"), b"a").unwrap();
        std::fs::write(root.join("Thumbs.db"), b"x").unwrap();
        std::fs::write(root.join("day 2/c.jpg"), b"c").unwrap();
        std::fs::write(root.join("day 2/raw/d.raw"), b"d").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&root, root.join("day 2/loop")).unwrap();
        let got: Vec<(String, String)> = walk_folder(&root, 100)
            .unwrap()
            .into_iter()
            .map(|(p, f)| (p.file_name().unwrap().to_string_lossy().into_owned(), f))
            .collect();
        let want = [("a.jpg", "Trip"), ("b.jpg", "Trip"), ("c.jpg", "Trip/day 2"), ("d.raw", "Trip/day 2/raw")];
        assert_eq!(got, want.map(|(a, b)| (a.to_string(), b.to_string())));
        assert!(walk_folder(&root, 3).is_err(), "too many files");
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

    #[cfg(unix)]
    #[test]
    fn never_writes_through_a_planted_link() {
        let dir = crate::store::tests::tempdir("planted-link");
        let elsewhere = dir.join("elsewhere.txt");
        // A link waiting where the part file for "a.jpg" would go, pointing somewhere else.
        std::os::unix::fs::symlink(&elsewhere, part_path(&dir.join("a.jpg"))).unwrap();
        let (final_path, part) = reserve(&dir, "a.jpg").unwrap();
        assert_eq!(final_path, dir.join("a (1).jpg"), "skips the name with the link");
        assert!(part.symlink_metadata().unwrap().is_file());
        assert!(!elsewhere.exists(), "the link's target was never created");
    }

    #[test]
    fn finishing_never_replaces_a_file() {
        let dir = crate::store::tests::tempdir("finish-part");
        let (final_path, part) = reserve(&dir, "notes.txt").unwrap();
        std::fs::write(&part, b"received").unwrap();
        // Someone saves their own notes.txt while the transfer runs.
        std::fs::write(&final_path, b"mine").unwrap();
        let saved = finish_part(&part, &final_path).unwrap();
        assert_eq!(saved, dir.join("notes (1).txt"));
        assert_eq!(std::fs::read(&final_path).unwrap(), b"mine");
        assert_eq!(std::fs::read(&saved).unwrap(), b"received");
        assert!(!part.exists());
    }
}
