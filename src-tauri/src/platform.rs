//! Linux-specific startup fixes.
//!
//! Newer GNOME moved the font keys (`antialiasing`, `hinting`, `rgba-order`) out of the
//! `org.gnome.settings-daemon.plugins.xsettings` schema into a `.deprecated` one. GTK3 (which
//! WebKitGTK, and so Tauri, uses) still reads them from the old name and aborts with
//! "does not contain a key named 'antialiasing'". We restore the old schema in a private folder
//! and point GLib at it. System files are never touched.
//!
//! Also: terminals started from a snap-packaged app (VS Code's, for one) leak snap-only
//! environment variables such as `GIO_MODULE_DIR` and `GTK_PATH`. They make GTK and WebKit load
//! snap's own copies of their plug-in modules, which are built against an older libc and crash
//! with "symbol lookup error ... libpthread.so.0 ... __libc_pthread_init". We drop those entries.
use std::path::{Path, PathBuf};

const SCHEMA_ID: &str = "org.gnome.settings-daemon.plugins.xsettings";
const SYSTEM_SCHEMAS: &str = "/usr/share/glib-2.0/schemas";

/// Builds a schema file that defines the old schema name from the system's `.deprecated` one.
/// Returns `None` when the system file doesn't have that layout (nothing to fix).
pub fn compat_schema_xml(system_xml: &str) -> Option<String> {
    let open = format!("<schema id=\"{SCHEMA_ID}.deprecated\">");
    let start = system_xml.find(&open)?;
    let body_start = start + open.len();
    let body_end = body_start + system_xml[body_start..].find("</schema>")?;
    // an already-complete old schema means GTK finds its keys and no fix is needed
    let body = &system_xml[body_start..body_end];
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<schemalist>\n  <schema id=\"{SCHEMA_ID}\" path=\"/org/gnome/settings-daemon/plugins/xsettings/\">{body}</schema>\n</schemalist>\n"
    ))
}

fn data_home() -> PathBuf {
    match std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"),
    }
}

fn setup(system: &Path, out: &Path) -> Option<()> {
    let xml = std::fs::read_to_string(system.join(format!("{SCHEMA_ID}.gschema.xml"))).ok()?;
    let compat = compat_schema_xml(&xml)?;
    std::fs::create_dir_all(out).ok()?;
    std::fs::write(out.join(format!("{SCHEMA_ID}.gschema.xml")), compat).ok()?;
    // the font keys are enums defined in a sibling file
    std::fs::copy(
        system.join("org.gnome.settings-daemon.enums.xml"),
        out.join("org.gnome.settings-daemon.enums.xml"),
    )
    .ok()?;
    let status = std::process::Command::new("glib-compile-schemas")
        .arg(out)
        .status()
        .ok()?;
    status.success().then_some(())
}

/// Variables that snap launchers point at snap-private copies of system libraries/modules.
/// Each may hold a colon-separated list; only the snap entries are removed.
const SNAP_POLLUTED_VARS: &[&str] = &[
    "GIO_MODULE_DIR",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GDK_PIXBUF_MODULE_FILE",
    "GST_PLUGIN_PATH",
    "GST_PLUGIN_PATH_1_0",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GST_PLUGIN_SCANNER",
    "LD_LIBRARY_PATH",
    "XDG_DATA_DIRS",
    "XDG_CONFIG_DIRS",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "GSETTINGS_SCHEMA_DIR",
];

/// Paths inside a snap's own tree: `/snap/<name>/...` or a per-user `~/snap/<name>/...`.
/// (`/var/lib/snapd/desktop`, which the system adds on purpose, is not matched.)
fn is_snap_path(p: &Path) -> bool {
    let s = p.to_string_lossy();
    s.starts_with("/snap/") || (s.starts_with("/home/") && s.contains("/snap/"))
}

/// The value without its snap entries, or `None` if nothing is left.
fn strip_snap_entries(value: &std::ffi::OsStr) -> Option<std::ffi::OsString> {
    let kept: Vec<_> = std::env::split_paths(value)
        .filter(|p| !p.as_os_str().is_empty() && !is_snap_path(p))
        .collect();
    if kept.is_empty() {
        None
    } else {
        std::env::join_paths(kept).ok()
    }
}

/// Removes snap-only entries from the environment (see module docs). Returns the names changed.
pub fn sanitize_snap_env() -> Vec<&'static str> {
    let mut changed = Vec::new();
    for &name in SNAP_POLLUTED_VARS {
        let Some(value) = std::env::var_os(name) else {
            continue;
        };
        let cleaned = strip_snap_entries(&value);
        if cleaned.as_deref() == Some(value.as_os_str()) {
            continue;
        }
        match cleaned {
            Some(v) => std::env::set_var(name, v),
            None => std::env::remove_var(name),
        }
        changed.push(name);
    }
    changed
}

