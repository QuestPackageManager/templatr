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

use crate::git;

use super::{
    data::{PlaceHolder, TemplateManifest},
    paths::get_template_cache_path_from_git,
};

#[allow(dead_code)]
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

pub type Result<T> = core::result::Result<T, Box<ExecError>>;

pub fn get_or_clone_to_cache(git: &str) -> Result<TemplateManifest> {
    let path_info = get_template_cache_path_from_git(git)?;

    let template = path_info.path;

    if !template.exists() {
        fs::create_dir_all(&template).map_err(ExecError::IoError)?;

        Repository::clone_recurse(git, template).map_err(ExecError::GitError)?;
    } else {
        update_cache(&template)?;
    }

    get_manifest(git)
}

pub fn get_manifest(git: &str) -> Result<TemplateManifest> {
    let path_info = get_template_cache_path_from_git(git)?;

    let template = path_info.path;

    if !template.exists() {
        return Err(Box::new(ExecError::TemplateNotFound));
    }

    let manifest_file = template.join(".templatr");

    if !manifest_file.exists() {
        return Err(Box::new(ExecError::NoManifestFound));
    }

    let manifest: TemplateManifest =
        serde_json::from_reader(File::open(manifest_file).map_err(ExecError::IoError)?)
            .map_err(ExecError::SerdeError)?;

    if !template.join(&manifest.src).exists() {
        return Err(Box::new(ExecError::NoSrcFolderFound));
    }

    // validate all regexes
    for placeholder in &manifest.placeholders {
        if let Some(regex) = placeholder.regex.as_ref() {
            Regex::new(regex.as_str()).map_err(ExecError::RegexError)?;
        }
    }
    Ok(manifest)
}

// https://github.com/rust-lang/git2-rs/blob/88c67f788d59b4c180580b0ac6d119d42c59f61c/examples/pull.rs#L26
fn update_cache(template: &Path) -> Result<()> {
    println!("Checking for updates");
    let repo = Repository::open(template).map_err(ExecError::GitError)?;

    let remotes = repo.remotes().map_err(ExecError::GitError)?;
    let remote_name = remotes
        .get(0)
        .unwrap_or_else(|| panic!("No remote on this repo?"));

    let mut remote = repo.find_remote(remote_name).map_err(ExecError::GitError)?;

    remote
        .connect(git2::Direction::Fetch)
        .map_err(ExecError::GitError)?;

    let refs = &[remote
        .default_branch()
        .map_err(ExecError::GitError)?
        .as_str()
        .unwrap()
        .to_string()];

    remote.disconnect().map_err(ExecError::GitError)?;

    let fetch_commit: git2::AnnotatedCommit =
        git::do_fetch(&repo, refs, &mut remote).map_err(ExecError::GitError)?;

    // if fetch_commit.id() == repo.head().map_err(ExecError::GitError)?.peel_to_commit().unwrap().id() {
    //     println!("Already on latest");
    //     return Ok(());
    // }

    let refname = refs.get(0).unwrap();
    let mut branch = repo.find_reference(refname).map_err(ExecError::GitError)?;

    git::fast_forward(&repo, &mut branch, &fetch_commit).map_err(ExecError::GitError)?;
    println!("Checked out {}", fetch_commit.id());

    Ok(())
    // // remote.fetch(&[repo.head().map_err(ExecError::GitError)?.shorthand().unwrap()], None, None);
    // remote.fetch(&[remote.default_branch().map_err(ExecError::GitError)?.as_str().unwrap()], None, None);

    // remote.
}

/**
 * 
// https://github.com/rust-lang/git2-rs/blob/88c67f788d59b4c180580b0ac6d119d42c59f61c/examples/pull.rs#L26
fn update_cache(template: &Path) -> Result<()> {
    println!("Checking for updates");
    let repo = Repository::open(template).map_err(ExecError::GitError)?;

    let remotes = repo.remotes().map_err(ExecError::GitError)?;
    let remote_name = remotes
        .get(0)
        .unwrap_or_else(|| panic!("No remote on this repo?"));

    let mut remote = repo.find_remote(remote_name).map_err(ExecError::GitError)?;

    remote
        .connect(git2::Direction::Fetch)
        .map_err(ExecError::GitError)?;

    // Get main or master
    // I can't be bothered to support other branches
    let refs_vec = &[
        repo.find_branch("main", git2::BranchType::Local),
        repo.find_branch("master", git2::BranchType::Local),
    ];

    let refs = &[refs_vec
        .iter()
        .find(|r| r.is_ok() && r.as_ref().unwrap().upstream().is_ok())
        .unwrap()
        .as_ref()
        .unwrap()
        .name()
        .unwrap()
        .unwrap()];

    remote.disconnect().map_err(ExecError::GitError)?;

    let fetch_commit: git2::AnnotatedCommit =
        git::do_fetch(&repo, refs, &mut remote).map_err(ExecError::GitError)?;

    if fetch_commit.id()
        == repo
            .head()
            .map_err(ExecError::GitError)?
            .peel_to_commit()
            .unwrap()
            .id()
    {
        println!("Already on latest");
        return Ok(());
    }

    let refname = format!("refs/heads/{}", refs.first().unwrap());
    let mut branch = repo.find_reference(&refname).map_err(ExecError::GitError)?;

    git::fast_forward(&repo, &mut branch, &fetch_commit).map_err(ExecError::GitError)?;
    println!("Checked out {}", fetch_commit.id());

    Ok(())
    // // remote.fetch(&[repo.head().map_err(ExecError::GitError)?.shorthand().unwrap()], None, None);
    // remote.fetch(&[remote.default_branch().map_err(ExecError::GitError)?.as_str().unwrap()], None, None);

    // remote.
}
 */

pub fn copy_template(
    git: &str,
    target: &Path,
    placeholders: &HashMap<PlaceHolder, String>,
) -> Result<()> {
    let path_info = get_template_cache_path_from_git(git)?;
    let template = path_info.path;
    let manifest = get_manifest(git)?;
    let src = template.join(manifest.src);

    // validate placeholders

    // validate all placeholders are in hashmap
    let missing_placeholders: Vec<PlaceHolder> = manifest
        .placeholders
        .into_iter()
        .filter(|placeholder| {
            !placeholder.optional
            // true if no placeholder in the map has the same target
                && !placeholders
                    .iter()
                    .any(|p| p.0.target == placeholder.target)
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

    fs_extra::dir::copy(&src, target, &options).map_err(ExecError::CopyError)?;

    let mut open_options = File::options();
    open_options.read(true).write(true).append(false);

    for entry in WalkDir::new(target) {
        let entry = entry.map_err(ExecError::WalkDir)?;
        if entry.path().is_dir() {
            continue;
        }

        let mut contents = String::new();
        let mut edited = false;
        let mut file = open_options
            .open(entry.path().canonicalize().map_err(ExecError::IoError)?)
            .map_err(ExecError::IoError)?;
        file.read_to_string(&mut contents)
            .map_err(ExecError::IoError)?;
        for (placeholder, value) in placeholders.iter() {
            let new_contents = contents.replace(placeholder.target.as_str(), value);
            if new_contents != contents {
                contents = new_contents;
                edited = true;
            }
        }
        if edited {
            file.seek(SeekFrom::Start(0)).map_err(ExecError::IoError)?;
            let bytes = contents.into_bytes();
            file.set_len(bytes.len().try_into().expect("Casting to u64 failed"))
                .map_err(ExecError::IoError)?;
            file.write_all(bytes.as_slice())
                .map_err(ExecError::IoError)?;
        }
    }

    Ok(())
}
