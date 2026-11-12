//! Issues and pull requests from the hosting service (GitHub and GitHub Enterprise).
//!
//! Tokens: Whittle never writes a token to a file. It uses, in this order, a token the user
//! pasted (kept in the system keyring) or the user's existing GitHub CLI login (asked for at
//! call time, never copied).
pub mod github;

use std::path::Path;

/// Where the repository's `origin` lives.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RemoteRepo {
    pub host: String,
    pub owner: String,
    pub name: String,
}

/// Parses `https://host/o/r(.git)`, `ssh://git@host[:port]/o/r.git`, `git@host:o/r.git`,
/// `git://host/o/r`. Credentials in the URL are dropped.
pub fn parse_remote(url: &str) -> Option<RemoteRepo> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(char::is_whitespace) {
        return None;
    }
    let (host, path) = if let Some((_, rest)) = url.split_once("://") {
        let (authority, path) = rest.split_once('/')?;
        let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        (host_port.split(':').next()?.to_string(), path.to_string())
    } else {
        // scp-like: git@github.com:owner/repo.git
        let (left, path) = url.split_once(':')?;
        let host = left.rsplit_once('@').map_or(left, |(_, h)| h);
        (host.to_string(), path.to_string())
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    if host.is_empty() || owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(RemoteRepo {
        host: host.to_lowercase(),
        owner: owner.into(),
        name: name.into(),
    })
}

/// The default remote's repository, or why there isn't one.
pub fn repo_remote(path: &Path) -> Result<RemoteRepo, String> {
    let repo = git2::Repository::open(path).map_err(|e| e.message().to_string())?;
    let name = crate::git::default_remote(path)
        .map_err(|e| e.message().to_string())?
        .ok_or("This repository has no remote, so there are no issues or pull requests to show.")?;
    let remote = repo
        .find_remote(&name)
        .map_err(|e| e.message().to_string())?;
    let url = remote.url().ok_or("The remote has no URL.")?;
    parse_remote(url).ok_or_else(|| format!("Can't tell which repository “{url}” points to."))
}

/// What kind of host this is. Only GitHub-style APIs are implemented.
pub fn provider_of(host: &str) -> &'static str {
    if host.contains("gitlab") {
        "gitlab"
    } else if host.contains("bitbucket")
        || host.contains("dev.azure")
        || host.contains("visualstudio")
    {
        "other"
    } else {
        "github"
    }
}

pub fn api_base(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com".into()
    } else {
        format!("https://{host}/api/v3")
    }
}

/// UTC timestamp (`2026-08-14T00:00:00Z`) from `days` days before now.
pub fn iso_days_ago(days: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    iso_from_unix(now.saturating_sub(days * 86_400))
}

