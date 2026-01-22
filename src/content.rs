use memmap2::Mmap;
use pathdiff::diff_paths;
use std::fs::File;
use std::path::{Path, PathBuf};
use twox_hash::XxHash3_128;
use walkdir::WalkDir;

pub fn calculate_hash(path: &Path) -> std::io::Result<u128> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let mut hasher = XxHash3_128::default();
    hasher.write(&mmap);
    Ok(hasher.finish_128())
}

pub fn get_file_size(path: &Path) -> std::io::Result<u64> {
    let meta = std::fs::metadata(path)?;
    Ok(meta.len())
}

pub struct FileInfo<'a> {
    pub path: &'a mut PathBuf,
    pub hash: u128,
    pub size: u64,
}

pub fn check_and_find_missing_files<'a>(
    search_root: &Path,
    files: impl Iterator<Item = FileInfo<'a>>,
) {
    let mut missing: Vec<_> = files
        .filter(|file| !file.path.exists())
        .map(|file| (file, None))
        .collect();

    for entry in WalkDir::new(search_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Ok(meta) = entry.metadata() {
                for (file, found) in &mut missing {
                    if found.is_some() {
                        continue;
                    }

                    if meta.len() == file.size {
                        if let Ok(current_hash) = calculate_hash(path) {
                            if current_hash == file.hash {
                                if let Some(path) = diff_paths(path, search_root) {
                                    *found = Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for (file, found) in missing {
        if let Some(path) = found {
            println!("{:?} => {path:?}", file.path);
            *file.path = path;
        } else {
            println!("{:?} => NOT FOUND", file.path);
        }
    }
}