/// `ours` first, then whatever the environment already listed (GLib accepts a colon-separated
/// list). Ours must come first so our copy of the schema wins over a broken one further down.
fn merge_schema_dirs(ours: &Path, existing: Option<&std::ffi::OsStr>) -> std::ffi::OsString {
    let mut dirs = vec![ours.to_path_buf()];
    if let Some(e) = existing {
        dirs.extend(std::env::split_paths(e).filter(|d| d != ours && !d.as_os_str().is_empty()));
    }
    std::env::join_paths(dirs).unwrap_or_else(|_| ours.as_os_str().to_owned())
}

/// Call first thing in `main`, before GTK starts. Best effort: on any failure the app starts
/// exactly as it would have without this. Prints one line to stderr saying what happened, so a
/// crash report from a terminal shows whether the fix was active.
pub fn apply_gsettings_workaround() {
    let cleaned = sanitize_snap_env();
    if !cleaned.is_empty() {
        eprintln!(
            "whittle: ignored snap-only settings in {}",
            cleaned.join(", ")
        );
    }
    let out = data_home().join("whittle").join("gschemas");
    let ready =
        out.join("gschemas.compiled").is_file() || setup(Path::new(SYSTEM_SCHEMAS), &out).is_some();
    if !ready {
        eprintln!("whittle: GNOME font-settings workaround not needed or not possible");
        return;
    }
    let existing = std::env::var_os("GSETTINGS_SCHEMA_DIR");
    let merged = merge_schema_dirs(&out, existing.as_deref());
    std::env::set_var("GSETTINGS_SCHEMA_DIR", &merged);
    eprintln!("whittle: GSETTINGS_SCHEMA_DIR={}", merged.to_string_lossy());
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEW_LAYOUT: &str = r#"<schemalist>
  <schema id="org.gnome.settings-daemon.plugins.xsettings" path="/x/">
    <key name="overrides" type="a{sv}"><default>{}</default></key>
  </schema>
  <schema id="org.gnome.settings-daemon.plugins.xsettings.deprecated">
    <key name="antialiasing" enum="E"><default>'grayscale'</default></key>
    <key name="hinting" enum="H"><default>'slight'</default></key>
  </schema>
</schemalist>"#;

    #[test]
    fn restores_old_schema_name_from_deprecated_block() {
        let out = compat_schema_xml(NEW_LAYOUT).unwrap();
        assert!(out.contains(r#"<schema id="org.gnome.settings-daemon.plugins.xsettings" path="#));
        assert!(out.contains(r#"name="antialiasing""#) && out.contains(r#"name="hinting""#));
        assert!(!out.contains("deprecated"));
        assert!(
            !out.contains("overrides"),
            "only the font keys are carried over"
        );
    }

    #[test]
    fn nothing_to_fix_on_other_layouts() {
        assert!(compat_schema_xml("<schemalist/>").is_none());
        let old = r#"<schema id="org.gnome.settings-daemon.plugins.xsettings"><key name="antialiasing"/></schema>"#;
        assert!(compat_schema_xml(old).is_none());
    }

    #[test]
    fn our_schema_dir_goes_first_and_existing_ones_are_kept() {
        let ours = Path::new("/home/u/.local/share/whittle/gschemas");
        let m = merge_schema_dirs(ours, None);
        assert_eq!(m, ours.as_os_str());

        let existing = std::ffi::OsString::from("/snap/code/1/schemas:/opt/other");
        let m = merge_schema_dirs(ours, Some(&existing));
        let parts: Vec<_> = std::env::split_paths(&m).collect();
        assert_eq!(parts[0], ours);
        assert_eq!(
            parts[1..],
            [Path::new("/snap/code/1/schemas"), Path::new("/opt/other")]
        );

        // already listed: not duplicated
        let again = merge_schema_dirs(ours, Some(&m));
        assert_eq!(std::env::split_paths(&again).count(), 3);
        assert_eq!(
            merge_schema_dirs(ours, Some(std::ffi::OsStr::new(""))),
            ours.as_os_str()
        );
    }

    #[test]
    fn strips_snap_entries_but_keeps_system_ones() {
        let v = std::ffi::OsString::from(
            "/snap/core20/current/lib:/usr/lib/x86_64-linux-gnu:/home/david/snap/code/275/.local/share:/var/lib/snapd/desktop:",
        );
        let out = strip_snap_entries(&v).unwrap();
        let parts: Vec<_> = std::env::split_paths(&out).collect();
        assert_eq!(
            parts,
            [
                Path::new("/usr/lib/x86_64-linux-gnu"),
                Path::new("/var/lib/snapd/desktop")
            ]
        );
        // only snap entries: nothing left -> variable gets removed
        assert!(strip_snap_entries(std::ffi::OsStr::new(
            "/snap/code/current/usr/lib/gio/modules"
        ))
        .is_none());
        assert!(
            strip_snap_entries(std::ffi::OsStr::new("/home/u/snap/code/1/.local/share")).is_none()
        );
        // lookalikes that are not snap trees are kept
        assert!(strip_snap_entries(std::ffi::OsStr::new("/home/u/snapshots/x")).is_some());
        assert!(strip_snap_entries(std::ffi::OsStr::new("/opt/snap-tools/lib")).is_some());
    }
}
