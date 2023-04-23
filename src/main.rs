use clap::Parser;

use templatr::prompt::prompt;

/// Templatr rust rewrite (implementation not based on the old one)
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Link to the git repo, sinonymous with the git clone link
    #[clap(short, long)]
    git: String,

    /// Destination where template will be copied to. FILES WILL BE OVERWRITTEN
    dest: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let git = args.git;
    let dest = args.dest;

    prompt(&git, &dest)
}
