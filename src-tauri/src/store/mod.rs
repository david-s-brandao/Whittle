//! Persistent projects (JSON under `~/.config/whittle/`). Never stores credentials.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub path: String,
    pub name: String,
    pub starred: bool,
    /// Unix seconds of last use.
    pub last_used: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("corrupt projects file: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project not in list: {0}")]
    Unknown(String),
}

/// User preferences, stored in `settings.json` next to the project list. No secrets in here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    /// Where new clones go by default (empty = ask / `~/projects`).
    pub clone_dir: Option<String>,
    /// Path to the git executable (empty = the one on PATH).
    pub git_path: Option<String>,
    /// Command used by "Open in editor", e.g. `code` or `subl`. Empty = auto-detect.
    pub editor_cmd: Option<String>,
    /// Command used by "Open in terminal". Empty = auto-detect.
    pub terminal_cmd: Option<String>,
    /// Interface font size in px (10-20).
    pub font_size: u8,
    /// Accent colour as `#rrggbb`.
    pub accent: String,
    /// Background fetch interval in minutes; zero disables it.
    pub auto_fetch_minutes: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            clone_dir: None,
            git_path: None,
            editor_cmd: None,
            terminal_cmd: None,
            font_size: 13,
            accent: "#f66008".into(),
            auto_fetch_minutes: 5,
        }
    }
}

impl Settings {
    /// Clamps and checks values; empty strings become `None`. Does not run anything.
    pub fn sanitized(mut self) -> Result<Self, String> {
        let norm = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.clone_dir = norm(self.clone_dir);
        self.git_path = norm(self.git_path);
        self.editor_cmd = norm(self.editor_cmd);
        self.terminal_cmd = norm(self.terminal_cmd);
        self.font_size = self.font_size.clamp(10, 20);
        if self.auto_fetch_minutes > 60 {
            return Err("Auto-fetch must be disabled (0) or between 1 and 60 minutes.".into());
        }
        let a = self.accent.trim();
        let ok =
            a.len() == 7 && a.starts_with('#') && a[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !ok {
            return Err("The accent colour must look like #f66008.".into());
        }
        self.accent = a.to_lowercase();
        if let Some(d) = &self.clone_dir {
            if !Path::new(d).is_dir() {
                return Err(format!("The clone folder “{d}” does not exist."));
            }
        }
        Ok(self)
    }
}

pub fn load_settings(file: &Path) -> Settings {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(file: &Path, settings: &Settings) -> Result<(), StoreError> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(settings)?)?;
    std::fs::rename(tmp, file)?;
    Ok(())
}

pub struct ProjectStore {
    file: PathBuf,
    projects: Vec<Project>,
}

fn config_base() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(x);
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
}

/// Default location: `$XDG_CONFIG_HOME/whittle` or `~/.config/whittle`.
pub fn config_dir() -> PathBuf {
    config_base().join("whittle")
}

/// Copies the project list from the old `knife` config folder (the app's former name),
/// once, if the new folder has none yet. The old folder is left untouched.
pub fn migrate_legacy_config() -> std::io::Result<()> {
    migrate_projects(&config_base().join("knife"), &config_dir())
}

