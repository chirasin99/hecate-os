use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use colored::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about = "Generate changelog from git commits")]
struct Cli {
    /// Version for the changelog
    #[arg(short, long)]
    version: Option<String>,
    
    /// Git range (e.g., v0.1.0..HEAD)
    #[arg(short, long, default_value = "HEAD~10..HEAD")]
    range: String,
    
    /// Output format (markdown, json, html)
    #[arg(short = 'f', long, default_value = "markdown")]
    format: String,
    
    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<String>,
    
    /// Update CHANGELOG.md file
    #[arg(short = 'u', long)]
    update: bool,
}

#[derive(Debug, serde::Serialize)]
struct Commit {
    hash: String,
    commit_type: String,
    scope: Option<String>,
    description: String,
    breaking: bool,
    author: String,
    date: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let commits = parse_commits(&cli.range)?;
    
    if commits.is_empty() {
        println!("{} No commits found in range {}", "⚠".yellow(), cli.range);
        return Ok(());
    }
    
    let changelog = match cli.format.as_str() {
        "markdown" => generate_markdown(&commits, cli.version.as_deref()),
        "json" => serde_json::to_string_pretty(&commits)?,
        "html" => generate_html(&commits, cli.version.as_deref()),
        _ => anyhow::bail!("Unsupported format: {}", cli.format),
    };
    
    if cli.update {
        update_changelog_file(&changelog, cli.version.as_deref())?;
        println!("{} Updated CHANGELOG.md", "✓".green().bold());
    } else if let Some(output) = cli.output {
        fs::write(&output, changelog)?;
        println!("{} Wrote changelog to {}", "✓".green().bold(), output);
    } else {
        println!("{}", changelog);
    }
    
    Ok(())
}

fn parse_commits(range: &str) -> Result<Vec<Commit>> {
    let output = Command::new("git")
        .args(&[
            "log",
            range,
            "--pretty=format:%H|%s|%an|%ad",
            "--date=short",
        ])
        .output()?;
    
    let log = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    
    let commit_re = Regex::new(r"^([a-z]+)(?:\(([^)]+)\))?: (.+)$")?;
    
    for line in log.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 4 {
            continue;
        }
        
        let hash = parts[0][..7].to_string();
        let subject = parts[1];
        let author = parts[2].to_string();
        let date = parts[3].to_string();
        
        if let Some(caps) = commit_re.captures(subject) {
            commits.push(Commit {
                hash,
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

fn generate_markdown(commits: &[Commit], version: Option<&str>) -> String {
    let mut output = String::new();
    
    if let Some(v) = version {
        output.push_str(&format!("## [{}] - {}\n\n", v, Utc::now().format("%Y-%m-%d")));
    }
    
    let mut grouped: HashMap<String, Vec<&Commit>> = HashMap::new();
    for commit in commits {
        grouped.entry(commit.commit_type.clone()).or_default().push(commit);
    }
    
    // Breaking changes
    let breaking: Vec<&Commit> = commits.iter().filter(|c| c.breaking).collect();
    if !breaking.is_empty() {
        output.push_str("### ⚠️ BREAKING CHANGES\n\n");
        for commit in breaking {
            output.push_str(&format!("* {}", format_commit(commit)));
        }
        output.push('\n');
    }
    
    // Features
    if let Some(features) = grouped.get("feat") {
        output.push_str("### ✨ Features\n\n");
        for commit in features {
            output.push_str(&format!("* {}", format_commit(commit)));
        }
        output.push('\n');
    }
    
    // Bug Fixes
    if let Some(fixes) = grouped.get("fix") {
        output.push_str("### 🐛 Bug Fixes\n\n");
        for commit in fixes {
            output.push_str(&format!("* {}", format_commit(commit)));
        }
        output.push('\n');
    }
    
    // Other sections
    let sections = vec![
        ("docs", "📝 Documentation"),
        ("perf", "⚡ Performance"),
        ("refactor", "♻️ Refactoring"),
        ("test", "✅ Tests"),
        ("build", "🏗️ Build"),
        ("ci", "🔧 CI"),
        ("chore", "🧹 Chores"),
    ];
    
    for (commit_type, title) in sections {
        if let Some(commits) = grouped.get(commit_type) {
            output.push_str(&format!("### {}\n\n", title));
            for commit in commits {
                output.push_str(&format!("* {}", format_commit(commit)));
            }
            output.push('\n');
        }
    }
    
    output
}

fn generate_html(commits: &[Commit], version: Option<&str>) -> String {
    let markdown = generate_markdown(commits, version);
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Changelog</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
        h2 {{ color: #2c3e50; }}
        h3 {{ color: #34495e; }}
        ul {{ line-height: 1.8; }}
        code {{ background: #f4f4f4; padding: 2px 4px; border-radius: 3px; }}
    </style>
</head>
<body>
    {}
</body>
</html>"#,
        markdown_to_html(&markdown)
    )
}

fn format_commit(commit: &Commit) -> String {
    format!(
        "{}{} ({})\n",
        commit.scope.as_ref().map(|s| format!("**{}:** ", s)).unwrap_or_default(),
        commit.description,
        commit.hash
    )
}

fn markdown_to_html(markdown: &str) -> String {
    markdown
        .replace("### ", "<h3>")
        .replace("## ", "<h2>")
        .replace("\n\n", "</p><p>")
        .replace("* ", "<li>")
        .replace("\n", "</li>")
}

fn update_changelog_file(new_content: &str, version: Option<&str>) -> Result<()> {
    let path = "CHANGELOG.md";
    let existing = fs::read_to_string(path).unwrap_or_else(|_| "# Changelog\n\n".to_string());
    
    let mut lines: Vec<&str> = existing.lines().collect();
    let insert_pos = lines.iter()
        .position(|l| l.starts_with("## "))
        .unwrap_or(lines.len());
    
    let mut updated = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == insert_pos {
            updated.push_str(new_content);
            updated.push_str("\n");
        }
        updated.push_str(line);
        updated.push('\n');
    }
    
    fs::write(path, updated)?;
    Ok(())
}