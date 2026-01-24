//! # HecateOS ML Workload Optimizer
//!
//! This library provides comprehensive machine learning workload optimization for HecateOS,
//! including automatic configuration for PyTorch, TensorFlow, and ONNX models, batch size
//! optimization, distributed training coordination, and dataset caching strategies.
//!
//! ## Features
//!
//! - **Framework Support**: PyTorch, TensorFlow, ONNX, Hugging Face Transformers
//! - **Automatic Optimization**: Batch size tuning, memory optimization, GPU utilization
//! - **Distributed Training**: Multi-GPU and multi-node coordination
//! - **Dataset Management**: Intelligent caching, preprocessing pipeline optimization
//! - **Performance Profiling**: Training speed analysis, bottleneck identification
//! - **Resource Management**: Memory allocation, compute optimization
//!
//! ## Example
//!
//! ```no_run
//! use hecate_ml::{MLOptimizer, OptimizationConfig, WorkloadType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let optimizer = MLOptimizer::new().await?;
//!     
//!     // Detect available ML frameworks
//!     let frameworks = optimizer.detect_frameworks().await?;
//!     println!("Found frameworks: {:?}", frameworks);
//!     
//!     // Optimize for a training workload
//!     let config = OptimizationConfig {
//!         workload_type: WorkloadType::Training,
//!         model_size: Some(1_000_000_000), // 1B parameters
//!         use_mixed_precision: true,
//!         optimize_memory: true,
//!         enable_distributed: true,
//!         ..Default::default()
//!     };
//!     
//!     let recommendations = optimizer.optimize_workload(&config).await?;
//!     println!("Optimization recommendations: {:?}", recommendations);
//!     
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

pub mod error;
pub mod frameworks;
pub mod optimization;
pub mod distributed;
pub mod dataset;
pub mod profiling;

pub use error::{MLError, Result};
pub use frameworks::{FrameworkInfo as FrameworkInfoInternal, FrameworkType as InternalFrameworkType};
pub use optimization::SystemInfo as SystemInfoInternal;

// ============================================================================
// CORE DATA STRUCTURES
// ============================================================================

/// Main ML workload optimizer
#[derive(Debug)]
pub struct MLOptimizer {
    /// Detected ML frameworks
    frameworks: Arc<RwLock<Vec<FrameworkInfo>>>,
    /// System specifications
    system_info: Arc<RwLock<SystemInfo>>,
    /// Optimization cache
    optimization_cache: Arc<RwLock<HashMap<String, OptimizationResult>>>,
    /// Performance profiler
    profiler: Arc<RwLock<profiling::Profiler>>,
}

/// Information about detected ML frameworks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameworkInfo {
    pub name: String,
    pub version: String,
    pub framework_type: FrameworkType,
    pub installation_path: PathBuf,
    pub cuda_support: bool,
    pub distributed_support: bool,
    pub mixed_precision_support: bool,
    pub capabilities: Vec<String>,
}

/// Supported ML framework types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FrameworkType {
    PyTorch,
    TensorFlow,
    ONNX,
    HuggingFace,
    JAX,
    MXNet,
    Unknown,
}


/// Convert between external and internal FrameworkType
impl From<InternalFrameworkType> for FrameworkType {
    fn from(internal: InternalFrameworkType) -> Self {
        match internal {
            InternalFrameworkType::PyTorch => FrameworkType::PyTorch,
            InternalFrameworkType::TensorFlow => FrameworkType::TensorFlow,
            InternalFrameworkType::ONNX => FrameworkType::ONNX,
            InternalFrameworkType::JAX => FrameworkType::JAX,
            InternalFrameworkType::MXNet => FrameworkType::MXNet,
            InternalFrameworkType::HuggingFace => FrameworkType::HuggingFace,
        }
    }
}