fn migrate_projects(old: &Path, new: &Path) -> std::io::Result<()> {
    let (from, to) = (old.join("projects.json"), new.join("projects.json"));
    if from.is_file() && !to.exists() {
        std::fs::create_dir_all(new)?;
        std::fs::copy(from, to)?;
    }
    Ok(())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl ProjectStore {
    pub fn load(file: PathBuf) -> Result<Self, StoreError> {
        let projects = match std::fs::read_to_string(&file) {
            Ok(s) => serde_json::from_str(&s)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { file, projects })
    }

    fn save(&self) -> Result<(), StoreError> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.projects)?)?;
        std::fs::rename(tmp, &self.file)?;
        Ok(())
    }

    /// Starred first, then by most recently used.
    pub fn list(&self) -> Vec<Project> {
        let mut v = self.projects.clone();
        v.sort_by(|a, b| {
            b.starred
                .cmp(&a.starred)
                .then(b.last_used.cmp(&a.last_used))
        });
        v
    }

    /// Adds the project, or marks an existing one as just used.
    pub fn add_or_touch(&mut self, path: &Path) -> Result<Project, StoreError> {
        let key = path.to_string_lossy().into_owned();
        let t = now();
        let idx = match self.projects.iter().position(|p| p.path == key) {
            Some(i) => {
                self.projects[i].last_used = t;
                i
            }
            None => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| key.clone());
                self.projects.push(Project {
                    path: key,
                    name,
                    starred: false,
                    last_used: t,
                });
                self.projects.len() - 1
            }
        };
        self.save()?;
        Ok(self.projects[idx].clone())
    }

    pub fn set_starred(&mut self, path: &str, starred: bool) -> Result<(), StoreError> {
        let p = self
            .projects
            .iter_mut()
            .find(|p| p.path == path)
            .ok_or_else(|| StoreError::Unknown(path.into()))?;
        p.starred = starred;
        self.save()
    }

    /// Removes from the list only; files on disk are untouched.
    pub fn remove(&mut self, path: &str) -> Result<(), StoreError> {
        self.projects.retain(|p| p.path != path);
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &tempfile::TempDir) -> ProjectStore {
        ProjectStore::load(dir.path().join("projects.json")).unwrap()
    }

    #[test]
    fn add_persist_star_remove() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(&dir);
        s.add_or_touch(Path::new("/a/one")).unwrap();
        s.add_or_touch(Path::new("/a/two")).unwrap();
        s.add_or_touch(Path::new("/a/one")).unwrap(); // no duplicate
        assert_eq!(s.list().len(), 2);

        s.set_starred("/a/two", true).unwrap();
        assert_eq!(s.list()[0].path, "/a/two");

        let reloaded = store(&dir);
        assert!(reloaded.list()[0].starred);

        s.remove("/a/two").unwrap();
        assert_eq!(store(&dir).list().len(), 1);
        assert!(s.set_starred("/nope", true).is_err());
    }

    #[test]
    fn recent_sorted_by_last_used() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(&dir);
        s.add_or_touch(Path::new("/x/old")).unwrap();
        s.add_or_touch(Path::new("/x/new")).unwrap();
        s.projects[0].last_used = 10;
        s.projects[1].last_used = 20;
        assert_eq!(s.list()[0].path, "/x/new");
    }

    #[test]
    fn migrates_legacy_projects_once_without_overwriting() {
        let base = tempfile::tempdir().unwrap();
        let (old, new) = (base.path().join("knife"), base.path().join("whittle"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("projects.json"), "[]").unwrap();

        migrate_projects(&old, &new).unwrap();
        assert_eq!(
            std::fs::read_to_string(new.join("projects.json")).unwrap(),
            "[]"
        );
        assert!(old.join("projects.json").exists(), "old file is kept");

        std::fs::write(new.join("projects.json"), "[1]").unwrap();
        migrate_projects(&old, &new).unwrap();
        assert_eq!(
            std::fs::read_to_string(new.join("projects.json")).unwrap(),
            "[1]"
        );
        migrate_projects(&base.path().join("missing"), &base.path().join("x")).unwrap();
    }

    #[test]
    fn settings_defaults_roundtrip_and_validation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        assert_eq!(
            load_settings(&file),
            Settings::default(),
            "missing file = defaults"
        );
        std::fs::write(&file, "not json").unwrap();
        assert_eq!(
            load_settings(&file),
            Settings::default(),
            "corrupt file = defaults, no crash"
        );
        std::fs::write(&file, r#"{"font_size": 15}"#).unwrap();
        let partial = load_settings(&file);
        assert_eq!(
            partial.auto_fetch_minutes, 5,
            "existing settings get auto-fetch defaults"
        );
        assert_eq!(
            (partial.font_size, partial.accent.as_str()),
            (15, "#f66008"),
            "missing keys fall back"
        );

        let s = Settings {
            editor_cmd: Some("  code  ".into()),
            git_path: Some("   ".into()),
            font_size: 40,
            accent: "#ABCDEF".into(),
            clone_dir: Some(dir.path().to_string_lossy().into_owned()),
            ..Default::default()
        }
        .sanitized()
        .unwrap();
        assert_eq!(s.editor_cmd.as_deref(), Some("code"));
        assert_eq!(s.git_path, None, "blank means unset");
        assert_eq!(s.font_size, 20, "clamped");
        assert_eq!(s.accent, "#abcdef");
        save_settings(&file, &s).unwrap();
        assert_eq!(load_settings(&file), s);
        for minutes in [0, 1, 5, 60] {
            let configured = Settings {
                auto_fetch_minutes: minutes,
                ..Default::default()
            }
            .sanitized()
            .unwrap();
            save_settings(&file, &configured).unwrap();
            assert_eq!(load_settings(&file).auto_fetch_minutes, minutes);
        }
        assert!(Settings {
            auto_fetch_minutes: 61,
            ..Default::default()
        }
        .sanitized()
        .is_err());

        for bad in ["red", "#fff", "#gggggg", "f66008", "#f66008;"] {
            assert!(
                Settings {
                    accent: bad.into(),
                    ..Default::default()
                }
                .sanitized()
                .is_err(),
                "{bad}"
            );
        }
        assert!(Settings {
            clone_dir: Some("/definitely/not/here".into()),
            ..Default::default()
        }
        .sanitized()
        .unwrap_err()
        .contains("does not exist"));
    }
}
