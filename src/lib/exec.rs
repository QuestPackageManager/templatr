use std::{path::{Path, PathBuf}, ffi::OsStr, collections::HashMap, fs::{File, self}, io::{self, Read, Write}, borrow::Borrow};

use fs_extra::dir::CopyOptions;
use git2::Repository;
use regex::Regex;
use thiserror::Error;
use walkdir::WalkDir;

use super::data::{TemplateManifest, PlaceHolder};

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("None of the expected cache folders for the system were found")]
    NoCacheFound,

    #[error("Unable to clone repository")]
    GitError(#[from] git2::Error),

    #[error("Unable to clone repository")]
    IoError(#[from] io::Error),

    #[error("Unable to copy template")]
    CopyError(#[from] fs_extra::error::Error),

    #[error("Unable to deserailize json")]
    SerdeError(#[from] serde_json::Error),

    #[error("Unable to parse regex")]
    RegexError(#[from] regex::Error),

    #[error("Unable to walk directory")]
    WalkDir(#[from] walkdir::Error),

    #[error("No upstream found")]
    NoUpstreamFound,

    #[error("No branch found")]
    NoBranchFound,

    #[error("Template already exists, cannot update")]
    TemplateAlreadyExists,

    #[error("Template not found in cache")]
    TemplateNotFound,

    #[error("Git repo has no template manifest .templatr in root")]
    NoManifestFound,

    #[error("Template src folder does not exist")]
    NoSrcFolderFound,

    #[error("String `{0}` does not match regex expression `{1}`")]
    PlaceholderDoesNotMatchRegex(String, String, PlaceHolder),

    #[error("Placeholders `{0:?}` have no values")]
    MissingPlaceholders(Vec<PlaceHolder>)
}

pub fn get_cache_folder() -> Result<PathBuf, ExecError> {
    let root = dirs::cache_dir().or_else(|| dirs::data_local_dir().or_else(dirs::data_dir));

    if root.is_none() {
        return Err(ExecError::NoCacheFound)
    }

    Ok(root.unwrap().join("templatr-rust"))
}

pub fn get_template_cache_path(str: &str) -> Result<PathBuf, ExecError> {
    let cache = get_cache_folder()?;

    Ok(cache.join(str))
}

pub fn get_template_cache_path_from_git(git: &str) -> Result<(&str, PathBuf), ExecError>  {
        let name_path = Path::new(git).file_name().unwrap_or_else(|| OsStr::new(git)); // TODO: Handle error properly
    let name = name_path.to_str().unwrap();

    Ok((name, get_template_cache_path(name)?))
}

pub fn clone_to_cache(git: &str) -> Result<TemplateManifest, ExecError> {
    let (_, template) = get_template_cache_path_from_git(git)?;

    if template.exists() {
        return Err(ExecError::TemplateAlreadyExists);

        // TODO: Update repository


        // let mut repo = Repository::open(template)?;
        // let branches: Vec<Branch> = repo.branches(Some(BranchType::Local))?.collect()?;
        // let branch = branches.get(0).ok_or(ExecError::NoBranchFound)?;
        // let upstream_name = repo.remotes()?.get(0).ok_or(ExecError::NoUpstreamFound)?;
        // let upstream = repo.branch_upstream_name(branch.name()?.unwrap())?;
        // repo.fetchhead_foreach(|refname, url, oid, merge| {

        // })?



    }

    fs::create_dir_all(&template)?;

    Repository::clone_recurse(git, template)?;

    get_manifest(git)
}

pub fn get_manifest(git: &str) -> Result<TemplateManifest, ExecError> {
    let (_, template) = get_template_cache_path_from_git(git)?;

    if !template.exists() {
        return Err(ExecError::TemplateNotFound);
    }

    let manifest_file = template.join(".templatr");
    
    if !manifest_file.exists() {
        return Err(ExecError::NoManifestFound);
    }
    
    let manifest: TemplateManifest = serde_json::from_reader(File::open(manifest_file)?)?;

    if !manifest.src.exists() {
        return Err(ExecError::NoSrcFolderFound);
    }

    // validate all regexes
    for placeholder in &manifest.placeholders {
        if let Some(regex) = placeholder.regex.as_ref() {
            Regex::new(regex.as_str())?;
        }
    }
    Ok(manifest)
}

pub fn copy_template(git: &str, target: &Path, placeholders: &HashMap<PlaceHolder, String>) -> Result<(), ExecError> {
    let (_, template) = get_template_cache_path_from_git(git)?;
    let manifest = get_manifest(git)?;
    let src = template.join(manifest.src);
    
    // validate placeholders

    // validate all placeholders are in hashmap
    let missing_placeholders: Vec<PlaceHolder> = manifest.placeholders.into_iter()
    .filter(|placeholder| 
        !placeholder.optional && 
        // true if no placeholder in the map has the same target
        !placeholders.iter().any(|p| p.0.target == placeholder.target)
    ).collect();

    if !missing_placeholders.is_empty() {
        return Err(ExecError::MissingPlaceholders(missing_placeholders));
    }



    // validate all regexes
    let invalid_values: Vec<(&PlaceHolder, &String)> = placeholders.iter().filter(|(placeholder, value)| placeholder.regex.borrow().is_some() && Regex::new(placeholder.regex.as_ref().unwrap().as_str()).unwrap().is_match(value)).collect();

    if !invalid_values.is_empty() {
        let (mismatch_p, mismatch_v) = *invalid_values.get(0).unwrap();
        return Err(ExecError::PlaceholderDoesNotMatchRegex(mismatch_v.clone(), mismatch_p.regex.as_ref().unwrap().clone(), mismatch_p.clone()));
    }

    
    let options = CopyOptions::new(); //Initialize default values for CopyOptio

    fs_extra::dir::copy(src, target, &options)?;

    for entry in WalkDir::new(target) {
        let entry = entry?;

        let mut contents = String::new();
        let mut file = File::open(entry.path())?;
        file.read_to_string(&mut contents)?;
        for (placeholder, value) in placeholders.iter() {
            contents = contents.replace(placeholder.target.as_str(), value);
        }
        file.write_all(contents.into_bytes().as_slice())?;
    }

    Ok(())
}