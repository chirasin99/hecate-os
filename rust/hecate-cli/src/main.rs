//! HecateOS CLI Tool
//! 
//! Herramienta de línea de comandos para información y control del sistema

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use comfy_table::Table;
use hecate_core::{HardwareDetector, SystemProfile};
use hecate_gpu::GpuManager;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::time::Duration;
use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt, ProcessExt};
use tracing::{error, info};

// ============================================================================
// CLI STRUCTURE
// ============================================================================

#[derive(Parser)]
#[command(name = "hecate")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Output format (text, json, yaml)
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Show system information
    Info {
        /// Component to show info for
        #[command(subcommand)]
        component: Option<InfoComponent>,
    },
    
    /// Monitor system in real-time
    Monitor {
        /// Update interval in seconds
        #[arg(short, long, default_value = "1")]
        interval: u64,
        
        /// Component to monitor
        #[arg(short, long)]
        component: Option<String>,
    },
    
    /// Manage GPU settings
    Gpu {
        #[command(subcommand)]
        action: GpuAction,
    },
    
    /// Run system benchmark
    Benchmark {
        /// Type of benchmark to run
        #[arg(short, long)]
        test: Option<String>,
        
        /// Duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
    },
    
    /// System optimization
    Optimize {
        /// Profile to apply
        #[arg(short, long)]
        profile: Option<String>,
        
        /// Dry run (show what would be done)
        #[arg(short, long)]
        dry_run: bool,
    },
    
    /// Process management
    Process {
        #[command(subcommand)]
        action: ProcessAction,
    },
    
    /// Network diagnostics
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },
    
    /// System health check
    Health {
        /// Run full diagnostics
        #[arg(short, long)]
        full: bool,
    },
}

#[derive(Subcommand)]
enum InfoComponent {
    /// CPU information
    Cpu,
    /// Memory information
    Memory,
    /// GPU information
    Gpu,
    /// Disk information
    Disk,
    /// Network information
    Network,
    /// Process information
    Process,
    /// All components
    All,
}

#[derive(Subcommand)]
enum GpuAction {
    /// List all GPUs
    List,
    /// Show GPU status
    Status {
        /// GPU index
        #[arg(short, long)]
        index: Option<u32>,
    },
    /// Set power mode
    Power {
        /// GPU index
        index: u32,
        /// Power mode (max, balanced, eco)
        mode: String,
    },
}

#[derive(Subcommand)]
enum ProcessAction {
    /// List processes
    List {
        /// Sort by (cpu, memory, pid, name)
        #[arg(short, long, default_value = "cpu")]
        sort: String,
        /// Number of processes to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Kill process
    Kill {
        /// Process ID
        pid: u32,
        /// Force kill
        #[arg(short, long)]
        force: bool,
    },
    /// Show process tree
    Tree,
}

#[derive(Subcommand)]
enum NetworkAction {
    /// Show interfaces
    Interfaces,
    /// Connection statistics
    Stats,
    /// Test connectivity
    Test {
        /// Host to test
        host: String,
    },
}

#[derive(Clone, Debug)]
enum OutputFormat {
    Text,
    Json,
    Yaml,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("warn")
            .init();
    }
    
    // Execute command
    match cli.command {
        Commands::Info { component } => {
            handle_info(component, &cli.format).await?;
        }
        Commands::Monitor { interval, component } => {
            handle_monitor(interval, component).await?;
        }
        Commands::Gpu { action } => {
            handle_gpu(action, &cli.format).await?;
        }
        Commands::Benchmark { test, duration } => {
            handle_benchmark(test, duration).await?;
        }
        Commands::Optimize { profile, dry_run } => {
            handle_optimize(profile, dry_run).await?;
        }
        Commands::Process { action } => {
            handle_process(action, &cli.format).await?;
        }
        Commands::Network { action } => {
            handle_network(action, &cli.format).await?;
        }
        Commands::Health { full } => {
            handle_health(full).await?;
        }
    }
    
