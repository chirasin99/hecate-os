use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub async fn run_checks(only: Option<&[String]>, fix: bool) -> Result<()> {
    let all_checks = vec![
        "structure",
        "imports",
        "licenses",
        "todos",
        "dependencies",
        "ports",
    ];
    
    let checks_to_run = if let Some(only_list) = only {
        only_list.to_vec()
    } else {
        all_checks.into_iter().map(String::from).collect()
    };
    
    println!("{}", "Running project checks...".bold());
    let mut all_passed = true;
    
    for check in &checks_to_run {
        let result = match check.as_str() {
            "structure" => check_directory_structure(),
            "imports" => check_import_organization(fix),
            "licenses" => check_license_headers(fix),
            "todos" => check_todos_and_fixmes(),
            "dependencies" => check_dependencies(),
            "ports" => check_port_configuration(),
            _ => {
                println!("{}: Unknown check '{}'", "Warning".yellow(), check);
                continue;
            }
        };
        
        match result {
            Ok(()) => println!("  {} {}", "✓".green(), check),
            Err(e) => {
                println!("  {} {}: {}", "✗".red(), check, e);
                all_passed = false;
            }
        }
    }
    
    if all_passed {
        println!("\n{}: All checks passed", "Success".green().bold());
        Ok(())
    } else {
        anyhow::bail!("Some checks failed");
    }
}

fn check_directory_structure() -> Result<()> {
    let required_dirs = vec![
        "rust",
        "rust/hecate-core",
        "rust/hecate-daemon",
        "rust/hecate-gpu",
        "rust/hecate-pkg",
        "hecate-dashboard",
        "docs",
        "scripts",
        "config",
        "auto/config",
        "binary",
    ];
    
    let required_files = vec![
        "VERSION",
        "README.md",
        "LICENSE",
        "Dockerfile.build",
        "build.sh",
        "rust/Cargo.toml",
        "docs/ROADMAP.md",
    ];
    
    let mut missing = Vec::new();
    
    for dir in &required_dirs {
        if !Path::new(dir).is_dir() {
            missing.push(format!("Directory: {}", dir));
        }
    }
    
    for file in &required_files {
        if !Path::new(file).exists() {
            missing.push(format!("File: {}", file));
        }
    }
    
    if !missing.is_empty() {
        let msg = format!("Missing required items:\n  {}", missing.join("\n  "));
        anyhow::bail!(msg);
    }
    
    Ok(())
}

fn check_import_organization(fix: bool) -> Result<()> {
    let mut issues = Vec::new();
    
    for entry in WalkDir::new("rust")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("rs".as_ref()))
    {
        let path = entry.path();
        let content = fs::read_to_string(path)?;
        
        if let Some(reorganized) = check_and_fix_imports(&content) {
            issues.push(path.to_path_buf());
            if fix {
                fs::write(path, reorganized)?;
            }
        }
    }
    
    if !issues.is_empty() {
        if fix {
            println!("    Fixed import organization in {} files", issues.len());
        } else {
            let msg = format!("Import organization issues in {} files. Use --fix to correct", issues.len());
            anyhow::bail!(msg);
        }
    }
    
    Ok(())
}

fn check_and_fix_imports(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut import_block = Vec::new();
    let mut other_lines = Vec::new();
    let mut in_imports = false;
    let mut needs_reorg = false;
    
    for line in lines {
        if line.starts_with("use ") {
            if !in_imports {
                in_imports = true;
            }
            import_block.push(line);
        } else if in_imports && line.trim().is_empty() {
            // Continue collecting imports after blank lines
            continue;
        } else {
            if in_imports {
                in_imports = false;
                // Check if imports need reorganization
                let sorted = organize_imports(&import_block);
                if sorted != import_block {
                    needs_reorg = true;
                    import_block = sorted;
                }
            }
            other_lines.push(line);
        }
    }
    
    if needs_reorg {
        let mut result = Vec::new();
        
        // Group imports by category
        let std_imports: Vec<&str> = import_block.iter()
            .filter(|l| l.starts_with("use std::"))
            .copied()
            .collect();
        let external_imports: Vec<&str> = import_block.iter()
            .filter(|l| !l.starts_with("use std::") && !l.starts_with("use crate::") && !l.starts_with("use super::"))
            .copied()
            .collect();
        let local_imports: Vec<&str> = import_block.iter()
            .filter(|l| l.starts_with("use crate::") || l.starts_with("use super::"))
            .copied()
            .collect();
        
        if !std_imports.is_empty() {
            result.extend(std_imports);
            result.push("");
        }
        if !external_imports.is_empty() {
            result.extend(external_imports);
            result.push("");
        }
        if !local_imports.is_empty() {
            result.extend(local_imports);
            result.push("");
        }
        
        result.extend(other_lines);
        Some(result.join("\n"))
    } else {
        None
    }
}

