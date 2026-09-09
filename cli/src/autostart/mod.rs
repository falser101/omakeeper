use crate::util;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const LOCKED: &[&str] = &[
    "at-spi-dbus-bus",
    "gnome-keyring-pkcs11",
    "gnome-keyring-secrets",
    "xdg-user-dirs",
    "user-dirs-update-gtk",
];

#[derive(Debug, Clone, Serialize)]
pub struct AutostartItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub exec: String,
    pub icon: String,
    pub enabled: bool,
    pub source: String,
    pub path: String,
    pub user_added: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableApp {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Serialize)]
struct Report {
    items: Vec<AutostartItem>,
    available: Vec<AvailableApp>,
}

struct Desktop {
    id: String,
    path: PathBuf,
    name: String,
    comment: String,
    exec: String,
    icon: String,
    hidden: bool,
    no_display: bool,
    gnome_enabled: Option<bool>,
    type_application: bool,
    stub: bool,
}

pub fn run(
    json: bool,
    enable: Option<&str>,
    disable: Option<&str>,
    add: Option<&str>,
    remove: Option<&str>,
) -> Result<()> {
    if let Some(id) = enable {
        return mutate(id, Action::Enable, json);
    }
    if let Some(id) = disable {
        return mutate(id, Action::Disable, json);
    }
    if let Some(id) = add {
        return mutate(id, Action::Add, json);
    }
    if let Some(id) = remove {
        return mutate(id, Action::Remove, json);
    }
    let report = scan()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if report.items.is_empty() {
        println!("No XDG autostart entries.");
        return Ok(());
    }
    for item in &report.items {
        let mark = if item.enabled { "on " } else { "off" };
        let src = if item.user_added { "user" } else { "system" };
        println!("  [{mark}] {}  ({src})  {}", item.name, item.id);
    }
    Ok(())
}

enum Action {
    Enable,
    Disable,
    Add,
    Remove,
}