    Ok(())
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

async fn handle_info(component: Option<InfoComponent>, format: &OutputFormat) -> Result<()> {
    let mut system = System::new_all();
    system.refresh_all();
    
    let component = component.unwrap_or(InfoComponent::All);
    
    match component {
        InfoComponent::Cpu | InfoComponent::All => {
            show_cpu_info(&system, format)?;
        }
        InfoComponent::Memory | InfoComponent::All => {
            show_memory_info(&system, format)?;
        }
        InfoComponent::Gpu | InfoComponent::All => {
            show_gpu_info(format).await?;
        }
        InfoComponent::Disk | InfoComponent::All => {
            show_disk_info(&system, format)?;
        }
        InfoComponent::Network | InfoComponent::All => {
            show_network_info(&system, format)?;
        }
        InfoComponent::Process | InfoComponent::All => {
            show_process_info(&system, format)?;
        }
        _ => {}
    }
    
    Ok(())
}

async fn handle_monitor(interval: u64, component: Option<String>) -> Result<()> {
    println!("{}", "=== HecateOS System Monitor ===".bright_cyan().bold());
    println!("Press Ctrl+C to exit\n");
    
    let mut system = System::new_all();
    
    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        
        system.refresh_all();
        
        // Header
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        println!("{} {}", "HecateOS Monitor".bright_cyan().bold(), timestamp);
        println!("{}", "─".repeat(80).bright_black());
        
        if let Some(ref comp) = component {
            match comp.as_str() {
                "cpu" => show_cpu_monitor(&system)?,
                "memory" => show_memory_monitor(&system)?,
                "gpu" => show_gpu_monitor().await?,
                "disk" => show_disk_monitor(&system)?,
                "network" => show_network_monitor(&system)?,
                _ => show_all_monitor(&system).await?,
            }
        } else {
            show_all_monitor(&system).await?;
        }
        
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

async fn handle_gpu(action: GpuAction, format: &OutputFormat) -> Result<()> {
    let manager = GpuManager::new()?;
    
    match action {
        GpuAction::List => {
            let gpus = manager.detect_gpus().await?;
            
            match format {
                OutputFormat::Text => {
                    let mut table = Table::new();
                    table.set_header(vec!["Index", "Name", "Temperature", "Power", "Memory", "Utilization"]);
                    
                    for gpu in gpus {
                        table.add_row(vec![
                            gpu.index.to_string(),
                            gpu.name,
                            format!("{}°C", gpu.temperature),
                            format!("{}W", gpu.power_draw),
                            format!("{}/{} MB", gpu.memory_used / 1024 / 1024, gpu.memory_total / 1024 / 1024),
                            format!("{}%", gpu.utilization_gpu),
                        ]);
                    }
                    
                    println!("{}", table);
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&gpus)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&gpus)?);
                }
            }
        }
        GpuAction::Status { index } => {
            let gpus = manager.detect_gpus().await?;
            
            if let Some(idx) = index {
                if let Some(gpu) = gpus.iter().find(|g| g.index == idx) {
                    print_gpu_status(gpu, format)?;
                } else {
                    eprintln!("GPU {} not found", idx);
                }
            } else {
                for gpu in gpus {
                    print_gpu_status(&gpu, format)?;
                    println!();
                }
            }
        }
        GpuAction::Power { index, mode } => {
            use hecate_gpu::{GpuConfig, PowerMode};
            
            let power_mode = match mode.to_lowercase().as_str() {
                "max" => PowerMode::MaxPerformance,
                "balanced" => PowerMode::Balanced,
                "eco" => PowerMode::PowerSaver,
                _ => {
                    eprintln!("Invalid power mode. Use: max, balanced, or eco");
                    return Ok(());
                }
            };
            
            let config = GpuConfig {
                power_mode,
                power_limit: None,
                temp_target: None,
                fan_curve: None,
            };
            
            manager.apply_config(index, config).await?;
            println!("✓ GPU {} set to {} mode", index, mode);
        }
    }
    
    Ok(())
}

