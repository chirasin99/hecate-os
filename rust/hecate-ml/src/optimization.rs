//! ML optimization engine for performance recommendations

use crate::{
    error::{MLError, Result},
    frameworks::{FrameworkInfo, FrameworkType},
    dataset::DatasetInfo,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use std::time::Duration;

/// Optimization recommendation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    BatchSize,
    LearningRate,
    Optimizer,
    DataLoader,
    Model,
    Memory,
    Distributed,
    Mixed,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub optimization_type: OptimizationType,
    pub description: String,
    pub parameter: String,
    pub current_value: Option<String>,
    pub recommended_value: String,
    pub expected_improvement: f64, // Percentage improvement
    pub confidence: f64,           // 0.0 to 1.0
    pub rationale: String,
}

/// Optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub framework: FrameworkType,
    pub model_name: Option<String>,
    pub dataset_info: Option<DatasetInfo>,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub estimated_speedup: f64,
    pub memory_savings: Option<u64>, // bytes
    pub energy_savings: Option<f64>, // percentage
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// System resource information for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub cpu_cores: u32,
    pub total_memory: u64,
    pub available_memory: u64,
    pub gpu_count: u32,
    pub gpu_memory: Vec<u64>,
    pub storage_type: StorageType,
    pub network_bandwidth: Option<u64>, // Mbps
}

/// Storage type for optimization decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    RAM,
    Network,
}

/// Optimization engine
pub struct OptimizationEngine {
    system_info: SystemInfo,
    optimization_rules: HashMap<FrameworkType, Vec<OptimizationRule>>,
}

/// Optimization rule
#[derive(Debug, Clone)]
pub struct OptimizationRule {
    pub name: String,
    pub condition: fn(&SystemInfo, &FrameworkInfo, Option<&DatasetInfo>) -> bool,
    pub recommendation: fn(&SystemInfo, &FrameworkInfo, Option<&DatasetInfo>) -> OptimizationRecommendation,
    pub priority: u8, // 1-10, higher is more important
}

impl OptimizationEngine {
    /// Create a new optimization engine
    pub fn new(system_info: SystemInfo) -> Self {
        let mut engine = Self {
            system_info,
            optimization_rules: HashMap::new(),
        };
        
        engine.initialize_rules();
        engine
    }

    /// Initialize optimization rules for each framework
    fn initialize_rules(&mut self) {
        // PyTorch optimization rules
        let pytorch_rules = vec![
            OptimizationRule {
                name: "pytorch_batch_size".to_string(),
                condition: |sys, _fw, dataset| {
                    dataset.map_or(false, |d| d.size > 10000) && sys.gpu_count > 0
                },
                recommendation: |sys, _fw, dataset| {
                    let gpu_memory = sys.gpu_memory.first().unwrap_or(&0);
                    let recommended_batch = calculate_optimal_batch_size(*gpu_memory, dataset);
                    OptimizationRecommendation {
                        optimization_type: OptimizationType::BatchSize,
                        description: "Optimize batch size for GPU memory".to_string(),
                        parameter: "batch_size".to_string(),
                        current_value: None,
                        recommended_value: recommended_batch.to_string(),
                        expected_improvement: 15.0,
                        confidence: 0.85,
                        rationale: "Larger batch sizes utilize GPU memory more efficiently".to_string(),
                    }
                },
                priority: 9,
            },
            OptimizationRule {
                name: "pytorch_dataloader_workers".to_string(),
                condition: |sys, _fw, _dataset| sys.cpu_cores >= 4,
                recommendation: |sys, _fw, _dataset| {
                    let workers = (sys.cpu_cores / 2).min(8);
                    OptimizationRecommendation {
                        optimization_type: OptimizationType::DataLoader,
                        description: "Optimize DataLoader worker processes".to_string(),
                        parameter: "num_workers".to_string(),
                        current_value: Some("0".to_string()),
                        recommended_value: workers.to_string(),
                        expected_improvement: 25.0,
                        confidence: 0.9,
                        rationale: format!("Use {} workers to parallelize data loading", workers),
                    }
                },
                priority: 8,
            },
            OptimizationRule {
                name: "pytorch_amp".to_string(),
                condition: |sys, _fw, _dataset| {
                    sys.gpu_count > 0 && sys.gpu_memory.iter().any(|&mem| mem > 6_000_000_000)
                },
                recommendation: |_sys, _fw, _dataset| {
                    OptimizationRecommendation {
                        optimization_type: OptimizationType::Mixed,
                        description: "Enable Automatic Mixed Precision (AMP)".to_string(),
                        parameter: "enable_amp".to_string(),
                        current_value: Some("false".to_string()),
                        recommended_value: "true".to_string(),
                        expected_improvement: 30.0,
                        confidence: 0.8,
                        rationale: "AMP reduces memory usage and increases training speed".to_string(),
                    }
                },
                priority: 7,
            },
        ];

        // TensorFlow optimization rules
        let tensorflow_rules = vec![
            OptimizationRule {
                name: "tf_mixed_precision".to_string(),
                condition: |sys, _fw, _dataset| sys.gpu_count > 0,
                recommendation: |_sys, _fw, _dataset| {
                    OptimizationRecommendation {
                        optimization_type: OptimizationType::Mixed,
                        description: "Enable mixed precision training".to_string(),
                        parameter: "mixed_precision".to_string(),
                        current_value: None,
                        recommended_value: "mixed_float16".to_string(),
                        expected_improvement: 25.0,
                        confidence: 0.85,
                        rationale: "Mixed precision reduces memory usage and training time".to_string(),
                    }
                },
                priority: 8,
            },
            OptimizationRule {
                name: "tf_xla".to_string(),
                condition: |_sys, _fw, dataset| {
                    dataset.map_or(true, |d| d.size > 5000)
                },
                recommendation: |_sys, _fw, _dataset| {
                    OptimizationRecommendation {
                        optimization_type: OptimizationType::Optimizer,
                        description: "Enable XLA compilation".to_string(),
                        parameter: "jit_compile".to_string(),
                        current_value: Some("false".to_string()),
                        recommended_value: "true".to_string(),
                        expected_improvement: 20.0,
                        confidence: 0.7,
                        rationale: "XLA optimizes computation graphs for better performance".to_string(),
                    }
                },
                priority: 6,
            },
        ];

        self.optimization_rules.insert(FrameworkType::PyTorch, pytorch_rules);
        self.optimization_rules.insert(FrameworkType::TensorFlow, tensorflow_rules);
    }

