use std::path::PathBuf;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct PlaceHolder {
    #[serde(rename="match")]
    pub target: String,
    pub prompt: String,
    #[serde(default)]
    pub optional: bool,
    pub regex: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TemplateManifest {
    pub name: String,
    pub author: String,
    pub description: String,
    pub src: PathBuf,
    pub placeholders: Vec<PlaceHolder>
} 