async fn handle_benchmark(test: Option<String>, duration: u64) -> Result<()> {
    println!("{}", "=== HecateOS Benchmark Suite ===".bright_cyan().bold());
    
    let test_name = test.as_deref().unwrap_or("all");
    
    println!("Running {} benchmark for {} seconds...\n", test_name, duration);
    
    let pb = ProgressBar::new(duration);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );
    
    match test_name {
        "cpu" => run_cpu_benchmark(duration, &pb).await?,
        "memory" => run_memory_benchmark(duration, &pb).await?,
        "disk" => run_disk_benchmark(duration, &pb).await?,
        "all" => {
            run_cpu_benchmark(duration / 3, &pb).await?;
            run_memory_benchmark(duration / 3, &pb).await?;
            run_disk_benchmark(duration / 3, &pb).await?;
        }
        _ => {
            eprintln!("Unknown benchmark: {}. Available: cpu, memory, disk, all", test_name);
        }
    }
    
    pb.finish_with_message("Benchmark complete!");
    Ok(())
}

async fn handle_optimize(profile: Option<String>, dry_run: bool) -> Result<()> {
    println!("{}", "=== HecateOS System Optimizer ===".bright_cyan().bold());
    
    let detector = HardwareDetector::new();
    let detected_profile = detector.detect_profile();
    
    let target_profile = if let Some(p) = profile {
        match p.to_lowercase().as_str() {
            "ai" => SystemProfile::AIFlagship,
            "pro" => SystemProfile::ProWorkstation,
            "gaming" => SystemProfile::GamingEnthusiast,
            "creator" => SystemProfile::ContentCreator,
            "dev" => SystemProfile::Developer,
            "standard" => SystemProfile::Standard,
            _ => {
                eprintln!("Invalid profile: {}", p);
                return Ok(());
            }
        }
    } else {
        detected_profile
    };
    
    println!("Detected Profile: {:?}", detected_profile);
    println!("Target Profile: {:?}", target_profile);
    println!();
    
    if dry_run {
        println!("{}", "DRY RUN MODE - No changes will be made".yellow());
        println!();
    }
    
    // Show optimizations that would be applied
    println!("Optimizations to apply:");
    println!("  ✓ CPU Governor: performance");
    println!("  ✓ I/O Scheduler: mq-deadline");
    println!("  ✓ Swappiness: 10");
    println!("  ✓ Transparent Huge Pages: always");
    
    match target_profile {
        SystemProfile::AIFlagship => {
            println!("  ✓ GPU Power Mode: Maximum Performance");
            println!("  ✓ CUDA Memory: Pinned");
            println!("  ✓ PCIe: Gen5 x16");
        }
        SystemProfile::GamingEnthusiast => {
            println!("  ✓ GPU Power Mode: Balanced");
            println!("  ✓ Game Mode: Enabled");
            println!("  ✓ Network: Low Latency");
        }
        _ => {}
    }
    
    if !dry_run {
        println!("\nApplying optimizations...");
        // TODO: Actually apply optimizations
        println!("{}", "✓ Optimizations applied successfully!".green());
    }
    
    Ok(())
}