impl From<FrameworkType> for InternalFrameworkType {
    fn from(external: FrameworkType) -> Self {
        match external {
            FrameworkType::PyTorch => InternalFrameworkType::PyTorch,
            FrameworkType::TensorFlow => InternalFrameworkType::TensorFlow,
            FrameworkType::ONNX => InternalFrameworkType::ONNX,
            FrameworkType::JAX => InternalFrameworkType::JAX,
            FrameworkType::MXNet => InternalFrameworkType::MXNet,
            FrameworkType::HuggingFace => InternalFrameworkType::HuggingFace,
            FrameworkType::Unknown => InternalFrameworkType::PyTorch, // Default fallback
        }
    }
}

/// System hardware information relevant to ML workloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub cpu_cores: u32,
    pub memory_total: u64,
    pub memory_available: u64,
    pub gpu_count: u32,
    pub gpu_memory_total: u64,
    pub storage_type: StorageType,
    pub network_bandwidth: Option<u64>,
}

/// Storage type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    Network,
}

/// Convert between external and internal SystemInfo
impl From<SystemInfoInternal> for SystemInfo {
    fn from(internal: SystemInfoInternal) -> Self {
        Self {
            cpu_cores: internal.cpu_cores,
            memory_total: internal.total_memory,
            memory_available: internal.available_memory,
            gpu_count: internal.gpu_count,
            gpu_memory_total: internal.gpu_memory.iter().sum(),
            storage_type: match internal.storage_type {
                optimization::StorageType::HDD => StorageType::HDD,
                optimization::StorageType::SSD => StorageType::SSD,
                optimization::StorageType::NVMe => StorageType::NVMe,
                optimization::StorageType::RAM => StorageType::SSD, // Map to closest equivalent
                optimization::StorageType::Network => StorageType::Network,
            },
            network_bandwidth: internal.network_bandwidth,
        }
    }
}

impl From<SystemInfo> for SystemInfoInternal {
    fn from(external: SystemInfo) -> Self {
        Self {
            cpu_cores: external.cpu_cores,
            total_memory: external.memory_total,
            available_memory: external.memory_available,
            gpu_count: external.gpu_count,
            gpu_memory: if external.gpu_count > 0 {
                vec![external.gpu_memory_total / external.gpu_count as u64; external.gpu_count as usize]
            } else {
                vec![]
            },
            storage_type: match external.storage_type {
                StorageType::HDD => optimization::StorageType::HDD,
                StorageType::SSD => optimization::StorageType::SSD,
                StorageType::NVMe => optimization::StorageType::NVMe,
                StorageType::Network => optimization::StorageType::Network,
            },
            network_bandwidth: external.network_bandwidth,
        }
    }
}

/// ML workload optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub workload_type: WorkloadType,
    pub model_size: Option<u64>, // Parameters count
    pub batch_size: Option<u32>,
    pub sequence_length: Option<u32>,
    pub use_mixed_precision: bool,
    pub optimize_memory: bool,
    pub enable_distributed: bool,
    pub target_framework: Option<FrameworkType>,
    pub max_memory_usage: Option<f32>, // Percentage of available memory
    pub target_throughput: Option<f32>, // Samples per second
    pub latency_requirement: Option<Duration>,
    pub dataset_size: Option<u64>,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            workload_type: WorkloadType::Training,
            model_size: None,
            batch_size: None,
            sequence_length: None,
            use_mixed_precision: true,
            optimize_memory: true,
            enable_distributed: false,
            target_framework: None,
            max_memory_usage: Some(0.8), // 80% of available memory
            target_throughput: None,
            latency_requirement: None,
            dataset_size: None,
        }
    }
}

/// Types of ML workloads
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash)]
pub enum WorkloadType {
    Training,
    Inference,
    FineTuning,
    Evaluation,
    DataPreprocessing,
    HyperparameterTuning,
}

/// Optimization result with recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub config: OptimizationConfig,
    pub recommendations: Vec<Recommendation>,
    pub estimated_performance: PerformanceEstimate,
    pub resource_allocation: ResourceAllocation,
    pub environment_variables: HashMap<String, String>,
    pub command_line_args: Vec<String>,
    pub warnings: Vec<String>,
}

