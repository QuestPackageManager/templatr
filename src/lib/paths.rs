use std::{path::{PathBuf, Path}, ffi::OsStr};

use super::exec::ExecError;


pub fn get_cache_folder() -> Result<PathBuf, ExecError> {
    let root = dirs::cache_dir().or_else(|| dirs::data_local_dir().or_else(dirs::data_dir));

    if root.is_none() {
        return Err(ExecError::NoCacheFound)
    }

    Ok(root.unwrap().join("templatr-rust"))
}

pub fn get_template_cache_path(name: &str) -> Result<PathBuf, ExecError> {
    let cache = get_cache_folder()?;

    Ok(cache.join(name))
}

/// Returns (name, path) 
pub fn get_template_cache_path_from_git(git: &str) -> Result<(&str, PathBuf), ExecError>  {
        let name_path = Path::new(git).file_name().unwrap_or_else(|| OsStr::new(git)); // TODO: Handle error properly
    let name = name_path.to_str().unwrap();

    Ok((name, get_template_cache_path(name)?))
}