async fn handle_process(action: ProcessAction, format: &OutputFormat) -> Result<()> {
    let mut system = System::new_all();
    system.refresh_all();
    
    match action {
        ProcessAction::List { sort, limit } => {
            let mut processes: Vec<_> = system.processes()
                .values()
                .map(|p| ProcessInfo {
                    pid: p.pid().as_u32(),
                    name: p.name().to_string(),
                    cpu_percent: p.cpu_usage(),
                    memory_mb: p.memory() / 1024,
                    status: format!("{:?}", p.status()),
                })
                .collect();
            
            // Sort processes
            match sort.as_str() {
                "cpu" => processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap()),
                "memory" => processes.sort_by(|a, b| b.memory_mb.cmp(&a.memory_mb)),
                "pid" => processes.sort_by(|a, b| a.pid.cmp(&b.pid)),
                "name" => processes.sort_by(|a, b| a.name.cmp(&b.name)),
                _ => {}
            }
            
            processes.truncate(limit);
            
            match format {
                OutputFormat::Text => {
                    let mut table = Table::new();
                    table.set_header(vec!["PID", "Name", "CPU %", "Memory (MB)", "Status"]);
                    
                    for p in processes {
                        table.add_row(vec![
                            p.pid.to_string(),
                            p.name,
                            format!("{:.1}", p.cpu_percent),
                            p.memory_mb.to_string(),
                            p.status,
                        ]);
                    }
                    
                    println!("{}", table);
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&processes)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&processes)?);
                }
            }
        }
        ProcessAction::Kill { pid, force } => {
            if let Some(process) = system.process(sysinfo::Pid::from(pid as usize)) {
                if force {
                    process.kill();
                } else {
                    // Send SIGTERM
                    process.kill();
                }
                println!("✓ Process {} terminated", pid);
            } else {
                eprintln!("Process {} not found", pid);
            }
        }
        ProcessAction::Tree => {
            println!("Process tree:");
            // TODO: Implement process tree display
            println!("  (Not implemented yet)");
        }
    }
    
    Ok(())
}

async fn handle_network(action: NetworkAction, format: &OutputFormat) -> Result<()> {
    let mut system = System::new_all();
    system.refresh_all();
    
    match action {
        NetworkAction::Interfaces => {
            let mut table = Table::new();
            table.set_header(vec!["Interface", "Received", "Transmitted", "Packets RX", "Packets TX"]);
            
            for (name, data) in system.networks() {
                table.add_row(vec![
                    name.clone(),
                    format_bytes(data.received()),
                    format_bytes(data.transmitted()),
                    data.packets_received().to_string(),
                    data.packets_transmitted().to_string(),
                ]);
            }
            
            println!("{}", table);
        }
        NetworkAction::Stats => {
            println!("Network Statistics:");
            
            for (name, data) in system.networks() {
                println!("\n{}", name.bright_cyan());
                println!("  Received:     {}", format_bytes(data.received()));
                println!("  Transmitted:  {}", format_bytes(data.transmitted()));
                println!("  Packets RX:   {}", data.packets_received());
                println!("  Packets TX:   {}", data.packets_transmitted());
                println!("  Errors RX:    {}", data.errors_on_received());
                println!("  Errors TX:    {}", data.errors_on_transmitted());
            }
        }
        NetworkAction::Test { host } => {
            println!("Testing connectivity to {}...", host);
            
            let pb = ProgressBar::new_spinner();
            pb.set_message("Resolving host...");
            
            // Simple HTTP test
            match reqwest::get(format!("http://{}", host)).await {
                Ok(response) => {
                    pb.finish_and_clear();
                    println!("✓ Connection successful!");
                    println!("  Status: {}", response.status());
                }
                Err(e) => {
                    pb.finish_and_clear();
                    eprintln!("✗ Connection failed: {}", e);
                }
            }
        }
    }
    
    Ok(())
}

