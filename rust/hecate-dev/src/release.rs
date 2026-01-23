use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use regex::Regex;
use semver::Version;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

pub async fn create_release(
    version: Option<&str>,
    skip_tests: bool,
    skip_changelog: bool,
) -> Result<()> {
    println!("{}", "Creating new release...".bold());
    
    // Determine version
    let target_version = if let Some(v) = version {
        v.to_string()
    } else {
        // Auto-determine version based on commits since last tag
        determine_next_version()?
    };
    
    // Validate version
    Version::parse(&target_version)?;
    
    println!("  Target version: {}", target_version.green());
    
    // Run tests unless skipped
    if !skip_tests {
        println!("  Running tests...");
        run_tests()?;
        println!("  {} Tests passed", "✓".green());
    }
    
    // Update version files
    println!("  Updating version files...");
    crate::version::sync_version(Some(&target_version))?;
    println!("  {} Version files updated", "✓".green());
    
    // Generate changelog unless skipped
    if !skip_changelog {
        println!("  Generating changelog...");
        generate_changelog_file(&target_version)?;
        println!("  {} Changelog generated", "✓".green());
    }
    
    // Create git tag
    println!("  Creating git tag...");
    create_git_tag(&target_version)?;
    println!("  {} Tag created: v{}", "✓".green(), target_version);
    
    // Generate release notes
    println!("  Generating release notes...");
    let notes = generate_release_notes_content(&target_version)?;
    
    // Save release notes
    let notes_path = format!("docs/releases/v{}.md", target_version);
    fs::create_dir_all("docs/releases")?;
    fs::write(&notes_path, &notes)?;
    println!("  {} Release notes saved to {}", "✓".green(), notes_path);
    
    println!("\n{}: Release v{} created successfully!", 
        "Success".green().bold(), 
        target_version
    );
    println!("\nNext steps:");
    println!("  1. Review the changes");
    println!("  2. Push with: git push && git push --tags");
    println!("  3. Create GitHub release with the generated notes");
    
    Ok(())
}

pub fn generate_changelog(range: Option<&str>, format: &str) -> Result<()> {
    let range = range.unwrap_or("HEAD");
    let commits = get_commits_in_range(range)?;
    
    let changelog = match format {
        "markdown" => format_changelog_markdown(&commits),
        "json" => format_changelog_json(&commits)?,
        _ => anyhow::bail!("Unsupported format: {}", format),
    };
    
    println!("{}", changelog);
    Ok(())
}

pub fn generate_release_notes(version: Option<&str>) -> Result<()> {
    let version = version.unwrap_or_else(|| {
        fs::read_to_string("VERSION")
            .unwrap_or_else(|_| "0.1.0".to_string())
            .trim()
            .to_string()
    });
    
    let notes = generate_release_notes_content(&version)?;
    println!("{}", notes);
    Ok(())
}

fn determine_next_version() -> Result<String> {
    let current = crate::version::read_version_file()?;
    let mut version = Version::parse(&current)?;
    
    // Get commits since last tag
    let last_tag = get_last_tag()?;
    let commits = get_commits_in_range(&format!("{}..HEAD", last_tag))?;
    
    // Analyze commits to determine version bump
    let mut has_breaking = false;
    let mut has_features = false;
    let mut has_fixes = false;
    
    for commit in &commits {
        if commit.breaking {
            has_breaking = true;
        }
        match commit.commit_type.as_str() {
            "feat" => has_features = true,
            "fix" => has_fixes = true,
            _ => {}
        }
    }
    
    if has_breaking {
        version.major += 1;
        version.minor = 0;
        version.patch = 0;
    } else if has_features {
        version.minor += 1;
        version.patch = 0;
    } else if has_fixes {
        version.patch += 1;
    }
    
    Ok(version.to_string())
}