/// Individual optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub impact: Impact,
    pub implementation: Implementation,
    pub confidence: f32, // 0.0 - 1.0
}

/// Recommendation categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    BatchSize,
    Memory,
    Compute,
    Storage,
    Network,
    Framework,
    Environment,
}

/// Expected impact of a recommendation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Impact {
    Low,
    Medium,
    High,
    Critical,
}

/// How to implement a recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Implementation {
    EnvironmentVariable { key: String, value: String },
    CommandLineArg { arg: String },
    ConfigFile { path: PathBuf, content: String },
    CodeChange { description: String, example: Option<String> },
    SystemSetting { description: String },
}

/// Performance estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEstimate {
    pub throughput_estimate: f32, // Samples/second
    pub memory_usage_estimate: u64, // Bytes
    pub training_time_estimate: Option<Duration>,
    pub gpu_utilization_estimate: f32, // 0.0 - 1.0
    pub bottleneck_analysis: Vec<String>,
}

/// Resource allocation recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub cpu_allocation: CPUAllocation,
    pub memory_allocation: MemoryAllocation,
    pub gpu_allocation: Option<GPUAllocation>,
    pub storage_allocation: StorageAllocation,
}

/// CPU resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUAllocation {
    pub worker_threads: u32,
    pub dataloader_workers: u32,
    pub cpu_affinity: Option<Vec<u32>>,
}

/// Memory allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    pub heap_size: Option<u64>,
    pub cache_size: u64,
    pub buffer_size: u64,
    pub use_memory_mapping: bool,
}

/// GPU allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUAllocation {
    pub gpu_ids: Vec<u32>,
    pub memory_fraction: f32,
    pub allow_growth: bool,
    pub distributed_strategy: Option<String>,
}

/// Storage allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageAllocation {
    pub cache_directory: PathBuf,
    pub tmp_directory: PathBuf,
    pub prefetch_buffer: u64,
    pub use_ssd_cache: bool,
}

// ============================================================================
// OPTIMIZATION PROFILES
// ============================================================================

/// Predefined optimization profiles for common scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationProfile {
    /// Maximize training speed
    FastTraining,
    /// Minimize memory usage
    MemoryEfficient,
    /// Balance speed and memory
    Balanced,
    /// Optimize for inference latency
    LowLatency,
    /// Maximize throughput for inference
    HighThroughput,
    /// Multi-node distributed training
    Distributed,
    /// Development and debugging
    Development,
}

impl OptimizationProfile {
    /// Convert profile to optimization config
    pub fn to_config(&self) -> OptimizationConfig {
        match self {
            Self::FastTraining => OptimizationConfig {
                workload_type: WorkloadType::Training,
                use_mixed_precision: true,
                optimize_memory: false,
                enable_distributed: false,
                max_memory_usage: Some(0.95),
                ..Default::default()
            },
            Self::MemoryEfficient => OptimizationConfig {
                workload_type: WorkloadType::Training,
                use_mixed_precision: true,
                optimize_memory: true,
                max_memory_usage: Some(0.6),
                ..Default::default()
            },
            Self::Balanced => OptimizationConfig {
                workload_type: WorkloadType::Training,
                use_mixed_precision: true,
                optimize_memory: true,
                max_memory_usage: Some(0.8),
                ..Default::default()
            },
            Self::LowLatency => OptimizationConfig {
                workload_type: WorkloadType::Inference,
                batch_size: Some(1),
                use_mixed_precision: false, // Prioritize accuracy over speed
                optimize_memory: false,
                ..Default::default()
            },
            Self::HighThroughput => OptimizationConfig {
                workload_type: WorkloadType::Inference,
                use_mixed_precision: true,
                optimize_memory: false,
                max_memory_usage: Some(0.95),
                ..Default::default()
            },
            Self::Distributed => OptimizationConfig {
                workload_type: WorkloadType::Training,
                enable_distributed: true,
                use_mixed_precision: true,
                optimize_memory: true,
                ..Default::default()
            },
            Self::Development => OptimizationConfig {
                workload_type: WorkloadType::Training,
                batch_size: Some(2), // Small batch for debugging
                use_mixed_precision: false,
                optimize_memory: false,
                enable_distributed: false,
                max_memory_usage: Some(0.5),
                ..Default::default()
            },
        }
    }
}

