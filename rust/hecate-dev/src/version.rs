use anyhow::{Context, Result};
use clap::ValueEnum;
use colored::*;
use semver::Version;
use std::fs;
use std::path::Path;
use toml_edit::{Document, Item};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
    Prerelease,
}

pub fn show_version() -> Result<()> {
    let version = read_version_file()?;
    let cargo_version = read_cargo_version()?;
    
    println!("{}: {}", "VERSION file".bold(), version.green());
    println!("{}: {}", "Cargo.toml".bold(), cargo_version.green());
    
    if version != cargo_version {
        println!("{}: Versions are out of sync!", "Warning".yellow().bold());
    }
    
    Ok(())
}

pub fn bump_version(level: BumpLevel, dry_run: bool) -> Result<()> {
    let current = read_version_file()?;
    let mut version = Version::parse(&current)?;
    
    match level {
        BumpLevel::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
            version.pre = semver::Prerelease::EMPTY;
        }
        BumpLevel::Minor => {
            version.minor += 1;
            version.patch = 0;
            version.pre = semver::Prerelease::EMPTY;
        }
        BumpLevel::Patch => {
            version.patch += 1;
            version.pre = semver::Prerelease::EMPTY;
        }
        BumpLevel::Prerelease => {
            if version.pre.is_empty() {
                version.pre = semver::Prerelease::new("alpha.1")?;
            } else {
                // Increment prerelease version
                let pre_str = version.pre.as_str();
                if let Some(pos) = pre_str.rfind('.') {
                    let (prefix, num_str) = pre_str.split_at(pos);
                    if let Ok(num) = num_str[1..].parse::<u32>() {
                        version.pre = semver::Prerelease::new(&format!("{}.{}", prefix, num + 1))?;
                    }
                }
            }
        }
    }
    
    let new_version = version.to_string();
    
    if dry_run {
        println!("{}: {} → {}", 
            "Would bump version".yellow(), 
            current.red(), 
            new_version.green()
        );
    } else {
        println!("{}: {} → {}", 
            "Bumping version".green().bold(), 
            current.red(), 
            new_version.green()
        );
        update_version_everywhere(&new_version)?;
        println!("{}: Version bumped successfully", "Success".green().bold());
    }
    
    Ok(())
}

pub fn sync_version(version: Option<&str>) -> Result<()> {
    let target_version = match version {
        Some(v) => v.to_string(),
        None => read_version_file()?,
    };
    
    // Validate version format
    Version::parse(&target_version)?;
    
    println!("{}: Syncing to version {}", 
        "Syncing".green().bold(), 
        target_version.green()
    );
    
    update_version_everywhere(&target_version)?;
    
    println!("{}: All versions synced successfully", "Success".green().bold());
    Ok(())
}

pub fn check_version_sync() -> Result<()> {
    let version_file = read_version_file()?;
    let mut all_match = true;
    let mut versions = vec![("VERSION file", version_file.clone())];
    
    // Check workspace Cargo.toml
    let cargo_version = read_cargo_version()?;
    versions.push(("Cargo.toml (workspace)", cargo_version.clone()));
    if cargo_version != version_file {
        all_match = false;
    }
    
    // Check all member Cargo.toml files
    let members = get_workspace_members()?;
    for member in members {
        let member_path = format!("rust/{}/Cargo.toml", member);
        if let Ok(member_version) = read_specific_cargo_version(&member_path) {
            versions.push((Box::leak(member.into_boxed_str()), member_version.clone()));
            if member_version != version_file {
                all_match = false;
            }
        }
    }
    
    // Display results
    println!("{}", "Version Check Results:".bold());
    for (name, version) in versions {
        let status = if version == version_file {
            "✓".green()
        } else {
            "✗".red()
        };
        println!("  {} {}: {}", status, name, version);
    }
    
    if all_match {
        println!("\n{}: All versions are in sync", "Success".green().bold());
        Ok(())
    } else {
        println!("\n{}: Version mismatch detected", "Error".red().bold());
        println!("Run 'hecate-dev version sync' to fix");
        anyhow::bail!("Version sync check failed");
    }
}

pub fn read_version_file() -> Result<String> {
    fs::read_to_string("VERSION")
        .context("Failed to read VERSION file")?
        .trim()
        .to_string()
        .parse()
        .context("Invalid version in VERSION file")
}

fn read_cargo_version() -> Result<String> {
    read_specific_cargo_version("rust/Cargo.toml")
}

fn read_specific_cargo_version(path: &str) -> Result<String> {
    let content = fs::read_to_string(path)?;
    let doc = content.parse::<Document>()?;
    
    if let Some(workspace) = doc.get("workspace") {
        if let Some(package) = workspace.get("package") {
            if let Some(version) = package.get("version") {
                if let Some(v) = version.as_str() {
                    return Ok(v.to_string());
                }
            }
        }
    }
    
    if let Some(package) = doc.get("package") {
        if let Some(version) = package.get("version") {
            if let Some(v) = version.as_str() {
                return Ok(v.to_string());
            }
        }
    }
    
    anyhow::bail!("Could not find version in {}", path)
}

fn get_workspace_members() -> Result<Vec<String>> {
    let content = fs::read_to_string("rust/Cargo.toml")?;
    let doc = content.parse::<Document>()?;
    
    if let Some(workspace) = doc.get("workspace") {
        if let Some(members) = workspace.get("members") {
            if let Some(array) = members.as_array() {
                return Ok(array
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect());
            }
        }
    }
    
    Ok(Vec::new())
}

fn update_version_everywhere(version: &str) -> Result<()> {
    // Update VERSION file
    fs::write("VERSION", format!("{}\n", version))?;
    
    // Update workspace Cargo.toml
    update_cargo_version("rust/Cargo.toml", version)?;
    
    // Update all member Cargo.toml files
    let members = get_workspace_members()?;
    for member in members {
        let member_path = format!("rust/{}/Cargo.toml", member);
        if Path::new(&member_path).exists() {
            update_cargo_version(&member_path, version)?;
        }
    }
    
    // Update dashboard package.json if it exists
    if Path::new("hecate-dashboard/package.json").exists() {
        update_package_json_version("hecate-dashboard/package.json", version)?;
    }
    
    Ok(())
}

fn update_cargo_version(path: &str, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut doc = content.parse::<Document>()?;
    
    // Update workspace.package.version if it exists
    if let Some(workspace) = doc.get_mut("workspace") {
        if let Some(package) = workspace.get_mut("package") {
            if let Item::Table(table) = package {
                table["version"] = toml_edit::value(version);
            }
        }
    }
    
    // Update package.version if it exists
    if let Some(package) = doc.get_mut("package") {
        if let Item::Table(table) = package {
            table["version"] = toml_edit::value(version);
        }
    }
    
    fs::write(path, doc.to_string())?;
    Ok(())
}

fn update_package_json_version(path: &str, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut package: serde_json::Value = serde_json::from_str(&content)?;
    
    if let Some(obj) = package.as_object_mut() {
        obj.insert("version".to_string(), serde_json::Value::String(version.to_string()));
    }
    
    let updated = serde_json::to_string_pretty(&package)?;
    fs::write(path, updated)?;
    Ok(())
}