use anyhow::{Context, Result};
use colored::*;
use regex::Regex;
use std::process::Command;

const VALID_TYPES: &[&str] = &[
    "feat",     // New feature
    "fix",      // Bug fix
    "docs",     // Documentation only changes
    "style",    // Changes that do not affect the meaning of the code
    "refactor", // Code change that neither fixes a bug nor adds a feature
    "perf",     // Code change that improves performance
    "test",     // Adding missing tests or correcting existing tests
    "chore",    // Changes to the build process or auxiliary tools
    "build",    // Changes that affect the build system or external dependencies
    "ci",       // Changes to CI configuration files and scripts
    "revert",   // Reverts a previous commit
];

pub fn validate_commit(message: Option<&str>) -> Result<()> {
    let message = match message {
        Some(m) => m.to_string(),
        None => {
            // Read from .git/COMMIT_EDITMSG or get latest commit
            if std::path::Path::new(".git/COMMIT_EDITMSG").exists() {
                std::fs::read_to_string(".git/COMMIT_EDITMSG")?
            } else {
                get_latest_commit_message()?
            }
        }
    };
    
    let re = Regex::new(
        r"^(feat|fix|docs|style|refactor|perf|test|chore|build|ci|revert)(\([a-z0-9-]+\))?: .{1,100}"
    )?;
    
    let first_line = message.lines().next().unwrap_or("");
    
    if !re.is_match(first_line) {
        println!("{}: Invalid commit message format", "Error".red().bold());
        println!("\n{}: {}", "Message".bold(), first_line);
        println!("\n{}", "Expected format:".bold());
        println!("  <type>(<scope>): <subject>");
        println!("\n{}", "Example:".bold());
        println!("  feat(rust): add semantic version enforcement");
        println!("\n{}", "Valid types:".bold());
        for commit_type in VALID_TYPES {
            println!("  - {}", commit_type);
        }
        anyhow::bail!("Commit message validation failed");
    }
    
    // Check for breaking changes
    if message.contains("BREAKING CHANGE:") {
        println!("{}: Breaking change detected", "Warning".yellow().bold());
        println!("Make sure to bump major version before release");
    }
    
    println!("{}: Commit message is valid", "Success".green().bold());
    Ok(())
}

pub fn create_commit(
    commit_type: &str, 
    scope: Option<&str>, 
    message: &str, 
    breaking: bool
) -> Result<()> {
    // Validate commit type
    if !VALID_TYPES.contains(&commit_type) {
        println!("{}: Invalid commit type '{}'", "Error".red().bold(), commit_type);
        println!("\n{}", "Valid types:".bold());
        for t in VALID_TYPES {
            println!("  - {}", t);
        }
        anyhow::bail!("Invalid commit type");
    }
    
    // Build commit message
    let mut commit_msg = if let Some(s) = scope {
        format!("{}({}): {}", commit_type, s, message)
    } else {
        format!("{}: {}", commit_type, message)
    };
    
    if breaking {
        commit_msg.push_str("\n\nBREAKING CHANGE: This commit contains breaking changes");
    }
    
    // Stage all changes
    Command::new("git")
        .args(&["add", "-A"])
        .status()
        .context("Failed to stage changes")?;
    
    // Create commit
    let output = Command::new("git")
        .args(&["commit", "-m", &commit_msg])
        .output()
        .context("Failed to create commit")?;
    
    if output.status.success() {
        println!("{}: Commit created successfully", "Success".green().bold());
        println!("\n{}", String::from_utf8_lossy(&output.stdout));
        
        if breaking {
            println!("\n{}: Remember to bump major version before release", 
                "Reminder".yellow().bold());
        }
    } else {
        println!("{}: Failed to create commit", "Error".red().bold());
        println!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Commit creation failed");
    }
    
    Ok(())
}

pub fn show_conventions() {
    println!("{}", "HecateOS Commit Conventions".bold().underline());
    println!("\n{}", "Format:".bold());
    println!("  <type>(<scope>): <subject>");
    println!("  <blank line>");
    println!("  <body>");
    println!("  <blank line>");
    println!("  <footer>");
    
    println!("\n{}", "Types:".bold());
    println!("  {} - A new feature", "feat".green());
    println!("  {} - A bug fix", "fix".green());
    println!("  {} - Documentation only changes", "docs".green());
    println!("  {} - Formatting, white-space, etc", "style".green());
    println!("  {} - Code refactoring", "refactor".green());
    println!("  {} - Performance improvements", "perf".green());
    println!("  {} - Adding or correcting tests", "test".green());
    println!("  {} - Build process or auxiliary tool changes", "chore".green());
    println!("  {} - Changes to build system", "build".green());
    println!("  {} - CI configuration changes", "ci".green());
    println!("  {} - Reverts a previous commit", "revert".green());
    
    println!("\n{}", "Scope:".bold());
    println!("  Optional, can be any of:");
    println!("  - rust (Rust components)");
    println!("  - dashboard (Web dashboard)");
    println!("  - iso (ISO build system)");
    println!("  - docs (Documentation)");
    println!("  - deps (Dependencies)");
    
    println!("\n{}", "Examples:".bold());
    println!("  feat(rust): add GPU temperature monitoring");
    println!("  fix(dashboard): correct WebSocket reconnection logic");
    println!("  docs: update installation instructions");
    println!("  perf(rust): optimize memory allocation in monitor");
    
    println!("\n{}", "Breaking Changes:".bold());
    println!("  Add 'BREAKING CHANGE:' in the footer to indicate breaking changes");
    println!("  This will trigger a major version bump recommendation");
    
    println!("\n{}", "Version Impact:".bold());
    println!("  {} → major version bump", "BREAKING CHANGE".red());
    println!("  {} → minor version bump", "feat".yellow());
    println!("  {} → patch version bump", "fix, docs, style, refactor, perf, test, chore".blue());
}

fn get_latest_commit_message() -> Result<String> {
    let output = Command::new("git")
        .args(&["log", "-1", "--pretty=%B"])
        .output()
        .context("Failed to get latest commit")?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        anyhow::bail!("Failed to get latest commit message")
    }
}