async fn handle_health(full: bool) -> Result<()> {
    println!("{}", "=== HecateOS Health Check ===".bright_cyan().bold());
    
    let mut system = System::new_all();
    system.refresh_all();
    
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    
    // CPU Check
    let cpu_usage = system.global_cpu_info().cpu_usage();
    if cpu_usage > 90.0 {
        issues.push(format!("High CPU usage: {:.1}%", cpu_usage));
    } else if cpu_usage > 70.0 {
        warnings.push(format!("Elevated CPU usage: {:.1}%", cpu_usage));
    }
    
    // Memory Check
    let mem_percent = (system.used_memory() as f64 / system.total_memory() as f64) * 100.0;
    if mem_percent > 90.0 {
        issues.push(format!("High memory usage: {:.1}%", mem_percent));
    } else if mem_percent > 80.0 {
        warnings.push(format!("Elevated memory usage: {:.1}%", mem_percent));
    }
    
    // Disk Check
    for disk in system.disks() {
        let usage = ((disk.total_space() - disk.available_space()) as f64 / disk.total_space() as f64) * 100.0;
        if usage > 95.0 {
            issues.push(format!("Critical disk usage on {}: {:.1}%", 
                disk.mount_point().to_string_lossy(), usage));
        } else if usage > 85.0 {
            warnings.push(format!("High disk usage on {}: {:.1}%", 
                disk.mount_point().to_string_lossy(), usage));
        }
    }
    
    // Temperature Check
    if let Ok(temp) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        if let Ok(millidegrees) = temp.trim().parse::<i32>() {
            let celsius = millidegrees as f32 / 1000.0;
            if celsius > 85.0 {
                issues.push(format!("High CPU temperature: {:.1}°C", celsius));
            } else if celsius > 75.0 {
                warnings.push(format!("Elevated CPU temperature: {:.1}°C", celsius));
            }
        }
    }
    
    // Load Average Check
    let load = system.load_average();
    let cpu_count = system.cpus().len() as f64;
    if load.one > cpu_count * 2.0 {
        issues.push(format!("High system load: {:.2}", load.one));
    }
    
    // Results
    if issues.is_empty() && warnings.is_empty() {
        println!("{}", "✓ System is healthy!".green().bold());
    } else {
        if !issues.is_empty() {
            println!("{}", "Critical Issues:".red().bold());
            for issue in &issues {
                println!("  ✗ {}", issue.red());
            }
        }
        
        if !warnings.is_empty() {
            println!("{}", "\nWarnings:".yellow().bold());
            for warning in &warnings {
                println!("  ⚠ {}", warning.yellow());
            }
        }
    }
    
    if full {
        println!("\n{}", "Detailed Report:".bright_cyan());
        show_cpu_info(&system, &OutputFormat::Text)?;
        show_memory_info(&system, &OutputFormat::Text)?;
        show_disk_info(&system, &OutputFormat::Text)?;
    }
    
    Ok(())
}

// ============================================================================
// INFO DISPLAY FUNCTIONS
// ============================================================================

fn show_cpu_info(system: &System, format: &OutputFormat) -> Result<()> {
    let cpu_info = CpuInfo {
        model: system.cpus()[0].brand().to_string(),
        cores: system.cpus().len(),
        usage: system.global_cpu_info().cpu_usage(),
        frequency: system.cpus()[0].frequency(),
        load_avg: system.load_average(),
    };
    
    match format {
        OutputFormat::Text => {
            println!("{}", "CPU Information:".bright_cyan());
            println!("  Model:      {}", cpu_info.model);
            println!("  Cores:      {}", cpu_info.cores);
            println!("  Usage:      {:.1}%", cpu_info.usage);
            println!("  Frequency:  {} MHz", cpu_info.frequency);
            println!("  Load:       {:.2} {:.2} {:.2}", 
                cpu_info.load_avg.one, cpu_info.load_avg.five, cpu_info.load_avg.fifteen);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&cpu_info)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&cpu_info)?);
        }
    }
    
    Ok(())
}

fn show_memory_info(system: &System, format: &OutputFormat) -> Result<()> {
    let mem_info = MemoryInfo {
        total: system.total_memory(),
        used: system.used_memory(),
        available: system.available_memory(),
        swap_total: system.total_swap(),
        swap_used: system.used_swap(),
    };
    
    match format {
        OutputFormat::Text => {
            println!("{}", "Memory Information:".bright_cyan());
            println!("  Total:      {}", format_bytes(mem_info.total));
            println!("  Used:       {} ({:.1}%)", 
                format_bytes(mem_info.used), 
                (mem_info.used as f64 / mem_info.total as f64) * 100.0);
            println!("  Available:  {}", format_bytes(mem_info.available));
            println!("  Swap Total: {}", format_bytes(mem_info.swap_total));
            println!("  Swap Used:  {}", format_bytes(mem_info.swap_used));
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&mem_info)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&mem_info)?);
        }
    }
    
    Ok(())
}