// ============================================================================
// MAIN OPTIMIZER IMPLEMENTATION
// ============================================================================

impl MLOptimizer {
    /// Create a new ML optimizer
    pub async fn new() -> Result<Self> {
        info!("Initializing HecateOS ML Optimizer");

        let system_info = Self::gather_system_info().await?;
        let profiler_config = profiling::ProfilingConfig::default();
        let profiler = profiling::Profiler::new(profiler_config);

        Ok(Self {
            frameworks: Arc::new(RwLock::new(Vec::new())),
            system_info: Arc::new(RwLock::new(system_info)),
            optimization_cache: Arc::new(RwLock::new(HashMap::new())),
            profiler: Arc::new(RwLock::new(profiler)),
        })
    }

    /// Gather system information
    async fn gather_system_info() -> Result<SystemInfo> {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_cores = sys.cpus().len() as u32;
        let memory_total = sys.total_memory();
        let memory_available = sys.available_memory();

        // GPU information would come from hecate-gpu integration
        let gpu_count = 0; // Placeholder
        let gpu_memory_total = 0; // Placeholder

        // Detect storage type
        let storage_type = Self::detect_storage_type().await?;

        // Network bandwidth detection (placeholder)
        let network_bandwidth = None;

        Ok(SystemInfo {
            cpu_cores,
            memory_total,
            memory_available,
            gpu_count,
            gpu_memory_total,
            storage_type,
            network_bandwidth,
        })
    }

    /// Detect primary storage type
    async fn detect_storage_type() -> Result<StorageType> {
        // This would analyze /proc/diskstats, /sys/block, etc.
        // For now, assume SSD as it's most common in modern systems
        Ok(StorageType::SSD)
    }

    /// Detect available ML frameworks
    #[instrument]
    pub async fn detect_frameworks(&self) -> Result<Vec<FrameworkInfo>> {
        info!("Detecting available ML frameworks");

        let mut detected_frameworks = Vec::new();

        // Detect PyTorch
        #[cfg(feature = "pytorch")]
        if let Ok(pytorch_info) = frameworks::detect_pytorch().await {
            detected_frameworks.push(Self::convert_framework_info(pytorch_info));
        }

        // Detect TensorFlow
        #[cfg(feature = "tensorflow")]
        if let Ok(tensorflow_info) = frameworks::detect_tensorflow().await {
            detected_frameworks.push(Self::convert_framework_info(tensorflow_info));
        }

        // Detect ONNX
        #[cfg(feature = "onnx")]
        if let Ok(onnx_info) = frameworks::detect_onnx().await {
            detected_frameworks.push(Self::convert_framework_info(onnx_info));
        }

        // Detect Hugging Face
        #[cfg(feature = "huggingface")]
        if let Ok(hf_info) = frameworks::detect_huggingface().await {
            detected_frameworks.push(Self::convert_framework_info(hf_info));
        }

        // Fallback: detect common frameworks without features
        if detected_frameworks.is_empty() {
            detected_frameworks.extend(self.detect_frameworks_fallback().await?);
        }

        // Update internal state
        let mut frameworks = self.frameworks.write().await;
        *frameworks = detected_frameworks.clone();

        info!("Detected {} ML framework(s)", detected_frameworks.len());
        Ok(detected_frameworks)
    }

