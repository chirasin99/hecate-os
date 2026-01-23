use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod version;
mod commit;
mod check;
mod release;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage project versions
    Version {
        #[command(subcommand)]
        action: VersionAction,
    },
    /// Validate and format commits
    Commit {
        #[command(subcommand)]
        action: CommitAction,
    },
    /// Check project structure and conventions
    Check {
        /// Check only specific aspects
        #[arg(short, long)]
        only: Option<Vec<String>>,
        
        /// Auto-fix issues when possible
        #[arg(short, long)]
        fix: bool,
    },
    /// Manage releases
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },
    /// Initialize git hooks
    InitHooks {
        /// Force overwrite existing hooks
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum VersionAction {
    /// Show current version
    Show,
    /// Bump version based on commit type
    Bump {
        /// Version part to bump (major, minor, patch)
        #[arg(value_enum)]
        level: version::BumpLevel,
        
        /// Dry run without making changes
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Sync version across all files
    Sync {
        /// Version to set
        version: Option<String>,
    },
    /// Check if versions are in sync
    Check,
}

#[derive(Subcommand)]
enum CommitAction {
    /// Validate commit message format
    Validate {
        /// Commit message or range
        message: Option<String>,
    },
    /// Create a properly formatted commit
    Create {
        /// Commit type (feat, fix, chore, etc.)
        #[arg(short = 't', long)]
        commit_type: String,
        
        /// Scope of the change
        #[arg(short, long)]
        scope: Option<String>,
        
        /// Commit message
        #[arg(short, long)]
        message: String,
        
        /// Breaking change
        #[arg(short, long)]
        breaking: bool,
    },
    /// Show commit conventions
    Conventions,
}

#[derive(Subcommand)]
enum ReleaseAction {
    /// Create a new release
    Create {
        /// Version for the release
        version: Option<String>,
        
        /// Skip tests
        #[arg(long)]
        skip_tests: bool,
        
        /// Skip changelog generation
        #[arg(long)]
        skip_changelog: bool,
    },
    /// Generate changelog
    Changelog {
        /// Version range (e.g., v0.1.0..HEAD)
        #[arg(short, long)]
        range: Option<String>,
        
        /// Output format (markdown, json)
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },
    /// Prepare release notes
    Notes {
        /// Version to generate notes for
        version: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    match cli.command {
        Commands::Version { action } => {
            handle_version_command(action).await?;
        }
        Commands::Commit { action } => {
            handle_commit_command(action).await?;
        }
        Commands::Check { only, fix } => {
            check::run_checks(only.as_deref(), fix).await?;
        }
        Commands::Release { action } => {
            handle_release_command(action).await?;
        }
        Commands::InitHooks { force } => {
            init_git_hooks(force).await?;
        }
    }

    Ok(())
}

async fn handle_version_command(action: VersionAction) -> Result<()> {
    match action {
        VersionAction::Show => {
            version::show_version()?;
        }
        VersionAction::Bump { level, dry_run } => {
            version::bump_version(level, dry_run)?;
        }
        VersionAction::Sync { version } => {
            version::sync_version(version.as_deref())?;
        }
        VersionAction::Check => {
            version::check_version_sync()?;
        }
    }
    Ok(())
}

async fn handle_commit_command(action: CommitAction) -> Result<()> {
    match action {
        CommitAction::Validate { message } => {
            commit::validate_commit(message.as_deref())?;
        }
        CommitAction::Create { 
            commit_type, 
            scope, 
            message, 
            breaking 
        } => {
            commit::create_commit(&commit_type, scope.as_deref(), &message, breaking)?;
        }
        CommitAction::Conventions => {
            commit::show_conventions();
        }
    }
    Ok(())
}

async fn handle_release_command(action: ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Create { 
            version, 
            skip_tests, 
            skip_changelog 
        } => {
            release::create_release(
                version.as_deref(), 
                skip_tests, 
                skip_changelog
            ).await?;
        }
        ReleaseAction::Changelog { range, format } => {
            release::generate_changelog(range.as_deref(), &format)?;
        }
        ReleaseAction::Notes { version } => {
            release::generate_release_notes(version.as_deref())?;
        }
    }
    Ok(())
}

async fn init_git_hooks(force: bool) -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    
    info!("Installing git hooks...");
    
    let hooks_dir = ".git/hooks";
    fs::create_dir_all(hooks_dir)?;
    
    // Pre-commit hook
    let pre_commit_path = format!("{}/pre-commit", hooks_dir);
    if !force && std::path::Path::new(&pre_commit_path).exists() {
        warn!("pre-commit hook already exists. Use --force to overwrite.");
    } else {
        let pre_commit_content = r#"#!/bin/sh
# HecateOS pre-commit hook

# Run hecate-dev checks
hecate-dev check --only structure,imports,licenses

# Validate commit message format
if [ -f .git/COMMIT_EDITMSG ]; then
    hecate-dev commit validate
fi

# Run tests
cargo test --quiet
"#;
        fs::write(&pre_commit_path, pre_commit_content)?;
        let mut perms = fs::metadata(&pre_commit_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre_commit_path, perms)?;
        info!("Installed pre-commit hook");
    }
    
    // Pre-push hook
    let pre_push_path = format!("{}/pre-push", hooks_dir);
    if !force && std::path::Path::new(&pre_push_path).exists() {
        warn!("pre-push hook already exists. Use --force to overwrite.");
    } else {
        let pre_push_content = r#"#!/bin/sh
# HecateOS pre-push hook

# Check version sync
hecate-dev version check

# Run full test suite
cargo test

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Uncommitted changes detected"
    exit 1
fi
"#;
        fs::write(&pre_push_path, pre_push_content)?;
        let mut perms = fs::metadata(&pre_push_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre_push_path, perms)?;
        info!("Installed pre-push hook");
    }
    
    info!("Git hooks installed successfully");
    Ok(())
}