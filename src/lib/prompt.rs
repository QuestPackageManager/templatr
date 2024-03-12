use std::{collections::HashMap, fs, io, path::Path};

use color_eyre::{eyre::Context, owo_colors::OwoColorize};
use regex::Regex;

use crate::paths;

use super::{data::PlaceHolder, exec};

fn get_input() -> color_eyre::Result<String> {
    let mut ret = String::new();

    io::stdin()
        .read_line(&mut ret)
        .context("Unable to read input")?;

    Ok(ret.replace("\r\n", "").replace('\n', ""))
}

fn get_bool_user_input() -> color_eyre::Result<bool> {
    let v = get_input()?;
    Ok(v == "y" || v == "Y")
}

pub fn prompt(git: &str, dest: &str, branch: Option<&str>) -> color_eyre::Result<()> {
    println!(
        "Checking cache {}",
        paths::get_template_cache_path_from_git(git, branch)?
            .path
            .display()
            .yellow()
    );

    let manifest = match exec::get_manifest(git, branch) {
        // manifest exists
        Ok(manifest) => {
            // update cache
            exec::get_or_clone_to_cache(git, branch)?;
            Ok(manifest)
        }
        // if template not found
        Err(e) if matches!(*e, exec::ExecError::TemplateNotFound) => {
            println!(
                "{}",
                "Template not found in cache, do you want to continue? [Y/n]".yellow()
            );
            if !get_bool_user_input()? {
                // How to return main from here?
                std::process::exit(0);
            }

            exec::get_or_clone_to_cache(git, branch)
        }
        // raise error
        Err(e) => Err(e),
    }?;

    println!(
        "Using template {} by {}",
        manifest.name.green(),
        manifest.author.cyan()
    );
    println!("Template will be copied over to {}", dest.purple());
    println!("Do you want to continue? [Y/n]");
    if !get_bool_user_input()? {
        return Ok(());
    }

    let mut placeholders: HashMap<PlaceHolder, String> = HashMap::new();
    for placeholder in manifest.placeholders {
        println!(
            "{}: {} (optional: {}) Regex: {}",
            placeholder.target.cyan(),
            placeholder.prompt.yellow(),
            placeholder.optional,
            placeholder
                .regex
                .clone()
                .unwrap_or("N/A".to_string())
                .purple()
        );

        loop {
            let input = get_input()?;

            if input.is_empty() && !placeholder.optional {
                println!("Not optional, must provide a value");
                continue;
            }

            if let Some(regex_str) = placeholder.regex.as_ref() {
                let regex = Regex::new(regex_str)?;

                if !regex.is_match(&input) {
                    println!("Input does not match regex's requirements");
                    continue;
                }
            }

            placeholders.insert(placeholder, input);
            break;
        }
    }

    fs::create_dir_all(dest)?;

    exec::copy_template(git, branch, Path::new(&dest), &placeholders)?;

    Ok(())
}