    /// Convert internal FrameworkInfo to external
    fn convert_framework_info(internal: FrameworkInfoInternal) -> FrameworkInfo {
        FrameworkInfo {
            name: match internal.framework_type {
                InternalFrameworkType::PyTorch => "PyTorch".to_string(),
                InternalFrameworkType::TensorFlow => "TensorFlow".to_string(),
                InternalFrameworkType::ONNX => "ONNX".to_string(),
                InternalFrameworkType::JAX => "JAX".to_string(),
                InternalFrameworkType::MXNet => "MXNet".to_string(),
                InternalFrameworkType::HuggingFace => "Hugging Face".to_string(),
            },
            version: internal.version,
            framework_type: internal.framework_type.into(),
            installation_path: PathBuf::from(&internal.path),
            cuda_support: internal.features.contains(&"cuda".to_string()),
            distributed_support: internal.features.contains(&"distributed".to_string()),
            mixed_precision_support: internal.features.contains(&"amp".to_string()) || 
                                   internal.features.contains(&"mixed_precision".to_string()),
            capabilities: internal.features,
        }
    }

    /// Fallback framework detection
    async fn detect_frameworks_fallback(&self) -> Result<Vec<FrameworkInfo>> {
        let mut frameworks = Vec::new();
        
        // Try to detect PyTorch
        if let Ok(_) = tokio::process::Command::new("python3")
            .args(&["-c", "import torch; print(torch.__version__)"])
            .output()
            .await 
        {
            frameworks.push(FrameworkInfo {
                name: "PyTorch".to_string(),
                version: "detected".to_string(),
                framework_type: FrameworkType::PyTorch,
                installation_path: PathBuf::from("/unknown"),
                cuda_support: false,
                distributed_support: false,
                mixed_precision_support: false,
                capabilities: vec![],
            });
        }
        
        Ok(frameworks)
    }

    /// Optimize a workload with the given configuration
    #[instrument]
    pub async fn optimize_workload(&self, config: &OptimizationConfig) -> Result<OptimizationResult> {
        info!("Optimizing workload: {:?}", config.workload_type);

        // Check cache first
        let cache_key = self.generate_cache_key(config);
        if let Some(cached_result) = self.get_cached_result(&cache_key).await {
            debug!("Using cached optimization result");
            return Ok(cached_result);
        }

        // Gather system information
        let system_info = self.system_info.read().await;
        let frameworks = self.frameworks.read().await;

        // Convert to internal types
        let internal_system_info = (*system_info).clone().into();
        let internal_frameworks: Vec<FrameworkInfoInternal> = frameworks.iter()
            .map(|f| FrameworkInfoInternal {
                framework_type: f.framework_type.into(),
                version: f.version.clone(),
                path: f.installation_path.to_string_lossy().to_string(),
                features: f.capabilities.clone(),
                python_version: None, // Would be detected in real implementation
            })
            .collect();
        
        // Run optimization
        let optimizer = optimization::OptimizationEngine::new(internal_system_info);
        let opt_result = if !internal_frameworks.is_empty() {
            optimizer.optimize(&internal_frameworks[0], None, None)?
        } else {
            // Create a default result if no frameworks detected
            optimization::OptimizationResult {
                framework: InternalFrameworkType::PyTorch,
                model_name: None,
                dataset_info: None,
                recommendations: vec![],
                estimated_speedup: 1.0,
                memory_savings: None,
                energy_savings: None,
                timestamp: chrono::Utc::now(),
            }
        };
        
        // Convert optimization result to our format
        let mut result = self.convert_optimization_result(opt_result, config)?;

        // Add framework-specific optimizations
        self.apply_framework_optimizations(config, &mut result).await?;

        // Validate recommendations
        self.validate_recommendations(&mut result).await?;

        // Cache the result
        self.cache_result(cache_key, &result).await;

        info!("Optimization completed with {} recommendations", result.recommendations.len());
        Ok(result)
    }