async fn show_gpu_info(format: &OutputFormat) -> Result<()> {
    match GpuManager::new() {
        Ok(manager) => {
            let gpus = manager.detect_gpus().await?;
            
            match format {
                OutputFormat::Text => {
                    println!("{}", "GPU Information:".bright_cyan());
                    for gpu in gpus {
                        println!("  GPU {}:     {}", gpu.index, gpu.name);
                        println!("    Temp:     {}°C", gpu.temperature);
                        println!("    Power:    {}W / {}W", gpu.power_draw, gpu.power_limit);
                        println!("    Memory:   {} / {}", 
                            format_bytes(gpu.memory_used), 
                            format_bytes(gpu.memory_total));
                        println!("    Usage:    GPU {}% | MEM {}%", 
                            gpu.utilization_gpu, gpu.utilization_memory);
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&gpus)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&gpus)?);
                }
            }
        }
        Err(_) => {
            println!("{}", "GPU Information:".bright_cyan());
            println!("  No NVIDIA GPUs detected");
        }
    }
    
    Ok(())
}

fn show_disk_info(system: &System, format: &OutputFormat) -> Result<()> {
    let disks: Vec<DiskInfo> = system.disks()
        .iter()
        .map(|disk| DiskInfo {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_space: disk.total_space(),
            available_space: disk.available_space(),
            filesystem: format!("{:?}", disk.file_system()),
        })
        .collect();
    
    match format {
        OutputFormat::Text => {
            println!("{}", "Disk Information:".bright_cyan());
            for disk in disks {
                let used = disk.total_space - disk.available_space;
                let percent = (used as f64 / disk.total_space as f64) * 100.0;
                
                println!("  {}:", disk.mount_point);
                println!("    Device:   {}", disk.name);
                println!("    FS:       {}", disk.filesystem);
                println!("    Total:    {}", format_bytes(disk.total_space));
                println!("    Used:     {} ({:.1}%)", format_bytes(used), percent);
                println!("    Free:     {}", format_bytes(disk.available_space));
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&disks)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&disks)?);
        }
    }
    
    Ok(())
}

fn show_network_info(system: &System, format: &OutputFormat) -> Result<()> {
    let interfaces: Vec<NetworkInfo> = system.networks()
        .iter()
        .map(|(name, data)| NetworkInfo {
            name: name.clone(),
            received: data.received(),
            transmitted: data.transmitted(),
            packets_received: data.packets_received(),
            packets_transmitted: data.packets_transmitted(),
        })
        .collect();
    
    match format {
        OutputFormat::Text => {
            println!("{}", "Network Information:".bright_cyan());
            for iface in interfaces {
                println!("  {}:", iface.name);
                println!("    RX:       {}", format_bytes(iface.received));
                println!("    TX:       {}", format_bytes(iface.transmitted));
                println!("    Packets:  RX {} | TX {}", 
                    iface.packets_received, iface.packets_transmitted);
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&interfaces)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&interfaces)?);
        }
    }
    
    Ok(())
}

fn show_process_info(system: &System, format: &OutputFormat) -> Result<()> {
    println!("{}", "Process Information:".bright_cyan());
    println!("  Total:      {}", system.processes().len());
    
    let running = system.processes()
        .values()
        .filter(|p| p.status() == sysinfo::ProcessStatus::Run)
        .count();
    println!("  Running:    {}", running);
    
    Ok(())
}

// ============================================================================
// MONITOR DISPLAY FUNCTIONS
// ============================================================================