fn run_tests() -> Result<()> {
    let output = Command::new("cargo")
        .args(&["test", "--workspace", "--quiet"])
        .output()
        .context("Failed to run tests")?;
    
    if !output.status.success() {
        anyhow::bail!("Tests failed:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}

fn create_git_tag(version: &str) -> Result<()> {
    let tag = format!("v{}", version);
    let message = format!("Release version {}", version);
    
    Command::new("git")
        .args(&["tag", "-a", &tag, "-m", &message])
        .status()
        .context("Failed to create git tag")?;
    
    Ok(())
}

fn get_last_tag() -> Result<String> {
    let output = Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0"])
        .output()
        .context("Failed to get last tag")?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("HEAD~10".to_string()) // Default to last 10 commits if no tags
    }
}

#[derive(Debug)]
struct Commit {
    hash: String,
    commit_type: String,
    scope: Option<String>,
    description: String,
    breaking: bool,
    author: String,
    date: String,
}

fn get_commits_in_range(range: &str) -> Result<Vec<Commit>> {
    let output = Command::new("git")
        .args(&[
            "log",
            range,
            "--pretty=format:%H|%s|%an|%ad",
            "--date=short",
        ])
        .output()
        .context("Failed to get git log")?;
    
    let log = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    
    let commit_re = Regex::new(
        r"^([a-z]+)(?:\(([^)]+)\))?: (.+)$"
    )?;
    
    for line in log.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 4 {
            continue;
        }
        
        let hash = parts[0].to_string();
        let subject = parts[1];
        let author = parts[2].to_string();
        let date = parts[3].to_string();
        
        if let Some(caps) = commit_re.captures(subject) {
            commits.push(Commit {
                hash: hash[..7].to_string(),
                commit_type: caps[1].to_string(),
                scope: caps.get(2).map(|m| m.as_str().to_string()),
                description: caps[3].to_string(),
                breaking: subject.contains("BREAKING"),
                author,
                date,
            });
        }
    }
    
    Ok(commits)
}

fn format_changelog_markdown(commits: &[Commit]) -> String {
    let mut grouped: HashMap<String, Vec<&Commit>> = HashMap::new();
    
    for commit in commits {
        grouped
            .entry(commit.commit_type.clone())
            .or_default()
            .push(commit);
    }
    
    let mut output = String::new();
    
    // Breaking changes
    let breaking: Vec<&Commit> = commits.iter().filter(|c| c.breaking).collect();
    if !breaking.is_empty() {
        output.push_str("### ⚠️ BREAKING CHANGES\n\n");
        for commit in breaking {
            output.push_str(&format!(
                "* {}{} ({})\n",
                commit.scope.as_ref().map(|s| format!("**{}:** ", s)).unwrap_or_default(),
                commit.description,
                commit.hash
            ));
        }
        output.push('\n');
    }
    
    // Features
    if let Some(features) = grouped.get("feat") {
        output.push_str("### ✨ Features\n\n");
        for commit in features {
            output.push_str(&format!(
                "* {}{} ({})\n",
                commit.scope.as_ref().map(|s| format!("**{}:** ", s)).unwrap_or_default(),
                commit.description,
                commit.hash
            ));
        }
        output.push('\n');
    }
    
    // Bug fixes
    if let Some(fixes) = grouped.get("fix") {
        output.push_str("### 🐛 Bug Fixes\n\n");
        for commit in fixes {
            output.push_str(&format!(
                "* {}{} ({})\n",
                commit.scope.as_ref().map(|s| format!("**{}:** ", s)).unwrap_or_default(),
                commit.description,
                commit.hash
            ));
        }
        output.push('\n');
    }
    
    // Performance
    if let Some(perfs) = grouped.get("perf") {
        output.push_str("### ⚡ Performance\n\n");
        for commit in perfs {
            output.push_str(&format!(
                "* {}{} ({})\n",
                commit.scope.as_ref().map(|s| format!("**{}:** ", s)).unwrap_or_default(),
                commit.description,
                commit.hash
            ));
        }
        output.push('\n');
    }
    
    // Other changes
    let other_types = vec!["docs", "style", "refactor", "test", "build", "ci", "chore"];
    let mut has_other = false;
    for commit_type in other_types {
        if grouped.contains_key(commit_type) {
            has_other = true;
            break;
        }
    }
    
    if has_other {
        output.push_str("### 📝 Other Changes\n\n");
        for commit_type in other_types {
            if let Some(commits) = grouped.get(commit_type) {
                for commit in commits {
                    output.push_str(&format!(
                        "* {}{} ({})\n",
                        commit.scope.as_ref().map(|s| format!("**{}:** ", s)).unwrap_or_default(),
                        commit.description,
                        commit.hash
                    ));
                }
            }
        }
    }
    
    output
}

fn format_changelog_json(commits: &[Commit]) -> Result<String> {
    let json = serde_json::to_string_pretty(commits)?;
    Ok(json)
}

fn generate_changelog_file(version: &str) -> Result<()> {
    let changelog_path = "CHANGELOG.md";
    let existing = fs::read_to_string(changelog_path).unwrap_or_default();
    
    let last_tag = get_last_tag()?;
    let commits = get_commits_in_range(&format!("{}..HEAD", last_tag))?;
    let new_section = format!(
        "## [{}] - {}\n\n{}\n",
        version,
        Utc::now().format("%Y-%m-%d"),
        format_changelog_markdown(&commits)
    );
    
    // Insert new section after the title
    let mut lines: Vec<&str> = existing.lines().collect();
    let insert_pos = lines.iter().position(|l| l.starts_with("## ")).unwrap_or(1);
    
    let mut new_content = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == insert_pos {
            new_content.push_str(&new_section);
        }
        new_content.push_str(line);
        new_content.push('\n');
    }
    
    fs::write(changelog_path, new_content)?;
    Ok(())
}