    /// Generate a cache key for the given configuration
    fn generate_cache_key(&self, config: &OptimizationConfig) -> String {
        let mut hasher = DefaultHasher::new();
        
        // Hash relevant config fields
        config.workload_type.hash(&mut hasher);
        config.model_size.hash(&mut hasher);
        config.batch_size.hash(&mut hasher);
        config.use_mixed_precision.hash(&mut hasher);
        config.optimize_memory.hash(&mut hasher);
        config.enable_distributed.hash(&mut hasher);
        config.target_framework.hash(&mut hasher);
        
        format!("opt_{:x}", hasher.finish())
    }

    /// Get cached optimization result
    async fn get_cached_result(&self, cache_key: &str) -> Option<OptimizationResult> {
        let cache = self.optimization_cache.read().await;
        cache.get(cache_key).cloned()
    }

    /// Cache an optimization result
    async fn cache_result(&self, cache_key: String, result: &OptimizationResult) {
        let mut cache = self.optimization_cache.write().await;
        cache.insert(cache_key, result.clone());
    }

    /// Convert optimization result from internal format
    fn convert_optimization_result(
        &self, 
        opt_result: optimization::OptimizationResult, 
        config: &OptimizationConfig
    ) -> Result<OptimizationResult> {
        let recommendations: Vec<Recommendation> = opt_result.recommendations.iter()
            .map(|rec| Recommendation {
                category: match rec.optimization_type {
                    optimization::OptimizationType::BatchSize => RecommendationCategory::BatchSize,
                    optimization::OptimizationType::Memory => RecommendationCategory::Memory,
                    optimization::OptimizationType::Mixed => RecommendationCategory::Framework,
                    _ => RecommendationCategory::Framework,
                },
                title: rec.description.clone(),
                description: rec.rationale.clone(),
                impact: if rec.expected_improvement > 20.0 {
                    Impact::High
                } else if rec.expected_improvement > 10.0 {
                    Impact::Medium
                } else {
                    Impact::Low
                },
                implementation: Implementation::CodeChange {
                    description: format!("Set {} to {}", rec.parameter, rec.recommended_value),
                    example: Some(format!("{}={}", rec.parameter, rec.recommended_value)),
                },
                confidence: rec.confidence as f32,
            })
            .collect();

        Ok(OptimizationResult {
            config: config.clone(),
            recommendations,
            estimated_performance: PerformanceEstimate {
                throughput_estimate: opt_result.estimated_speedup as f32,
                memory_usage_estimate: opt_result.memory_savings.unwrap_or(0),
                training_time_estimate: None,
                gpu_utilization_estimate: 0.8, // Default estimate
                bottleneck_analysis: vec![],
            },
            resource_allocation: self.create_default_resource_allocation(),
            environment_variables: HashMap::new(),
            command_line_args: vec![],
            warnings: vec![],
        })
    }

    /// Create default resource allocation
    fn create_default_resource_allocation(&self) -> ResourceAllocation {
        ResourceAllocation {
            cpu_allocation: CPUAllocation {
                worker_threads: num_cpus::get() as u32,
                dataloader_workers: (num_cpus::get() / 2).max(1) as u32,
                cpu_affinity: None,
            },
            memory_allocation: MemoryAllocation {
                heap_size: None,
                cache_size: 1024 * 1024 * 1024, // 1GB
                buffer_size: 64 * 1024 * 1024,  // 64MB
                use_memory_mapping: true,
            },
            gpu_allocation: None, // Would be set if GPUs detected
            storage_allocation: StorageAllocation {
                cache_directory: PathBuf::from("/tmp/ml_cache"),
                tmp_directory: PathBuf::from("/tmp"),
                prefetch_buffer: 128 * 1024 * 1024, // 128MB
                use_ssd_cache: true,
            },
        }
    }