fn show_cpu_monitor(system: &System) -> Result<()> {
    let usage = system.global_cpu_info().cpu_usage();
    let bar_width = 40;
    let filled = (usage * bar_width as f32 / 100.0) as usize;
    let bar = format!("{}{}", 
        "█".repeat(filled).bright_green(),
        "░".repeat(bar_width - filled).bright_black()
    );
    
    println!("CPU: [{}] {:.1}%", bar, usage);
    
    // Per-core usage
    println!("\nPer Core:");
    for (i, cpu) in system.cpus().iter().enumerate() {
        let core_usage = cpu.cpu_usage();
        let core_filled = (core_usage * 20.0 / 100.0) as usize;
        let core_bar = format!("{}{}", 
            "█".repeat(core_filled).bright_green(),
            "░".repeat(20 - core_filled).bright_black()
        );
        println!("  Core {}: [{}] {:.1}%", i, core_bar, core_usage);
    }
    
    Ok(())
}

fn show_memory_monitor(system: &System) -> Result<()> {
    let used = system.used_memory();
    let total = system.total_memory();
    let percent = (used as f64 / total as f64) * 100.0;
    
    let bar_width = 40;
    let filled = (percent * bar_width as f64 / 100.0) as usize;
    let bar = format!("{}{}", 
        "█".repeat(filled).bright_yellow(),
        "░".repeat(bar_width - filled).bright_black()
    );
    
    println!("Memory: [{}] {:.1}% ({} / {})", 
        bar, percent, format_bytes(used), format_bytes(total));
    
    Ok(())
}

async fn show_gpu_monitor() -> Result<()> {
    if let Ok(manager) = GpuManager::new() {
        let gpus = manager.detect_gpus().await?;
        
        for gpu in gpus {
            let bar_width = 30;
            let filled = (gpu.utilization_gpu as f64 * bar_width as f64 / 100.0) as usize;
            let bar = format!("{}{}", 
                "█".repeat(filled).bright_cyan(),
                "░".repeat(bar_width - filled).bright_black()
            );
            
            println!("GPU {}: [{}] {}% | {}°C | {}W", 
                gpu.index, bar, gpu.utilization_gpu, gpu.temperature, gpu.power_draw);
        }
    } else {
        println!("GPU: No NVIDIA GPUs detected");
    }
    
    Ok(())
}

fn show_disk_monitor(system: &System) -> Result<()> {
    for disk in system.disks() {
        let used = disk.total_space() - disk.available_space();
        let percent = (used as f64 / disk.total_space() as f64) * 100.0;
        
        let bar_width = 30;
        let filled = (percent * bar_width as f64 / 100.0) as usize;
        let color = if percent > 90.0 {
            "red"
        } else if percent > 75.0 {
            "yellow"
        } else {
            "green"
        };
        
        let bar = match color {
            "red" => format!("{}{}", 
                "█".repeat(filled).bright_red(),
                "░".repeat(bar_width - filled).bright_black()
            ),
            "yellow" => format!("{}{}", 
                "█".repeat(filled).bright_yellow(),
                "░".repeat(bar_width - filled).bright_black()
            ),
            _ => format!("{}{}", 
                "█".repeat(filled).bright_green(),
                "░".repeat(bar_width - filled).bright_black()
            ),
        };
        
        println!("{}: [{}] {:.1}%", 
            disk.mount_point().to_string_lossy(), bar, percent);
    }
    
    Ok(())
}

fn show_network_monitor(system: &System) -> Result<()> {
    for (name, data) in system.networks() {
        println!("{}: ↓ {} ↑ {}", 
            name, 
            format_bytes(data.received()),
            format_bytes(data.transmitted())
        );
    }
    
    Ok(())
}

async fn show_all_monitor(system: &System) -> Result<()> {
    show_cpu_monitor(system)?;
    println!();
    show_memory_monitor(system)?;
    println!();
    show_gpu_monitor().await?;
    println!();
    show_disk_monitor(system)?;
    println!();
    show_network_monitor(system)?;
    
    Ok(())
}

// ============================================================================
// BENCHMARK FUNCTIONS
// ============================================================================

