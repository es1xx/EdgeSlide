//! One-shot native gesture integration. Never runs as a service.
use anyhow::{bail, Context, Result};
use edgeslide_core::{lua_string, MultiConfig};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
fn directory() -> Result<PathBuf> {
    Ok(MultiConfig::path()?.parent().unwrap().to_path_buf())
}
fn atomic(path: &Path, contents: &str) -> Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    let result = (|| -> Result<()> {
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
fn generate(clear: bool) -> Result<PathBuf> {
    let dir = directory()?;
    std::fs::create_dir_all(&dir)?;
    let mappings = if clear {
        "return {}\n".into()
    } else {
        MultiConfig::load()?.lua()?
    };
    atomic(&dir.join("native-mappings.lua"), &mappings)?;
    atomic(
        &dir.join("native-engine.lua"),
        include_str!("../../../data/multifinger.lua"),
    )?;
    let loader=format!("-- EdgeSlide-owned additive native integration.\nlocal specs=assert(loadfile({}))()\nassert(loadfile({}))(specs)\n",lua_string(&dir.join("native-mappings.lua").to_string_lossy()),lua_string(&dir.join("native-engine.lua").to_string_lossy()));
    let path = dir.join("native-gestures.lua");
    atomic(&path, &loader)?;
    Ok(path)
}
fn source_line(path: &Path) -> String {
    format!("do local p={}; local f=io.open(p,'r'); if f then f:close(); dofile(p) end end -- EdgeSlide native gestures\n",lua_string(&path.to_string_lossy()))
}
fn apply(clear: bool) -> Result<()> {
    let loader = generate(clear)?;
    let output = Command::new("hyprctl")
        .args([
            "eval",
            &format!("dofile({})", lua_string(&loader.to_string_lossy())),
        ])
        .output()
        .context("Cannot run hyprctl")?;
    let response = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() || response.trim() != "ok" {
        bail!("Native gesture application reported an error. Existing foreign gestures are not removed.\n{response}");
    }
    println!("Native gestures applied. No daemon restart or compositor reload requested.");
    Ok(())
}
fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("apply") => apply(false),
        Some("clear") => apply(true),
        Some("generate") => {
            println!("{}", generate(false)?.display());
            Ok(())
        }
        Some("source-line") => {
            print!("{}", source_line(&directory()?.join("native-gestures.lua")));
            Ok(())
        }
        Some("install-source") => {
            let path = PathBuf::from(
                std::env::args()
                    .nth(2)
                    .context("Supply the exact Hyprland Lua configuration path")?,
            );
            if path.extension().is_none_or(|extension| extension != "lua") {
                bail!("Only an existing Hyprland .lua configuration is supported");
            }
            let previous = std::fs::read_to_string(&path)?;
            if previous.contains("-- EdgeSlide native gestures") {
                println!("Source line already present; no change.");
                return Ok(());
            }
            let loader = generate(false)?;
            // Copy the original before appending; no existing line is modified.
            let backup =
                path.with_extension(format!("lua.edgeslide-backup-{}", std::process::id()));
            let mut backup_file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&backup)?;
            use std::io::Write;
            backup_file.write_all(previous.as_bytes())?;
            let mut file = std::fs::OpenOptions::new().append(true).open(&path)?;
            writeln!(file)?;
            file.write_all(source_line(&loader).as_bytes())?;
            println!("Appended one guarded source line. Backup: {}. Hyprland may automatically reload its configuration.",backup.display());
            Ok(())
        }
        _ => {
            bail!("Usage: edgeslide-gestures apply|clear|generate|source-line|install-source PATH")
        }
    }
}