fn mutate(id: &str, action: Action, json: bool) -> Result<()> {
    let id = strip_desktop(id);
    match action {
        Action::Enable => set_enabled(&id, true)?,
        Action::Disable => set_enabled(&id, false)?,
        Action::Add => add_app(&id)?,
        Action::Remove => remove_app(&id)?,
    }
    if json {
        let report = scan()?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn scan() -> Result<Report> {
    let system = load_dir(&system_dir());
    let user = load_dir(&user_dir()?);
    let mut ids: Vec<String> = system.keys().chain(user.keys()).cloned().collect();
    ids.sort();
    ids.dedup();

    let mut items = Vec::new();
    for id in &ids {
        let sys = system.get(id);
        let usr = user.get(id);
        let effective = usr.or(sys).expect("id present");
        let user_added = usr.is_some() && sys.is_none();
        let source = if user_added { "user" } else { "system" };
        let enabled = is_enabled(effective);
        let name = if effective.name.is_empty() {
            sys.map(|d| d.name.clone()).unwrap_or_else(|| id.clone())
        } else {
            effective.name.clone()
        };
        let description = if effective.comment.is_empty() {
            sys.map(|d| d.comment.clone()).unwrap_or_default()
        } else {
            effective.comment.clone()
        };
        let icon = if effective.icon.is_empty() {
            sys.map(|d| d.icon.clone()).unwrap_or_default()
        } else {
            effective.icon.clone()
        };
        let exec = if effective.exec.is_empty() {
            sys.map(|d| d.exec.clone()).unwrap_or_default()
        } else {
            effective.exec.clone()
        };
        items.push(AutostartItem {
            id: id.clone(),
            name,
            description,
            exec,
            icon,
            enabled,
            source: source.into(),
            path: effective.path.display().to_string(),
            user_added,
            locked: LOCKED.contains(&id.as_str()),
        });
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let taken: std::collections::HashSet<_> = ids.iter().cloned().collect();
    let mut available = Vec::new();
    for dir in app_dirs() {
        for desk in load_dir(&dir).into_values() {
            if taken.contains(&desk.id) {
                continue;
            }
            if !desk.type_application || desk.no_display || desk.hidden {
                continue;
            }
            if desk.exec.is_empty() {
                continue;
            }
            let name = if desk.name.is_empty() {
                desk.id.clone()
            } else {
                desk.name
            };
            available.push(AvailableApp {
                id: desk.id,
                name,
                description: desk.comment,
                icon: desk.icon,
            });
        }
    }
    available.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    available.dedup_by(|a, b| a.id == b.id);

    Ok(Report { items, available })
}

fn is_enabled(d: &Desktop) -> bool {
    if d.hidden {
        return false;
    }
    d.gnome_enabled != Some(false)
}

fn set_enabled(id: &str, on: bool) -> Result<()> {
    if LOCKED.contains(&id) && !on {
        bail!("{id} is required for the session and cannot be disabled");
    }
    let user = user_dir()?;
    fs::create_dir_all(&user)?;
    let user_path = user.join(format!("{id}.desktop"));
    let sys_path = system_dir().join(format!("{id}.desktop"));
    let sys = if sys_path.is_file() {
        parse_desktop(&sys_path, id)
    } else {
        None
    };
    let usr = if user_path.is_file() {
        parse_desktop(&user_path, id)
    } else {
        None
    };

    if on {
        if let Some(u) = &usr {
            if u.stub && sys.is_some() {
                fs::remove_file(&user_path)
                    .with_context(|| format!("remove {}", user_path.display()))?;
                return Ok(());
            }
            write_desktop(&user_path, &clear_hidden(fs::read_to_string(&user_path)?))?;
            return Ok(());
        }
        if sys.is_some() {
            return Ok(());
        }
        add_app(id)
    } else {
        if usr.as_ref().map(|d| d.user_full()).unwrap_or(false) && sys.is_none() {
            write_desktop(&user_path, &set_hidden(fs::read_to_string(&user_path)?))?;
            return Ok(());
        }
        fs::write(&user_path, "[Desktop Entry]\nHidden=true\n")
            .with_context(|| format!("write {}", user_path.display()))?;
        Ok(())
    }
}

fn add_app(id: &str) -> Result<()> {
    let id = strip_desktop(id);
    let src = find_app_desktop(&id).with_context(|| format!("no desktop file for {id}"))?;
    let dest_dir = user_dir()?;
    fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(format!("{id}.desktop"));
    let body = clear_hidden(fs::read_to_string(&src)?);
    write_desktop(&dest, &body)?;
    Ok(())
}

fn remove_app(id: &str) -> Result<()> {
    let id = strip_desktop(id);
    let user_path = user_dir()?.join(format!("{id}.desktop"));
    let sys_path = system_dir().join(format!("{id}.desktop"));
    if !user_path.is_file() {
        bail!("{id} is not a user autostart entry");
    }
    if sys_path.is_file() {
        fs::write(&user_path, "[Desktop Entry]\nHidden=true\n")
            .with_context(|| format!("write {}", user_path.display()))?;
        return Ok(());
    }
    fs::remove_file(&user_path).with_context(|| format!("remove {}", user_path.display()))?;
    Ok(())
}

impl Desktop {
    fn user_full(&self) -> bool {
        !self.stub && !self.exec.is_empty()
    }
}

fn load_dir(dir: &Path) -> BTreeMap<String, Desktop> {
    let mut out = BTreeMap::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for ent in entries.flatten() {
        let path = ent.path();
        if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if id.is_empty() {
            continue;
        }
        if let Some(d) = parse_desktop(&path, &id) {
            out.insert(id, d);
        }
    }
    out
}

fn parse_desktop(path: &Path, id: &str) -> Option<Desktop> {
    let text = fs::read_to_string(path).ok()?;
    let loc = locale_tags();
    let mut name = String::new();
    let mut name_generic = String::new();
    let mut comment = String::new();
    let mut comment_generic = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut hidden = false;
    let mut no_display = false;
    let mut gnome_enabled = None;
    let mut type_application = false;
    let mut in_entry = false;
    let mut keys = 0u32;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        keys += 1;
        let key = k.trim();
        let val = v.trim();
        match key {
            "Type" => type_application = val.eq_ignore_ascii_case("Application") || val.is_empty(),
            "Exec" => exec = val.to_string(),
            "Icon" => icon = val.to_string(),
            "Hidden" => hidden = is_true(val),
            "NoDisplay" => no_display = is_true(val),
            "X-GNOME-Autostart-enabled" => gnome_enabled = Some(is_true(val)),
            "Name" => name_generic = unquote(val),
            "Comment" => comment_generic = unquote(val),
            _ => {
                if let Some(tag) = localized_tag(key, "Name") {
                    if loc.iter().any(|t| t == &tag) && name.is_empty() {
                        name = unquote(val);
                    }
                }
                if let Some(tag) = localized_tag(key, "Comment") {
                    if loc.iter().any(|t| t == &tag) && comment.is_empty() {
                        comment = unquote(val);
                    }
                }
            }
        }
    }
    if name.is_empty() {
        name = name_generic;
    }
    if comment.is_empty() {
        comment = comment_generic;
    }
    let stub = hidden && exec.is_empty() && keys <= 3;
    let type_application = type_application || !exec.is_empty();
    Some(Desktop {
        id: id.to_string(),
        path: path.to_path_buf(),
        name,
        comment,
        exec,
        icon,
        hidden,
        no_display,
        gnome_enabled,
        type_application,
        stub,
    })
}

fn localized_tag(key: &str, base: &str) -> Option<String> {
    let prefix = format!("{base}[");
    if let Some(rest) = key.strip_prefix(&prefix) {
        return rest.strip_suffix(']').map(|s| s.to_string());
    }
    None
}

fn locale_tags() -> Vec<String> {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let base = raw.split('.').next().unwrap_or("").replace('_', "-");
    let mut out = Vec::new();
    if !base.is_empty() && base != "C" {
        out.push(base.replace('-', "_"));
        if let Some((lang, _)) = base.split_once(['-', '_']) {
            out.push(lang.to_string());
        }
        let lang = base.split(['-', '_']).next().unwrap_or("");
        if lang == "zh" {
            out.push("zh_CN".into());
            out.push("zh".into());
        }
    }
    out
}

fn is_true(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

fn unquote(v: &str) -> String {
    v.trim_matches('"').to_string()
}

fn strip_desktop(id: &str) -> String {
    id.trim()
        .trim_end_matches(".desktop")
        .trim()
        .to_string()
}

fn system_dir() -> PathBuf {
    PathBuf::from("/etc/xdg/autostart")
}

fn user_dir() -> Result<PathBuf> {
    Ok(util::home_dir()?.join(".config/autostart"))
}

fn app_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Ok(home) = util::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }
    dirs
}

fn find_app_desktop(id: &str) -> Option<PathBuf> {
    let name = format!("{id}.desktop");
    for dir in app_dirs() {
        let p = dir.join(&name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn set_hidden(text: String) -> String {
    let mut out = String::new();
    let mut in_entry = false;
    let mut wrote = false;
    if !text.contains("[Desktop Entry]") {
        return "[Desktop Entry]\nHidden=true\n".into();
    }
    for line in text.lines() {
        let trim = line.trim();
        if trim.starts_with('[') {
            if in_entry && !wrote {
                out.push_str("Hidden=true\n");
                wrote = true;
            }
            in_entry = trim.eq_ignore_ascii_case("[Desktop Entry]");
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_entry && trim.to_ascii_lowercase().starts_with("hidden=") {
            if !wrote {
                out.push_str("Hidden=true\n");
                wrote = true;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_entry && !wrote {
        out.push_str("Hidden=true\n");
    }
    out
}

fn clear_hidden(text: String) -> String {
    let mut out = String::new();
    let mut in_entry = false;
    for line in text.lines() {
        let trim = line.trim();
        if trim.starts_with('[') {
            in_entry = trim.eq_ignore_ascii_case("[Desktop Entry]");
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_entry && trim.to_ascii_lowercase().starts_with("hidden=") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn write_desktop(path: &Path, body: &str) -> Result<()> {
    fs::write(path, body).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_stub_is_disabled() {
        let dir = std::env::temp_dir().join("omakeeper-autostart-test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("print-applet.desktop");
        fs::write(&path, "[Desktop Entry]\nHidden=true\n").unwrap();
        let d = parse_desktop(&path, "print-applet").unwrap();
        assert!(d.hidden);
        assert!(d.stub);
        assert!(!is_enabled(&d));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn set_hidden_keeps_exec() {
        let src = "[Desktop Entry]\nName=Foo\nExec=foo\nHidden=false\n";
        let out = set_hidden(src.into());
        assert!(out.contains("Hidden=true"));
        assert!(out.contains("Exec=foo"));
        assert!(!out.contains("Hidden=false"));
    }
}