    /// Generate optimization recommendations
    pub fn optimize(
        &self,
        framework: &FrameworkInfo,
        dataset_info: Option<&DatasetInfo>,
        model_name: Option<&str>,
    ) -> Result<OptimizationResult> {
        info!("Generating optimizations for {:?} framework", framework.framework_type);

        let rules = self.optimization_rules
            .get(&framework.framework_type)
            .ok_or_else(|| MLError::FrameworkNotFound(format!("{:?}", framework.framework_type)))?;

        let mut recommendations = Vec::new();

        // Apply optimization rules
        for rule in rules {
            if (rule.condition)(&self.system_info, framework, dataset_info) {
                let recommendation = (rule.recommendation)(&self.system_info, framework, dataset_info);
                recommendations.push(recommendation);
                debug!("Applied optimization rule: {}", rule.name);
            }
        }

        // Sort by priority and expected improvement
        recommendations.sort_by(|a, b| {
            b.expected_improvement
                .partial_cmp(&a.expected_improvement)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Calculate estimated speedup
        let estimated_speedup = recommendations
            .iter()
            .map(|r| r.expected_improvement / 100.0)
            .fold(1.0, |acc, improvement| acc * (1.0 + improvement));

        // Estimate memory savings
        let memory_savings = self.estimate_memory_savings(&recommendations);

        // Estimate energy savings
        let energy_savings = self.estimate_energy_savings(&recommendations);

        Ok(OptimizationResult {
            framework: framework.framework_type.clone(),
            model_name: model_name.map(|s| s.to_string()),
            dataset_info: dataset_info.cloned(),
            recommendations,
            estimated_speedup,
            memory_savings,
            energy_savings,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Estimate memory savings from recommendations
    fn estimate_memory_savings(&self, recommendations: &[OptimizationRecommendation]) -> Option<u64> {
        let mut total_savings = 0u64;
        let mut has_memory_optimizations = false;

        for rec in recommendations {
            match rec.optimization_type {
                OptimizationType::Mixed => {
                    // Mixed precision typically saves 50% memory
                    if let Some(&gpu_memory) = self.system_info.gpu_memory.first() {
                        total_savings += gpu_memory / 2;
                        has_memory_optimizations = true;
                    }
                }
                OptimizationType::BatchSize => {
                    // Larger batch sizes may use more memory, but more efficiently
                    has_memory_optimizations = true;
                }
                OptimizationType::Memory => {
                    has_memory_optimizations = true;
                }
                _ => {}
            }
        }

        if has_memory_optimizations {
            Some(total_savings)
        } else {
            None
        }
    }

    /// Estimate energy savings from recommendations
    fn estimate_energy_savings(&self, recommendations: &[OptimizationRecommendation]) -> Option<f64> {
        let energy_savings: f64 = recommendations
            .iter()
            .map(|r| match r.optimization_type {
                OptimizationType::Mixed => 20.0, // Mixed precision saves energy
                OptimizationType::Optimizer => 10.0, // Better optimizers reduce training time
                OptimizationType::BatchSize => 5.0, // Optimal batch size improves efficiency
                _ => 0.0,
            })
            .sum();

        if energy_savings > 0.0 {
            Some(energy_savings.min(50.0)) // Cap at 50% savings
        } else {
            None
        }
    }

    /// Update system information
    pub fn update_system_info(&mut self, system_info: SystemInfo) {
        self.system_info = system_info;
        info!("Updated system information for optimization engine");
    }

    /// Get current system information
    pub fn system_info(&self) -> &SystemInfo {
        &self.system_info
    }
}

/// Calculate optimal batch size based on GPU memory
fn calculate_optimal_batch_size(gpu_memory: u64, dataset_info: Option<&DatasetInfo>) -> u32 {
    // Conservative estimation: use 70% of GPU memory
    let usable_memory = (gpu_memory as f64 * 0.7) as u64;
    
    // Estimate memory per sample based on dataset info
    let memory_per_sample = dataset_info
        .map(|d| estimate_memory_per_sample(d))
        .unwrap_or(100_000_000); // 100MB default estimate

    let optimal_batch = (usable_memory / memory_per_sample).max(1) as u32;
    
    // Round to power of 2 for better GPU utilization
    optimal_batch.next_power_of_two().min(512) // Cap at 512
}

/// Estimate memory per sample
fn estimate_memory_per_sample(dataset_info: &DatasetInfo) -> u64 {
    match dataset_info.data_type.as_str() {
        "image" => {
            // Estimate based on image dimensions
            let pixels = dataset_info.dimensions.iter().product::<u32>() as u64;
            pixels * 4 * 4 // 4 bytes per channel, assume 4 channels with gradients
        }
        "text" => {
            // Estimate based on sequence length
            (*dataset_info.dimensions.first().unwrap_or(&512) * 8 * 4) as u64 // 8 bytes per token, 4 for gradients
        }
        "audio" => {
            // Estimate based on sample rate and length
            (*dataset_info.dimensions.first().unwrap_or(&16000) * 4 * 4) as u64 // 4 bytes per sample
        }
        _ => 50_000_000, // 50MB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::DatasetInfo;

    fn create_test_system_info() -> SystemInfo {
        SystemInfo {
            cpu_cores: 8,
            total_memory: 16_000_000_000,
            available_memory: 12_000_000_000,
            gpu_count: 1,
            gpu_memory: vec![8_000_000_000],
            storage_type: StorageType::SSD,
            network_bandwidth: Some(1000),
        }
    }

    fn create_test_framework_info() -> FrameworkInfo {
        FrameworkInfo {
            framework_type: FrameworkType::PyTorch,
            version: "2.0.0".to_string(),
            path: "/usr/local/lib/python3.9/site-packages/torch".to_string(),
            features: vec!["cuda".to_string(), "cudnn".to_string()],
            python_version: Some("3.9.0".to_string()),
        }
    }

    #[test]
    fn test_optimization_engine_creation() {
        let system_info = create_test_system_info();
        let engine = OptimizationEngine::new(system_info);
        assert!(engine.optimization_rules.contains_key(&FrameworkType::PyTorch));
        assert!(engine.optimization_rules.contains_key(&FrameworkType::TensorFlow));
    }

    #[test]
    fn test_pytorch_optimization() {
        let system_info = create_test_system_info();
        let engine = OptimizationEngine::new(system_info);
        let framework = create_test_framework_info();
        
        let dataset = DatasetInfo {
            name: "ImageNet".to_string(),
            size: 100000,
            data_type: "image".to_string(),
            dimensions: vec![224, 224, 3],
        };

        let result = engine.optimize(&framework, Some(&dataset), Some("ResNet50"));
        assert!(result.is_ok());
        
        let optimization = result.unwrap();
        assert!(!optimization.recommendations.is_empty());
        assert!(optimization.estimated_speedup > 1.0);
    }

    #[test]
    fn test_batch_size_calculation() {
        let gpu_memory = 8_000_000_000; // 8GB
        let dataset = DatasetInfo {
            name: "CIFAR-10".to_string(),
            size: 50000,
            data_type: "image".to_string(),
            dimensions: vec![32, 32, 3],
        };

        let batch_size = calculate_optimal_batch_size(gpu_memory, Some(&dataset));
        assert!(batch_size > 0);
        assert!(batch_size <= 512);
        assert_eq!(batch_size.count_ones(), 1); // Should be power of 2
    }

    #[test]
    fn test_memory_per_sample_estimation() {
        let image_dataset = DatasetInfo {
            name: "ImageNet".to_string(),
            size: 100000,
            data_type: "image".to_string(),
            dimensions: vec![224, 224, 3],
        };

        let memory = estimate_memory_per_sample(&image_dataset);
        assert!(memory > 0);
        
        // Should be reasonable for 224x224x3 image
        assert!(memory > 1_000_000); // > 1MB
        assert!(memory < 100_000_000); // < 100MB
    }
}