fn iso_from_unix(secs: u64) -> String {
    // civil-from-days (Howard Hinnant)
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let rem = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

// ---- tokens -----------------------------------------------------------------------

pub trait TokenStore {
    fn get(&self, host: &str) -> Option<String>;
    fn set(&self, host: &str, token: &str) -> Result<(), String>;
    fn delete(&self, host: &str) -> Result<(), String>;
}

/// The system keyring (GNOME Keyring / KWallet through the Secret Service).
pub struct KeyringStore;

impl KeyringStore {
    fn entry(host: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new("whittle", &format!("github:{host}")).map_err(|e| e.to_string())
    }
}

impl TokenStore for KeyringStore {
    fn get(&self, host: &str) -> Option<String> {
        Self::entry(host)
            .ok()?
            .get_password()
            .ok()
            .filter(|t| !t.is_empty())
    }
    fn set(&self, host: &str, token: &str) -> Result<(), String> {
        Self::entry(host)?.set_password(token).map_err(|e| {
            format!("Could not save the token in your system keyring: {e}. Is a keyring service (GNOME Keyring or KWallet) running?")
        })
    }
    fn delete(&self, host: &str) -> Result<(), String> {
        match Self::entry(host)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// The token of the user's GitHub CLI login, if `gh` is installed and signed in. Not stored.
pub fn gh_cli_token(host: &str) -> Option<String> {
    let out = std::process::Command::new("gh")
        .args(["auth", "token", "--hostname", host])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !t.is_empty()).then_some(t)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Auth {
    pub token: Option<String>,
    /// "keyring", "gh" or "none".
    pub source: &'static str,
}

/// A token pasted into Whittle wins; otherwise the GitHub CLI login.
pub fn resolve_auth(
    store: &dyn TokenStore,
    host: &str,
    gh: impl Fn(&str) -> Option<String>,
) -> Auth {
    if let Some(t) = store.get(host) {
        return Auth {
            token: Some(t),
            source: "keyring",
        };
    }
    match gh(host) {
        Some(t) => Auth {
            token: Some(t),
            source: "gh",
        },
        None => Auth {
            token: None,
            source: "none",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[test]
    fn parses_every_common_remote_form() {
        let r = |o: &str, n: &str| {
            Some(RemoteRepo {
                host: "github.com".into(),
                owner: o.into(),
                name: n.into(),
            })
        };
        for url in [
            "https://github.com/octo/hello.git",
            "https://github.com/octo/hello",
            "https://github.com/octo/hello/",
            "http://github.com/octo/hello.git",
            "https://someuser@github.com/octo/hello.git",
            "https://user:secret@github.com/octo/hello.git",
            "git@github.com:octo/hello.git",
            "git@github.com:octo/hello",
            "ssh://git@github.com/octo/hello.git",
            "ssh://git@github.com:22/octo/hello.git",
            "git://github.com/octo/hello.git",
            "  https://GitHub.com/octo/hello.git  ",
        ] {
            assert_eq!(parse_remote(url), r("octo", "hello"), "{url}");
        }
        assert_eq!(
            parse_remote("git@ghe.example.com:team/svc.git"),
            Some(RemoteRepo {
                host: "ghe.example.com".into(),
                owner: "team".into(),
                name: "svc".into()
            })
        );
        for bad in [
            "",
            "not a url",
            "/local/path/repo",
            "https://github.com/onlyowner",
            "https://github.com/a/b/c/d",
            "git@host:",
        ] {
            assert_eq!(parse_remote(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn formats_unix_time_as_iso() {
        assert_eq!(iso_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_from_unix(1_709_210_096), "2024-02-29T12:34:56Z");
        assert_eq!(iso_from_unix(1_798_761_599), "2026-12-31T23:59:59Z");
    }

    #[test]
    fn api_base_and_provider() {
        assert_eq!(api_base("github.com"), "https://api.github.com");
        assert_eq!(
            api_base("ghe.example.com"),
            "https://ghe.example.com/api/v3"
        );
        assert_eq!(provider_of("github.com"), "github");
        assert_eq!(provider_of("ghe.example.com"), "github");
        assert_eq!(provider_of("gitlab.com"), "gitlab");
        assert_eq!(provider_of("bitbucket.org"), "other");
    }

    struct MemStore(Mutex<HashMap<String, String>>);
    impl TokenStore for MemStore {
        fn get(&self, h: &str) -> Option<String> {
            self.0.lock().unwrap().get(h).cloned()
        }
        fn set(&self, h: &str, t: &str) -> Result<(), String> {
            self.0.lock().unwrap().insert(h.into(), t.into());
            Ok(())
        }
        fn delete(&self, h: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(h);
            Ok(())
        }
    }

    #[test]
    fn pasted_token_beats_gh_login_and_gh_is_the_fallback() {
        let store = MemStore(Mutex::new(HashMap::new()));
        let gh = |h: &str| (h == "github.com").then(|| "gh-token".to_string());
        assert_eq!(
            resolve_auth(&store, "github.com", gh),
            Auth {
                token: Some("gh-token".into()),
                source: "gh"
            }
        );
        assert_eq!(
            resolve_auth(&store, "other.host", gh),
            Auth {
                token: None,
                source: "none"
            }
        );
        store.set("github.com", "pasted").unwrap();
        assert_eq!(
            resolve_auth(&store, "github.com", gh),
            Auth {
                token: Some("pasted".into()),
                source: "keyring"
            }
        );
        store.delete("github.com").unwrap();
        assert_eq!(resolve_auth(&store, "github.com", gh).source, "gh");
    }

    #[test]
    fn repo_remote_reads_the_default_remote() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        assert!(repo_remote(dir.path()).unwrap_err().contains("no remote"));
        repo.remote("upstream", "https://github.com/other/fork.git")
            .unwrap();
        repo.remote("origin", "git@github.com:octo/hello.git")
            .unwrap();
        let r = repo_remote(dir.path()).unwrap();
        assert_eq!(
            (r.owner.as_str(), r.name.as_str()),
            ("octo", "hello"),
            "origin is preferred"
        );
        repo.remote_set_url("origin", "/some/local/path").unwrap();
        assert!(repo_remote(dir.path()).unwrap_err().contains("Can't tell"));
    }
}