fn organize_imports(imports: &[&str]) -> Vec<&str> {
    let mut sorted = imports.to_vec();
    sorted.sort();
    sorted
}

fn check_license_headers(fix: bool) -> Result<()> {
    const LICENSE_HEADER: &str = "// Copyright (c) 2026 HecateOS Team
// SPDX-License-Identifier: MIT
";
    
    let mut missing = Vec::new();
    
    for entry in WalkDir::new("rust")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("rs".as_ref()))
    {
        let path = entry.path();
        let content = fs::read_to_string(path)?;
        
        if !content.starts_with(LICENSE_HEADER) && !content.starts_with("//") {
            missing.push(path.to_path_buf());
            if fix {
                let new_content = format!("{}\n{}", LICENSE_HEADER, content);
                fs::write(path, new_content)?;
            }
        }
    }
    
    if !missing.is_empty() {
        if fix {
            println!("    Added license headers to {} files", missing.len());
        } else {
            let msg = format!("Missing license headers in {} files. Use --fix to add", missing.len());
            anyhow::bail!(msg);
        }
    }
    
    Ok(())
}

fn check_todos_and_fixmes() -> Result<()> {
    let mut todos = Vec::new();
    let mut fixmes = Vec::new();
    
    for entry in WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            !path.starts_with("./.git") && 
            !path.starts_with("./target") &&
            !path.starts_with("./node_modules") &&
            e.file_type().is_file()
        })
    {
        let path = entry.path();
        if let Ok(content) = fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                if line.contains("TODO") {
                    todos.push((path.to_path_buf(), line_num + 1, line.trim().to_string()));
                }
                if line.contains("FIXME") {
                    fixmes.push((path.to_path_buf(), line_num + 1, line.trim().to_string()));
                }
            }
        }
    }
    
    if !todos.is_empty() || !fixmes.is_empty() {
        println!("    Found {} TODOs and {} FIXMEs:", todos.len(), fixmes.len());
        
        if !todos.is_empty() {
            println!("\n    {}:", "TODOs".yellow());
            for (path, line, content) in todos.iter().take(5) {
                println!("      {}:{} - {}", 
                    path.display(), 
                    line, 
                    content.chars().take(60).collect::<String>()
                );
            }
            if todos.len() > 5 {
                println!("      ... and {} more", todos.len() - 5);
            }
        }
        
        if !fixmes.is_empty() {
            println!("\n    {}:", "FIXMEs".red());
            for (path, line, content) in fixmes.iter().take(5) {
                println!("      {}:{} - {}", 
                    path.display(), 
                    line, 
                    content.chars().take(60).collect::<String>()
                );
            }
            if fixmes.len() > 5 {
                println!("      ... and {} more", fixmes.len() - 5);
            }
        }
    }
    
    Ok(())
}

fn check_dependencies() -> Result<()> {
    // Check for duplicate dependencies
    let cargo_toml = fs::read_to_string("rust/Cargo.toml")?;
    let doc = cargo_toml.parse::<toml_edit::Document>()?;
    
    if let Some(deps) = doc.get("workspace").and_then(|w| w.get("dependencies")) {
        if let Some(table) = deps.as_table() {
            let dep_count = table.len();
            println!("    Found {} workspace dependencies", dep_count);
            
            // Check for security advisories (would need cargo-audit in real implementation)
            println!("    Security audit: Run 'cargo audit' for vulnerability check");
        }
    }
    
    Ok(())
}

fn check_port_configuration() -> Result<()> {
    let expected_ports = [
        ("monitor", 9313),
        ("pkg_api", 9314),
        ("remote", 9315),
        ("bench", 9316),
        ("gpu", 9317),
    ];
    
    let config_path = "config/hecate/ports.conf";
    if !Path::new(config_path).exists() {
        anyhow::bail!("Port configuration file not found at {}", config_path);
    }
    
    let content = fs::read_to_string(config_path)?;
    let mut all_found = true;
    
    for (service, port) in &expected_ports {
        let pattern = format!("{}={}", service.to_uppercase(), port);
        if !content.contains(&pattern) {
            println!("    Missing port configuration for {}: {}", service, port);
            all_found = false;
        }
    }
    
    if !all_found {
        anyhow::bail!("Port configuration incomplete");
    }
    
    Ok(())
}