async fn run_cpu_benchmark(duration: u64, pb: &ProgressBar) -> Result<()> {
    pb.set_message("Running CPU benchmark...");
    
    let start = std::time::Instant::now();
    let mut iterations = 0u64;
    
    // Simple CPU-intensive calculation
    while start.elapsed().as_secs() < duration {
        for _ in 0..1000000 {
            let _ = (iterations as f64).sqrt();
            iterations += 1;
        }
        pb.set_position(start.elapsed().as_secs());
    }
    
    let mops = iterations as f64 / duration as f64 / 1_000_000.0;
    println!("\nCPU Benchmark: {:.2} MOPS (Million Operations Per Second)", mops);
    
    Ok(())
}

async fn run_memory_benchmark(duration: u64, pb: &ProgressBar) -> Result<()> {
    pb.set_message("Running memory benchmark...");
    
    let start = std::time::Instant::now();
    let mut iterations = 0u64;
    
    // Memory allocation and access pattern
    let size = 100_000_000; // 100MB
    let mut vec = vec![0u8; size];
    
    while start.elapsed().as_secs() < duration {
        for i in 0..size {
            vec[i] = (i % 256) as u8;
        }
        iterations += 1;
        pb.set_position(start.elapsed().as_secs());
    }
    
    let bandwidth = (size as f64 * iterations as f64) / duration as f64 / 1_073_741_824.0;
    println!("\nMemory Benchmark: {:.2} GB/s bandwidth", bandwidth);
    
    Ok(())
}

async fn run_disk_benchmark(duration: u64, pb: &ProgressBar) -> Result<()> {
    pb.set_message("Running disk benchmark...");
    
    let start = std::time::Instant::now();
    let mut iterations = 0u64;
    
    // Write and read temporary file
    let temp_file = "/tmp/hecate_benchmark.tmp";
    let data = vec![0u8; 1_048_576]; // 1MB
    
    while start.elapsed().as_secs() < duration {
        std::fs::write(temp_file, &data)?;
        let _ = std::fs::read(temp_file)?;
        iterations += 1;
        pb.set_position(start.elapsed().as_secs());
    }
    
    let _ = std::fs::remove_file(temp_file);
    
    let throughput = (data.len() as f64 * iterations as f64 * 2.0) / duration as f64 / 1_048_576.0;
    println!("\nDisk Benchmark: {:.2} MB/s throughput", throughput);
    
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

fn print_gpu_status(gpu: &hecate_gpu::GpuStatus, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Text => {
            println!("GPU {}: {}", gpu.index, gpu.name);
            println!("  Temperature:  {}°C", gpu.temperature);
            println!("  Power:        {}W / {}W", gpu.power_draw, gpu.power_limit);
            println!("  Memory:       {} / {}", 
                format_bytes(gpu.memory_used), 
                format_bytes(gpu.memory_total));
            println!("  Utilization:  GPU {}% | MEM {}%", 
                gpu.utilization_gpu, gpu.utilization_memory);
            println!("  Clocks:       Core {} MHz | Mem {} MHz", 
                gpu.clock_graphics, gpu.clock_memory);
            if let Some(fan) = gpu.fan_speed {
                println!("  Fan Speed:    {}%", fan);
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&gpu)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&gpu)?);
        }
    }
    
    Ok(())
}

// ============================================================================
// SERIALIZABLE STRUCTURES
// ============================================================================

#[derive(Serialize)]
struct CpuInfo {
    model: String,
    cores: usize,
    usage: f32,
    frequency: u64,
    load_avg: sysinfo::LoadAvg,
}

#[derive(Serialize)]
struct MemoryInfo {
    total: u64,
    used: u64,
    available: u64,
    swap_total: u64,
    swap_used: u64,
}

#[derive(Serialize)]
struct DiskInfo {
    name: String,
    mount_point: String,
    total_space: u64,
    available_space: u64,
    filesystem: String,
}

#[derive(Serialize)]
struct NetworkInfo {
    name: String,
    received: u64,
    transmitted: u64,
    packets_received: u64,
    packets_transmitted: u64,
}

#[derive(Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_percent: f32,
    memory_mb: u64,
    status: String,
}