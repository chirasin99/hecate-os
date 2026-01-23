use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use regex::Regex;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about = "HecateOS git hooks manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install git hooks
    Install {
        /// Force overwrite existing hooks
        #[arg(short, long)]
        force: bool,
    },
    /// Uninstall git hooks
    Uninstall,
    /// Run pre-commit checks
    PreCommit,
    /// Run pre-push checks
    PrePush,
    /// Validate commit message
    CommitMsg {
        /// Path to commit message file
        file: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Install { force } => install_hooks(force)?,
        Commands::Uninstall => uninstall_hooks()?,
        Commands::PreCommit => run_pre_commit()?,
        Commands::PrePush => run_pre_push()?,
        Commands::CommitMsg { file } => validate_commit_msg(&file)?,
    }
    
    Ok(())
}

fn install_hooks(force: bool) -> Result<()> {
    println!("{} Installing git hooks...", "→".blue());
    
    let hooks_dir = ".git/hooks";
    fs::create_dir_all(hooks_dir)?;
    
    // Pre-commit hook
    let pre_commit = format!("{}/pre-commit", hooks_dir);
    if !force && std::path::Path::new(&pre_commit).exists() {
        println!("  {} pre-commit hook already exists (use --force to overwrite)", "⚠".yellow());
    } else {
        fs::write(&pre_commit, PRE_COMMIT_HOOK)?;
        set_executable(&pre_commit)?;
        println!("  {} Installed pre-commit hook", "✓".green());
    }
    
    // Commit-msg hook
    let commit_msg = format!("{}/commit-msg", hooks_dir);
    if !force && std::path::Path::new(&commit_msg).exists() {
        println!("  {} commit-msg hook already exists (use --force to overwrite)", "⚠".yellow());
    } else {
        fs::write(&commit_msg, COMMIT_MSG_HOOK)?;
        set_executable(&commit_msg)?;
        println!("  {} Installed commit-msg hook", "✓".green());
    }
    
    // Pre-push hook
    let pre_push = format!("{}/pre-push", hooks_dir);
    if !force && std::path::Path::new(&pre_push).exists() {
        println!("  {} pre-push hook already exists (use --force to overwrite)", "⚠".yellow());
    } else {
        fs::write(&pre_push, PRE_PUSH_HOOK)?;
        set_executable(&pre_push)?;
        println!("  {} Installed pre-push hook", "✓".green());
    }
    
    println!("{} Git hooks installed successfully!", "✓".green().bold());
    Ok(())
}

fn uninstall_hooks() -> Result<()> {
    println!("{} Uninstalling git hooks...", "→".blue());
    
    let hooks = vec!["pre-commit", "commit-msg", "pre-push"];
    for hook in hooks {
        let path = format!(".git/hooks/{}", hook);
        if std::path::Path::new(&path).exists() {
            fs::remove_file(&path)?;
            println!("  {} Removed {} hook", "✓".green(), hook);
        }
    }
    
    println!("{} Git hooks uninstalled", "✓".green().bold());
    Ok(())
}

fn run_pre_commit() -> Result<()> {
    println!("{} Running pre-commit checks...", "→".blue());
    
    // Check for merge conflicts
    let output = Command::new("git")
        .args(&["diff", "--cached", "--name-only"])
        .output()?;
    
    let files = String::from_utf8_lossy(&output.stdout);
    for file in files.lines() {
        if let Ok(content) = fs::read_to_string(file) {
            if content.contains("<<<<<<<") || content.contains(">>>>>>>") {
                println!("{} Merge conflict markers found in {}", "✗".red(), file);
                std::process::exit(1);
            }
        }
    }
    println!("  {} No merge conflicts", "✓".green());
    
    // Check file sizes
    for file in files.lines() {
        if let Ok(metadata) = fs::metadata(file) {
            if metadata.len() > 10_000_000 {
                println!("{} File {} is too large (>10MB)", "✗".red(), file);
                std::process::exit(1);
            }
        }
    }
    println!("  {} File sizes OK", "✓".green());
    
    println!("{} Pre-commit checks passed", "✓".green().bold());
    Ok(())
}

fn run_pre_push() -> Result<()> {
    println!("{} Running pre-push checks...", "→".blue());
    
    // Check for uncommitted changes
    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()?;
    
    if !output.stdout.is_empty() {
        println!("{} Uncommitted changes detected", "✗".red());
        println!("Please commit or stash your changes before pushing");
        std::process::exit(1);
    }
    println!("  {} Working directory clean", "✓".green());
    
    // Run tests
    println!("  Running tests...");
    let test_output = Command::new("cargo")
        .args(&["test", "--quiet"])
        .status()?;
    
    if !test_output.success() {
        println!("{} Tests failed", "✗".red());
        std::process::exit(1);
    }
    println!("  {} Tests passed", "✓".green());
    
    println!("{} Pre-push checks passed", "✓".green().bold());
    Ok(())
}

fn validate_commit_msg(file: &str) -> Result<()> {
    let content = fs::read_to_string(file)?;
    let first_line = content.lines().next().unwrap_or("");
    
    // Skip merge commits
    if first_line.starts_with("Merge") {
        return Ok(());
    }
    
    let re = Regex::new(
        r"^(feat|fix|docs|style|refactor|perf|test|chore|build|ci|revert)(\([a-z0-9-]+\))?: .{1,72}$"
    )?;
    
    if !re.is_match(first_line) {
        println!("{} Invalid commit message format", "✗".red().bold());
        println!("\nExpected: <type>(<scope>): <subject>");
        println!("Example: feat(rust): add version management");
        println!("\nValid types: feat, fix, docs, style, refactor, perf, test, chore, build, ci, revert");
        std::process::exit(1);
    }
    
    Ok(())
}

fn set_executable(path: &str) -> Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

const PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
hecate-hooks pre-commit
"#;

const COMMIT_MSG_HOOK: &str = r#"#!/bin/sh
hecate-hooks commit-msg "$1"
"#;

const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
hecate-hooks pre-push
"#;