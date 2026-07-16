#[cfg(all(feature = "gix", feature = "git_cli"))]
compile_error!(
    "Features `gix` and `git_cli` are mutually exclusive. Enable exactly one git backend, e.g. `cargo build --no-default-features --features cli,serde,git_cli`."
);

#[cfg(not(any(feature = "gix", feature = "git_cli")))]
compile_error!("Either the `gix` or the `git_cli` feature must be enabled to select a git backend.");

pub mod data;
pub mod exec;
pub mod paths;
pub mod prompt;
