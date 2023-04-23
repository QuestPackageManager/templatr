use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use super::exec::ExecError;
use super::exec::Result;

pub struct PathInfo<'a> {
    pub name: &'a str,
    pub parent: Option<&'a str>,

    pub git: &'a str,
    pub path: PathBuf,
}

pub fn get_cache_folder() -> Result<PathBuf> {
    let root = dirs::cache_dir().or_else(|| dirs::data_local_dir().or_else(dirs::data_dir));

    if root.is_none() {
        return Err(Box::new(ExecError::NoCacheFound));
    }

    Ok(root.unwrap().join("templatr-rust"))
}

pub fn get_template_cache_path(name: &str) -> Result<PathBuf> {
    let cache = get_cache_folder()?;

    Ok(cache.join(name))
}

/// Returns (name, path)
pub fn get_template_cache_path_from_git(git: &str) -> Result<PathInfo> {
    let name_path = Path::new(git)
        .file_name()
        .unwrap_or_else(|| OsStr::new(git)); // TODO: Handle error properly

    let parent_path = Path::new(git)
        .parent()
        .map(|o| o.file_name().unwrap_or_default().to_str().unwrap()); // TODO: Handle error properly

    let path_itself = if let Some(parent_path) = parent_path {
        Path::new(parent_path)
            .join(name_path)
            .to_str()
            .unwrap()
            .to_string()
    } else {
        name_path.to_str().unwrap().to_string()
    };

    let name = name_path.to_str().unwrap();

    Ok(PathInfo {
        name,
        parent: parent_path,
        git,
        path: get_template_cache_path(&path_itself)?,
    })
}
