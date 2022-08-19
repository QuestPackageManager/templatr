mod lib;

use std::{io, fs, collections::HashMap, path::Path};

use clap::Parser;
use color_eyre::{eyre::Context, owo_colors::OwoColorize};
use regex::Regex;
use templatr::{exec, data::PlaceHolder};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long)]
    git: String,


    dest: String
}

fn get_input() -> color_eyre::Result<String> {
    let mut ret = String::new();

    io::stdin()
        .read_line(&mut ret)
        .context("Unable to read input")?;

    Ok(ret)
}

fn get_bool_input() -> color_eyre::Result<bool> {
    let v = get_input()?;
    Ok(v == "y" || v == "Y")
}


fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let git = args.git;
    let dest = args.dest;

    let manifest = match exec::get_manifest(&git) {
        Ok(manifest) => Ok(manifest),
        Err(e) => match e {
            exec::ExecError::TemplateNotFound => {
                println!("{}", "Template not found in cache, do you want to continue? [Y/n]".yellow());
                if !get_bool_input()? {
                    // How to return main from here?
                    std::process::exit(0);
                }

                exec::clone_to_cache(&git)
            },
            _ => Err(e)
        },
    }?;

    println!("Using template {} by {}", manifest.name.green(), manifest.author.cyan());
    println!("Template will be copied over to {}", dest.purple());
    println!("Do you want to continue? [Y/n]");
    if !get_bool_input()? {
        return Ok(());
    }

    let mut placeholders: HashMap<PlaceHolder, String> = HashMap::new();
    for placeholder in manifest.placeholders {
        println!("{}: {} (optional: {}) Regex: {}", placeholder.target.cyan(), placeholder.prompt.yellow(), placeholder.optional, placeholder.regex.clone().unwrap_or_else(|| "".to_string()).purple());
            
        loop {
            let input = get_input()?;

            

            if input.is_empty() && !placeholder.optional {
                println!("Not optional, must provide a value");
                continue;
            }

            if let Some(regex_str) = placeholder.regex.as_ref() {
                let regex = Regex::new(regex_str)?;
                
                if !regex.is_match(&input) {
                    println!("Input does not match regex's requirements")
                    continue;
                }
            }

            placeholders.insert(placeholder, input);
            break;
        }
    }

    fs::create_dir_all(&dest)?;
    

    
    exec::copy_template(&git, Path::new(&dest), &placeholders)?;


    Ok(())
}
