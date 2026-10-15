//! Battle's own files: the Handler's Notebook, kept in Battle's data folder and never in Desktop's.
//! A file that cannot be read is set aside, never deleted, and Battle starts a fresh one.

use formiga_battle_core::notebook::Page;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const NOTEBOOK_FILE: &str = "notebook.json";
const NOTEBOOK_FORMAT: &str = "formiga.battle.notebook";
const NOTEBOOK_VERSION: u32 = 1;
/// A notebook is a few KiB a creature; a colony of six with friends from next door, many times
/// over, is still well inside this.
const MAX_NOTEBOOK_BYTES: u64 = 1024 * 1024;

/// Battle's data folder: `FORMIGA_BATTLE_DATA_DIR` if it is set, for development and tests.
pub fn folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("FORMIGA_BATTLE_DATA_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("com", "Formiga", "Formiga Battle")
        .map(|dirs| dirs.data_dir().to_owned())
}

/// Every page, by colony and creature, so two colonies' companions never share a page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notebook {
    pub format: String,
    pub version: u32,
    pub pages: BTreeMap<String, Page>,
}

impl Notebook {
    pub fn key(colony_id: &str, creature_id: u64) -> String {
        format!("{colony_id}:{creature_id:016x}")
    }

    pub fn page(&self, colony_id: &str, creature_id: u64) -> Option<&Page> {
        self.pages.get(&Self::key(colony_id, creature_id))
    }

    pub fn page_mut(&mut self, colony_id: &str, creature_id: u64) -> &mut Page {
        self.pages
            .entry(Self::key(colony_id, creature_id))
            .or_default()
    }
}

/// The notebook in `folder`, or a fresh one.
pub fn load(folder: &Path) -> Notebook {
    let path = folder.join(NOTEBOOK_FILE);
    let fresh = Notebook {
        format: NOTEBOOK_FORMAT.to_owned(),
        version: NOTEBOOK_VERSION,
        pages: BTreeMap::new(),
    };
    let Ok(bytes) = formiga_expansion_rulebook::read_bounded(&path, MAX_NOTEBOOK_BYTES) else {
        if path.exists() {
            set_aside(&path);
        }
        return fresh;
    };
    match serde_json::from_slice::<Notebook>(&bytes) {
        Ok(notebook)
            if notebook.format == NOTEBOOK_FORMAT && notebook.version <= NOTEBOOK_VERSION =>
        {
            notebook
        }
        _ => {
            set_aside(&path);
            fresh
        }
    }
}

/// Writes the notebook whole, so a reader sees the old one or the new one.
pub fn save(folder: &Path, notebook: &Notebook) -> anyhow::Result<()> {
    std::fs::create_dir_all(folder)?;
    let bytes = serde_json::to_vec_pretty(notebook)?;
    formiga_expansion_rulebook::write_atomically(&folder.join(NOTEBOOK_FILE), &bytes)?;
    Ok(())
}

fn set_aside(path: &Path) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let aside = path.with_extension(format!("unreadable-{stamp}"));
    eprintln!("formiga-battle: setting aside {}", path.display());
    let _ = std::fs::rename(path, aside);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "formiga-battle-store-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_notebook_is_kept_and_read_back_whole() {
        let dir = scratch("keep");
        let mut notebook = load(&dir);
        notebook.page_mut("abc", 7).name = "Pip".to_owned();
        save(&dir, &notebook).unwrap();
        assert_eq!(load(&dir), notebook);
    }

    #[test]
    fn an_unreadable_notebook_is_set_aside_and_a_fresh_one_begun() {
        let dir = scratch("aside");
        std::fs::write(dir.join(NOTEBOOK_FILE), b"not a notebook").unwrap();
        let notebook = load(&dir);
        assert!(notebook.pages.is_empty());
        assert!(!dir.join(NOTEBOOK_FILE).exists());
        let kept = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(kept, 1, "the unreadable file is kept beside, not deleted");
    }
}
