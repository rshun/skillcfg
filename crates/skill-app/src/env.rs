use skill_core::{profile::Base, Platform};
use std::path::PathBuf;

#[derive(Clone)]
pub struct HostEnv {
    pub home: PathBuf,
    pub appdata: Option<PathBuf>,
    pub xdg_config: Option<PathBuf>,
    pub project_root: Option<PathBuf>,
    pub search_path: Vec<PathBuf>,
    pub platform: Platform,
}
impl HostEnv {
    pub fn base(&self, base: Base) -> Option<&PathBuf> {
        match base { Base::Home => Some(&self.home), Base::Project => self.project_root.as_ref(), Base::Appdata => self.appdata.as_ref(), Base::XdgConfig => self.xdg_config.as_ref() }
    }
}
