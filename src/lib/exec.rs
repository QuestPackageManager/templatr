use std::{
    borrow::Borrow,
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    num::NonZero,
    path::Path,
    process::Command,
    sync::atomic::AtomicBool,
};

use fs_extra::dir::CopyOptions;
use gix::progress::Discard;
use regex::Regex;
use thiserror::Error;
use walkdir::WalkDir;

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
    GixCloneError(#[from] gix::clone::Error),
    #[error("Unable to fetch repository")]
    GixFetchError(#[from] gix::clone::fetch::Error),
    #[error("Unable to checkout repository")]
    GixCheckoutError(#[from] gix::clone::checkout::main_worktree::Error),
    #[error("Unable to checkout repository")]
    GixSubmoduleError(#[from] gix::submodule::modules::Error),

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

pub fn get_or_clone_to_cache(git: &str, branch: Option<&str>) -> Result<TemplateManifest> {
    let template_path_info = get_template_cache_path_from_git(git, branch)?;

    let template = template_path_info.path;

    if !template.exists() {
        fs::create_dir_all(&template).map_err(ExecError::IoError)?;

        let mut fetch_options =
            gix::prepare_clone(git, template.as_path()).map_err(ExecError::GixCloneError)?;
        fetch_options = fetch_options.with_ref_name(branch).expect("Branch name is not valid");
        fetch_options = fetch_options.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(
            NonZero::new(1).unwrap(),
        ));
        let (mut checkout, _outcome) = fetch_options
            .fetch_then_checkout(Discard {}, &AtomicBool::new(false))
            .map_err(ExecError::GixFetchError)?;

        let (repo, _outcome) = checkout
            .main_worktree(Discard {}, &AtomicBool::new(false))
            .map_err(ExecError::GixCheckoutError)?;

        let modules = repo.submodules().map_err(ExecError::GixSubmoduleError)?;
        if let Some(modules) = modules {
            modules
                .for_each(|submodule| {
                    println!(
                        "Warning! Submodules are not supported yet! Skipping {}",
                        submodule.path().unwrap()
                    );
                    // let submodule_path = repo.workdir_path(submodule.path().unwrap()).unwrap();

                    // let sub_repo = submodule.open()?.unwrap();

                    // let mut fetch = gix::prepare_clone(submodule.url().unwrap(), &submodule_path)?;
                    // fetch.with_ref_name()
                    // fetch.tags(gix::remote::fetch::Tags::All);
                    // let outcome = fetch
                    //     .fetch(&mut Discard {}, &AtomicBool::new(false))
                    //     .map_err(|e| gix::submodule::modules::Error::Update(e.into()))?;

                    // // Update submodule to latest commit on its branch
                    // let main_branch = sub_repo
                    //     .find_reference("HEAD")
                    //     .and_then(|head| head.resolve())
                    //     .map_err(|e| gix::submodule::modules::Error::Update(e.into()))?;

                    // if let Some(target_id) = main_branch.target().id() {
                    //     let checkout = gix::checkout::tree::Options::default();
                    //     sub_repo
                    //         .checkout_tree()
                    //         .options(checkout)
                    //         .commit(target_id)
                    //         .map_err(|e| gix::submodule::modules::Error::Update(e.into()))?;
                    // }
                    // Ok(())
                });
        }
    } else {
        update_cache(&template)?;
    }

    get_manifest(git, branch)
}

pub fn get_manifest(git: &str, branch: Option<&str>) -> Result<TemplateManifest> {
    let path_info = get_template_cache_path_from_git(git, branch)?;

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
    println!("Checking for updates through git pull");

    let mut process = Command::new("git");
    process.current_dir(template.parent().unwrap());
    process.arg("pull");

    let res = process.output();

    if let Err(e) = res {
        println!("Error: {e}");
    }

    Ok(())
}

pub fn copy_template(
    git: &str,
    branch: Option<&str>,
    target: &Path,
    placeholders: &HashMap<PlaceHolder, String>,
) -> Result<()> {
    let path_info = get_template_cache_path_from_git(git, branch)?;
    let template = path_info.path;
    let manifest = get_manifest(git, branch)?;
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
        let (mismatch_p, mismatch_v) = *invalid_values.first().unwrap();
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
