use std::{
    borrow::Borrow,
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use fs_extra::dir::CopyOptions;
use git2::Repository;
use regex::Regex;
use thiserror::Error;
use walkdir::WalkDir;

use super::{
    data::{PlaceHolder, TemplateManifest},
    paths::get_template_cache_path_from_git,
};

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("None of the expected cache folders for the system were found")]
    NoCacheFound,

    #[error("Unable to clone repository")]
    GitError(#[from] git2::Error),

    #[error("Unable to copy directory")]
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
    MissingPlaceholders(Vec<PlaceHolder>),
}

type SelfResult<T> = Result<T, Box<ExecError>>;

pub fn clone_to_cache(git: &str) -> SelfResult<TemplateManifest> {
    let (_, template) = get_template_cache_path_from_git(git)?;

    if template.exists() {
        return Err(Box::new(ExecError::TemplateAlreadyExists));

        // TODO: Update repository

        // let mut repo = Repository::open(template)?;
        // let branches: Vec<Branch> = repo.branches(Some(BranchType::Local))?.collect()?;
        // let branch = branches.get(0).ok_or(ExecError::NoBranchFound)?;
        // let upstream_name = repo.remotes()?.get(0).ok_or(ExecError::NoUpstreamFound)?;
        // let upstream = repo.branch_upstream_name(branch.name()?.unwrap())?;
        // repo.fetchhead_foreach(|refname, url, oid, merge| {

        // })?
    }

    fs::create_dir_all(&template).map_err(|e| ExecError::IoError(e))?;

    Repository::clone_recurse(git, template).map_err(|e| ExecError::GitError(e));

    get_manifest(git)
}

pub fn get_manifest(git: &str) -> SelfResult<TemplateManifest> {
    let (_, template) = get_template_cache_path_from_git(git)?;

    if !template.exists() {
        return Err(Box::new(ExecError::TemplateNotFound));
    }

    let manifest_file = template.join(".templatr");

    if !manifest_file.exists() {
        return Err(Box::new(ExecError::NoManifestFound));
    }

    let manifest: TemplateManifest =
        serde_json::from_reader(File::open(manifest_file).map_err(|e| ExecError::IoError(e))?)
            .map_err(|e| ExecError::SerdeError(e))?;

    if !template.join(&manifest.src).exists() {
        return Err(Box::new(ExecError::NoSrcFolderFound));
    }

    // validate all regexes
    for placeholder in &manifest.placeholders {
        if let Some(regex) = placeholder.regex.as_ref() {
            Regex::new(regex.as_str()).map_err(|e| ExecError::RegexError(e))?;
        }
    }
    Ok(manifest)
}

pub fn copy_template(
    git: &str,
    target: &Path,
    placeholders: &HashMap<PlaceHolder, String>,
) -> SelfResult<()> {
    let (_, template) = get_template_cache_path_from_git(git)?;
    let manifest = get_manifest(git)?;
    let src = template.join(manifest.src);

    // validate placeholders

    // validate all placeholders are in hashmap
    let missing_placeholders: Vec<PlaceHolder> = manifest
        .placeholders
        .into_iter()
        .filter(|placeholder| {
            !placeholder.optional && 
        // true if no placeholder in the map has the same target
        !placeholders.iter().any(|p| p.0.target == placeholder.target)
        })
        .collect();

    if !missing_placeholders.is_empty() {
        return Err(Box::new(ExecError::MissingPlaceholders(
            missing_placeholders,
        )));
    }

    // validate all regexes
    let invalid_values: Vec<(&PlaceHolder, &String)> = placeholders
        .iter()
        .filter(|(placeholder, value)| {
            placeholder.regex.borrow().is_some()
                && Regex::new(placeholder.regex.as_ref().unwrap().as_str())
                    .unwrap()
                    .is_match(value)
        })
        .collect();

    if !invalid_values.is_empty() {
        let (mismatch_p, mismatch_v) = *invalid_values.get(0).unwrap();
        return Err(Box::new(ExecError::PlaceholderDoesNotMatchRegex(
            mismatch_v.clone(),
            mismatch_p.regex.as_ref().unwrap().clone(),
            mismatch_p.clone(),
        )));
    }

    let mut options = CopyOptions::new(); //Initialize default values for CopyOptio
    options.content_only = true;
    options.overwrite = true;

    fs_extra::dir::copy(&src, &target, &options).map_err(|e| ExecError::CopyError(e))?;

    let mut open_options = File::options();
    open_options.read(true).write(true).append(false);

    for entry in WalkDir::new(target) {
        let entry = entry.map_err(|e| ExecError::WalkDir(e))?;
        if entry.path().is_dir() {
            continue;
        }

        let mut contents = String::new();
        let mut edited = false;
        let mut file = open_options
            .open(
                entry
                    .path()
                    .canonicalize()
                    .map_err(|e| ExecError::IoError(e))?,
            )
            .map_err(|e| ExecError::IoError(e))?;
        file.read_to_string(&mut contents)
            .map_err(|e| ExecError::IoError(e))?;
        for (placeholder, value) in placeholders.iter() {
            let new_contents = contents.replace(placeholder.target.as_str(), value);
            if new_contents != contents {
                contents = new_contents;
                edited = true;
            }
        }
        if edited {
            file.seek(SeekFrom::Start(0))
                .map_err(|e| ExecError::IoError(e))?;
            let bytes = contents.into_bytes();
            file.set_len(bytes.len().try_into().expect("Casting to u64 failed"))
                .map_err(|e| ExecError::IoError(e))?;
            file.write_all(bytes.as_slice())
                .map_err(|e| ExecError::IoError(e))?;
        }
    }

    Ok(())
}