    /// Apply framework-specific optimizations
    async fn apply_framework_optimizations(
        &self,
        config: &OptimizationConfig,
        result: &mut OptimizationResult,
    ) -> Result<()> {
        let frameworks = self.frameworks.read().await;

        for framework in frameworks.iter() {
            match framework.framework_type {
                FrameworkType::PyTorch | FrameworkType::TensorFlow => {
                    // Apply generic optimizations for these frameworks
                    self.apply_generic_optimizations(config, result, framework).await?;
                }
                _ => {
                    // Generic optimizations for other frameworks
                    self.apply_generic_optimizations(config, result, framework).await?;
                }
            }
        }

        Ok(())
    }

    /// Apply generic optimizations for any framework
    async fn apply_generic_optimizations(
        &self,
        config: &OptimizationConfig,
        result: &mut OptimizationResult,
        framework: &FrameworkInfo,
    ) -> Result<()> {
        // Add generic environment variables
        result.environment_variables.insert(
            "OMP_NUM_THREADS".to_string(),
            result.resource_allocation.cpu_allocation.worker_threads.to_string(),
        );

        // Add memory optimization if supported
        if config.optimize_memory {
            result.recommendations.push(Recommendation {
                category: RecommendationCategory::Memory,
                title: "Enable memory optimization".to_string(),
                description: format!("Use memory optimization features for {}", framework.name),
                impact: Impact::Medium,
                implementation: Implementation::EnvironmentVariable {
                    key: format!("{}_MEMORY_OPTIMIZATION", framework.name.to_uppercase()),
                    value: "1".to_string(),
                },
                confidence: 0.7,
            });
        }

        Ok(())
    }

    /// Validate and adjust recommendations
    async fn validate_recommendations(&self, result: &mut OptimizationResult) -> Result<()> {
        let system_info = self.system_info.read().await;

        // Check if recommended batch size is feasible
        if let Some(batch_size) = result.config.batch_size {
            let estimated_memory = self.estimate_memory_usage(batch_size, &result.config)?;
            let available_memory = (system_info.memory_available as f64 * 
                                   result.config.max_memory_usage.unwrap_or(0.8) as f64) as u64;

            if estimated_memory > available_memory {
                result.warnings.push(format!(
                    "Recommended batch size {} may exceed available memory ({} > {})",
                    batch_size,
                    self.format_bytes(estimated_memory),
                    self.format_bytes(available_memory)
                ));

                // Suggest a smaller batch size
                let suggested_batch_size = (batch_size as f64 * 
                    (available_memory as f64 / estimated_memory as f64)) as u32;
                
                result.recommendations.push(Recommendation {
                    category: RecommendationCategory::BatchSize,
                    title: "Reduce batch size".to_string(),
                    description: format!("Reduce batch size to {} to fit in available memory", suggested_batch_size),
                    impact: Impact::High,
                    implementation: Implementation::CodeChange {
                        description: "Modify batch_size parameter in training loop".to_string(),
                        example: Some(format!("batch_size = {}", suggested_batch_size)),
                    },
                    confidence: 0.9,
                });
            }
        }

        Ok(())
    }

    /// Estimate memory usage for a given configuration
    fn estimate_memory_usage(&self, batch_size: u32, config: &OptimizationConfig) -> Result<u64> {
        let base_memory = 1_024 * 1_024 * 1_024; // 1GB base
        let model_memory = config.model_size.unwrap_or(100_000_000) * 4; // 4 bytes per parameter
        let batch_memory = (batch_size as u64) * 
                          config.sequence_length.unwrap_or(512) as u64 * 
                          4 * 768; // Estimate based on typical transformer dimensions

        Ok(base_memory + model_memory + batch_memory)
    }

    /// Format bytes to human-readable string
    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// Optimize using a predefined profile
    pub async fn optimize_with_profile(&self, profile: OptimizationProfile) -> Result<OptimizationResult> {
        let config = profile.to_config();
        self.optimize_workload(&config).await
    }

    /// Get system information
    pub async fn get_system_info(&self) -> SystemInfo {
        self.system_info.read().await.clone()
    }

