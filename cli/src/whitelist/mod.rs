use crate::util;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    paths: Vec<String>,
}

fn store_path() -> Result<PathBuf> {
    Ok(util::config_dir()?.join("whitelist.json"))
}

fn load() -> Result<Store> {
    let path = store_path()?;
    if !path.exists() {
        return Ok(Store::default());
    }
    let text = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn save(store: &Store) -> Result<()> {
    let path = store_path()?;
    let text = serde_json::to_string_pretty(store)?;
    fs::write(&path, text)?;
    Ok(())
}

pub fn list(json: bool) -> Result<()> {
    let store = load()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&store.paths)?);
        return Ok(());
    }
    if store.paths.is_empty() {
        println!("Whitelist is empty. Add with: omakeeper whitelist add <path>");
        return Ok(());
    }
    println!("Protected paths (skipped by clean):\n");
    for p in &store.paths {
        println!("  {p}");
    }
    Ok(())
}

pub fn add(path: &str) -> Result<()> {
    let abs = util::expand_user(path)
        .canonicalize()
        .with_context(|| format!("path not found: {path}"))?;
    let mut store = load()?;
    let s = abs.display().to_string();
    if !store.paths.iter().any(|p| p == &s) {
        store.paths.push(s.clone());
        store.paths.sort();
        save(&store)?;
    }
    println!("Whitelisted: {s}");
    Ok(())
}

pub fn remove(path: &str) -> Result<()> {
    let target = util::expand_user(path);
    let mut store = load()?;
    let before = store.paths.len();
    store.paths.retain(|p| {
        let existing = PathBuf::from(p);
        existing != target && existing.canonicalize().ok().as_ref() != Some(&target)
    });
    if store.paths.len() == before {
        anyhow::bail!("not in whitelist: {path}");
    }
    save(&store)?;
    println!("Removed from whitelist: {path}");
    Ok(())
}

pub fn is_protected(path: &Path) -> bool {
    let Ok(store) = load() else {
        return false;
    };
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    store.paths.iter().any(|p| {
        let protected = PathBuf::from(p);
        util::is_under(&canon, &protected) || util::is_under(&protected, &canon)
    })
}