fn generate_release_notes_content(version: &str) -> Result<String> {
    let last_tag = get_last_tag()?;
    let commits = get_commits_in_range(&format!("{}..HEAD", last_tag))?;
    
    let mut notes = format!("# Release v{}\n\n", version);
    notes.push_str(&format!("Released: {}\n\n", Utc::now().format("%Y-%m-%d")));
    
    // Summary
    notes.push_str("## Summary\n\n");
    notes.push_str("This release includes ");
    
    let features = commits.iter().filter(|c| c.commit_type == "feat").count();
    let fixes = commits.iter().filter(|c| c.commit_type == "fix").count();
    let breaking = commits.iter().filter(|c| c.breaking).count();
    
    let mut summary_parts = Vec::new();
    if features > 0 {
        summary_parts.push(format!("{} new feature{}", features, if features > 1 { "s" } else { "" }));
    }
    if fixes > 0 {
        summary_parts.push(format!("{} bug fix{}", fixes, if fixes > 1 { "es" } else { "" }));
    }
    if breaking > 0 {
        summary_parts.push(format!("{} breaking change{}", breaking, if breaking > 1 { "s" } else { "" }));
    }
    
    notes.push_str(&summary_parts.join(", "));
    notes.push_str(".\n\n");
    
    // Changelog
    notes.push_str("## Changelog\n\n");
    notes.push_str(&format_changelog_markdown(&commits));
    
    // Installation
    notes.push_str("## Installation\n\n");
    notes.push_str("```bash\n");
    notes.push_str("# Download the ISO\n");
    notes.push_str(&format!("wget https://github.com/Arakiss/hecate-os/releases/download/v{}/hecate-os-{}.iso\n", version, version));
    notes.push_str("\n");
    notes.push_str("# Or update existing installation\n");
    notes.push_str("hecate-pkg update && hecate-pkg upgrade\n");
    notes.push_str("```\n\n");
    
    // Contributors
    let contributors: Vec<String> = commits
        .iter()
        .map(|c| c.author.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    notes.push_str("## Contributors\n\n");
    for contributor in contributors {
        notes.push_str(&format!("* {}\n", contributor));
    }
    
    Ok(notes)
}