    /// Get detected frameworks
    pub async fn get_frameworks(&self) -> Vec<FrameworkInfo> {
        self.frameworks.read().await.clone()
    }

    /// Clear optimization cache
    pub async fn clear_cache(&self) {
        let mut cache = self.optimization_cache.write().await;
        cache.clear();
        info!("Optimization cache cleared");
    }

    /// Start performance profiling
    pub async fn start_profiling(&self) -> Result<()> {
        let mut profiler = self.profiler.write().await;
        profiler.start_profiling().await?;
        info!("Performance profiling started");
        Ok(())
    }

    /// Stop performance profiling and get results
    pub async fn stop_profiling(&self) -> Result<profiling::PerformanceSummary> {
        let mut profiler = self.profiler.write().await;
        profiler.stop_profiling();
        let result = profiler.get_performance_summary()?;
        info!("Performance profiling stopped");
        Ok(result)
    }

    /// Get current profiling bottlenecks
    pub async fn get_profiling_bottlenecks(&self) -> Vec<profiling::Bottleneck> {
        let profiler = self.profiler.read().await;
        profiler.get_bottlenecks().to_vec()
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Estimate optimal batch size for given constraints
pub fn estimate_optimal_batch_size(
    model_size: u64,
    available_memory: u64,
    sequence_length: u32,
    precision_bytes: u32,
) -> u32 {
    let model_memory = model_size * 4; // 4 bytes per parameter
    let available_for_batch = available_memory.saturating_sub(model_memory);
    
    let memory_per_sample = sequence_length as u64 * precision_bytes as u64 * 768; // Estimate
    let max_batch_size = (available_for_batch / memory_per_sample) as u32;
    
    // Round down to nearest power of 2 for efficiency
    if max_batch_size > 0 {
        1 << (31 - max_batch_size.leading_zeros() - 1)
    } else {
        1
    }
}

/// Calculate memory efficiency score
pub fn calculate_memory_efficiency(used_memory: u64, total_memory: u64) -> f32 {
    if total_memory == 0 {
        return 0.0;
    }
    
    let utilization = used_memory as f32 / total_memory as f32;
    
    // Optimal utilization is around 80-90%
    if utilization >= 0.8 && utilization <= 0.9 {
        1.0
    } else if utilization < 0.8 {
        utilization / 0.8
    } else {
        // Penalize over-utilization
        1.0 - ((utilization - 0.9) * 2.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_profile_conversion() {
        let profile = OptimizationProfile::FastTraining;
        let config = profile.to_config();
        
        assert_eq!(config.workload_type, WorkloadType::Training);
        assert!(config.use_mixed_precision);
        assert!(!config.optimize_memory);
        assert_eq!(config.max_memory_usage, Some(0.95));
    }

    #[test]
    fn test_batch_size_estimation() {
        let optimal_batch = estimate_optimal_batch_size(
            1_000_000_000, // 1B parameters
            8_589_934_592, // 8GB memory
            512,           // sequence length
            4,             // float32
        );
        
        assert!(optimal_batch > 0);
        assert!(optimal_batch.is_power_of_two());
    }

    #[test]
    fn test_memory_efficiency() {
        assert_eq!(calculate_memory_efficiency(8_589_934_592, 10_737_418_240), 1.0); // 80% utilization (optimal)
        assert_eq!(calculate_memory_efficiency(9_663_676_416, 10_737_418_240), 1.0); // 90% utilization (optimal)
        assert!(calculate_memory_efficiency(5_368_709_120, 10_737_418_240) < 1.0); // 50% utilization (sub-optimal)
    }

    #[tokio::test]
    async fn test_ml_optimizer_creation() {
        let optimizer = MLOptimizer::new().await;
        assert!(optimizer.is_ok());
    }

    #[test]
    fn test_framework_type_serialization() {
        let framework_type = FrameworkType::PyTorch;
        let serialized = serde_json::to_string(&framework_type).unwrap();
        let deserialized: FrameworkType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(framework_type, deserialized);
    }
}