//! ML workload profiling and performance analysis

use crate::error::{MLError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{debug, info, warn, instrument};

/// Profiling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingMetrics {
    pub timestamp: u64,
    pub gpu_utilization: Vec<f32>,
    pub gpu_memory_usage: Vec<u64>,
    pub cpu_utilization: f32,
    pub memory_usage: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub training_metrics: TrainingMetrics,
}

/// Training-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub batch_time: Option<Duration>,
    pub forward_time: Option<Duration>,
    pub backward_time: Option<Duration>,
    pub optimizer_time: Option<Duration>,
    pub data_loading_time: Option<Duration>,
    pub loss: Option<f32>,
    pub learning_rate: Option<f32>,
    pub gradients_norm: Option<f32>,
}

/// Performance bottleneck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub bottleneck_type: BottleneckType,
    pub severity: BottleneckSeverity,
    pub description: String,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
    pub recommendation: String,
}

/// Types of performance bottlenecks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    GPU,
    CPU,
    Memory,
    IO,
    Network,
    DataLoading,
    ModelComputation,
}

/// Bottleneck severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    pub enabled: bool,
    pub sampling_interval: Duration,
    pub retention_period: Duration,
    pub detailed_timing: bool,
    pub memory_profiling: bool,
    pub network_profiling: bool,
}

/// Profiler implementation
#[derive(Debug)]
pub struct Profiler {
    config: ProfilingConfig,
    metrics_buffer: VecDeque<ProfilingMetrics>,
    bottlenecks: Vec<Bottleneck>,
    baseline_metrics: Option<ProfilingMetrics>,
    profiling_active: bool,
}

impl Profiler {
    /// Create new profiler
    pub fn new(config: ProfilingConfig) -> Self {
        Self {
            config,
            metrics_buffer: VecDeque::new(),
            bottlenecks: Vec::new(),
            baseline_metrics: None,
            profiling_active: false,
        }
    }

    /// Start profiling
    #[instrument]
    pub async fn start_profiling(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        info!("Starting ML workload profiling");
        self.profiling_active = true;
        
        // Clear previous data
        self.metrics_buffer.clear();
        self.bottlenecks.clear();

        // Start background profiling task
        let mut interval = interval(self.config.sampling_interval);
        
        while self.profiling_active {
            interval.tick().await;
            
            match self.collect_metrics().await {
                Ok(metrics) => {
                    self.store_metrics(metrics);
                    self.detect_bottlenecks().await?;
                }
                Err(e) => {
                    warn!("Failed to collect metrics: {}", e);
                }
            }
            
            // Clean old metrics
            self.cleanup_old_metrics();
        }

        Ok(())
    }

    /// Stop profiling
    pub fn stop_profiling(&mut self) {
        info!("Stopping ML workload profiling");
        self.profiling_active = false;
    }

    /// Collect current system metrics
    async fn collect_metrics(&self) -> Result<ProfilingMetrics> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Collect GPU metrics
        let gpu_metrics = self.collect_gpu_metrics().await?;
        
        // Collect CPU metrics
        let cpu_utilization = self.collect_cpu_utilization().await?;
        
        // Collect memory metrics
        let memory_usage = self.collect_memory_usage().await?;
        
        // Collect I/O metrics
        let (io_read, io_write) = self.collect_io_metrics().await?;
        
        // Collect network metrics
        let (net_rx, net_tx) = if self.config.network_profiling {
            self.collect_network_metrics().await?
        } else {
            (0, 0)
        };

        // Collect training metrics (would be updated by training loop)
        let training_metrics = TrainingMetrics {
            batch_time: None,
            forward_time: None,
            backward_time: None,
            optimizer_time: None,
            data_loading_time: None,
            loss: None,
            learning_rate: None,
            gradients_norm: None,
        };

        Ok(ProfilingMetrics {
            timestamp,
            gpu_utilization: gpu_metrics.0,
            gpu_memory_usage: gpu_metrics.1,
            cpu_utilization,
            memory_usage,
            io_read_bytes: io_read,
            io_write_bytes: io_write,
            network_rx_bytes: net_rx,
            network_tx_bytes: net_tx,
            training_metrics,
        })
    }

    /// Collect GPU metrics
    async fn collect_gpu_metrics(&self) -> Result<(Vec<f32>, Vec<u64>)> {
        // In a real implementation, this would interface with GPU monitoring APIs
        // For now, simulate some metrics
        
        let gpu_count = self.get_gpu_count().await?;
        let mut utilization = Vec::new();
        let mut memory_usage = Vec::new();

        for _i in 0..gpu_count {
            // Simulate GPU metrics - in practice would query actual GPU status
            utilization.push(50.0); // 50% utilization
            memory_usage.push(4_000_000_000); // 4GB usage
        }

        Ok((utilization, memory_usage))
    }

    /// Get GPU count
    async fn get_gpu_count(&self) -> Result<usize> {
        // Simulate GPU detection - in practice would query system
        Ok(1)
    }

    /// Collect CPU utilization
    async fn collect_cpu_utilization(&self) -> Result<f32> {
        // Read from /proc/stat or use system monitoring library
        // For now, return simulated value
        Ok(30.0) // 30% CPU utilization
    }

    /// Collect memory usage
    async fn collect_memory_usage(&self) -> Result<u64> {
        // Read from /proc/meminfo
        // For now, return simulated value
        Ok(8_000_000_000) // 8GB usage
    }

    /// Collect I/O metrics
    async fn collect_io_metrics(&self) -> Result<(u64, u64)> {
        // Read from /proc/diskstats
        // For now, return simulated values
        Ok((1_000_000, 500_000)) // 1MB read, 500KB write
    }

    /// Collect network metrics
    async fn collect_network_metrics(&self) -> Result<(u64, u64)> {
        // Read from /proc/net/dev
        // For now, return simulated values
        Ok((10_000_000, 5_000_000)) // 10MB RX, 5MB TX
    }

    /// Store metrics in buffer
    fn store_metrics(&mut self, metrics: ProfilingMetrics) {
        self.metrics_buffer.push_back(metrics);
        
        // Set baseline if first measurement
        if self.baseline_metrics.is_none() {
            self.baseline_metrics = self.metrics_buffer.back().cloned();
        }
    }

    /// Clean up old metrics
    fn cleanup_old_metrics(&mut self) {
        let retention_samples = (self.config.retention_period.as_secs() / 
                                self.config.sampling_interval.as_secs()) as usize;
        
        while self.metrics_buffer.len() > retention_samples {
            self.metrics_buffer.pop_front();
        }
    }

    /// Detect performance bottlenecks
    async fn detect_bottlenecks(&mut self) -> Result<()> {
        if let Some(latest_metrics) = self.metrics_buffer.back() {
            let mut new_bottlenecks = Vec::new();

            // Check GPU bottlenecks
            new_bottlenecks.extend(self.detect_gpu_bottlenecks(latest_metrics)?);
            
            // Check CPU bottlenecks
            new_bottlenecks.extend(self.detect_cpu_bottlenecks(latest_metrics)?);
            
            // Check memory bottlenecks
            new_bottlenecks.extend(self.detect_memory_bottlenecks(latest_metrics)?);
            
            // Check I/O bottlenecks
            new_bottlenecks.extend(self.detect_io_bottlenecks(latest_metrics)?);
            
            // Check data loading bottlenecks
            new_bottlenecks.extend(self.detect_data_loading_bottlenecks(latest_metrics)?);

            // Update bottlenecks list
            self.bottlenecks = new_bottlenecks;
        }

        Ok(())
    }

    /// Detect GPU bottlenecks
    fn detect_gpu_bottlenecks(&self, metrics: &ProfilingMetrics) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();

        // Check GPU utilization
        for (i, &utilization) in metrics.gpu_utilization.iter().enumerate() {
            if utilization < 50.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::GPU,
                    severity: if utilization < 20.0 { 
                        BottleneckSeverity::High 
                    } else { 
                        BottleneckSeverity::Medium 
                    },
                    description: format!("GPU {} utilization is low", i),
                    metric_name: format!("gpu_{}_utilization", i),
                    current_value: utilization as f64,
                    threshold: 80.0,
                    recommendation: "Increase batch size or check for CPU bottlenecks".to_string(),
                });
            }
        }

        // Check GPU memory usage
        for (i, &memory_usage) in metrics.gpu_memory_usage.iter().enumerate() {
            let memory_gb = memory_usage as f64 / 1_000_000_000.0;
            if memory_gb > 7.0 && memory_gb < 8.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::Memory,
                    severity: BottleneckSeverity::Medium,
                    description: format!("GPU {} memory usage is high", i),
                    metric_name: format!("gpu_{}_memory", i),
                    current_value: memory_gb,
                    threshold: 6.0,
                    recommendation: "Consider reducing batch size or model size".to_string(),
                });
            }
        }

        Ok(bottlenecks)
    }

    /// Detect CPU bottlenecks
    fn detect_cpu_bottlenecks(&self, metrics: &ProfilingMetrics) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();

        if metrics.cpu_utilization > 80.0 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::CPU,
                severity: if metrics.cpu_utilization > 95.0 {
                    BottleneckSeverity::High
                } else {
                    BottleneckSeverity::Medium
                },
                description: "CPU utilization is high".to_string(),
                metric_name: "cpu_utilization".to_string(),
                current_value: metrics.cpu_utilization as f64,
                threshold: 80.0,
                recommendation: "Consider using more data loading workers or GPU acceleration".to_string(),
            });
        }

        Ok(bottlenecks)
    }

    /// Detect memory bottlenecks
    fn detect_memory_bottlenecks(&self, metrics: &ProfilingMetrics) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();

        let memory_gb = metrics.memory_usage as f64 / 1_000_000_000.0;
        if memory_gb > 60.0 { // Assume 64GB total system memory
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::Memory,
                severity: if memory_gb > 62.0 {
                    BottleneckSeverity::High
                } else {
                    BottleneckSeverity::Medium
                },
                description: "System memory usage is high".to_string(),
                metric_name: "memory_usage".to_string(),
                current_value: memory_gb,
                threshold: 50.0,
                recommendation: "Reduce batch size or enable gradient checkpointing".to_string(),
            });
        }

        Ok(bottlenecks)
    }

    /// Detect I/O bottlenecks
    fn detect_io_bottlenecks(&self, metrics: &ProfilingMetrics) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();

        // Check if I/O is unusually high compared to baseline
        if let Some(ref baseline) = self.baseline_metrics {
            let read_ratio = metrics.io_read_bytes as f64 / baseline.io_read_bytes.max(1) as f64;
            let write_ratio = metrics.io_write_bytes as f64 / baseline.io_write_bytes.max(1) as f64;

            if read_ratio > 5.0 || write_ratio > 5.0 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::IO,
                    severity: BottleneckSeverity::Medium,
                    description: "I/O activity is unusually high".to_string(),
                    metric_name: "io_activity".to_string(),
                    current_value: read_ratio.max(write_ratio),
                    threshold: 3.0,
                    recommendation: "Consider caching data in memory or using faster storage".to_string(),
                });
            }
        }

        Ok(bottlenecks)
    }

    /// Detect data loading bottlenecks
    fn detect_data_loading_bottlenecks(&self, metrics: &ProfilingMetrics) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();

        // Check data loading time if available
        if let Some(data_loading_time) = metrics.training_metrics.data_loading_time {
            if let Some(batch_time) = metrics.training_metrics.batch_time {
                let loading_ratio = data_loading_time.as_millis() as f64 / batch_time.as_millis() as f64;
                
                if loading_ratio > 0.3 { // Data loading takes >30% of batch time
                    bottlenecks.push(Bottleneck {
                        bottleneck_type: BottleneckType::DataLoading,
                        severity: if loading_ratio > 0.5 {
                            BottleneckSeverity::High
                        } else {
                            BottleneckSeverity::Medium
                        },
                        description: "Data loading is slow relative to computation".to_string(),
                        metric_name: "data_loading_ratio".to_string(),
                        current_value: loading_ratio,
                        threshold: 0.2,
                        recommendation: "Increase data loading workers or use data caching".to_string(),
                    });
                }
            }
        }

        Ok(bottlenecks)
    }

    /// Get current bottlenecks
    pub fn get_bottlenecks(&self) -> &[Bottleneck] {
        &self.bottlenecks
    }

    /// Get recent metrics
    pub fn get_recent_metrics(&self, count: usize) -> Vec<&ProfilingMetrics> {
        self.metrics_buffer.iter()
            .rev()
            .take(count)
            .collect()
    }

    /// Get performance summary
    pub fn get_performance_summary(&self) -> Result<PerformanceSummary> {
        if self.metrics_buffer.is_empty() {
            return Err(MLError::ProfilingError("No metrics available".to_string()));
        }

        let recent_metrics: Vec<_> = self.get_recent_metrics(10);
        
        // Calculate averages
        let avg_gpu_util: f32 = recent_metrics.iter()
            .flat_map(|m| &m.gpu_utilization)
            .sum::<f32>() / recent_metrics.len() as f32;
            
        let avg_cpu_util: f32 = recent_metrics.iter()
            .map(|m| m.cpu_utilization)
            .sum::<f32>() / recent_metrics.len() as f32;

        let avg_memory_gb = recent_metrics.iter()
            .map(|m| m.memory_usage as f64 / 1_000_000_000.0)
            .sum::<f64>() / recent_metrics.len() as f64;

        // Count bottlenecks by severity
        let critical_bottlenecks = self.bottlenecks.iter()
            .filter(|b| matches!(b.severity, BottleneckSeverity::Critical))
            .count();
            
        let high_bottlenecks = self.bottlenecks.iter()
            .filter(|b| matches!(b.severity, BottleneckSeverity::High))
            .count();

        Ok(PerformanceSummary {
            avg_gpu_utilization: avg_gpu_util,
            avg_cpu_utilization: avg_cpu_util,
            avg_memory_usage_gb: avg_memory_gb,
            total_bottlenecks: self.bottlenecks.len(),
            critical_bottlenecks,
            high_bottlenecks,
            performance_score: self.calculate_performance_score(),
        })
    }

    /// Calculate overall performance score (0-100)
    fn calculate_performance_score(&self) -> f32 {
        let mut score = 100.0;

        // Penalize bottlenecks
        for bottleneck in &self.bottlenecks {
            let penalty = match bottleneck.severity {
                BottleneckSeverity::Critical => 30.0,
                BottleneckSeverity::High => 20.0,
                BottleneckSeverity::Medium => 10.0,
                BottleneckSeverity::Low => 5.0,
            };
            score -= penalty;
        }

        // Penalize low GPU utilization
        if let Some(latest) = self.metrics_buffer.back() {
            let avg_gpu_util = latest.gpu_utilization.iter().sum::<f32>() / latest.gpu_utilization.len() as f32;
            if avg_gpu_util < 50.0 {
                score -= (50.0 - avg_gpu_util) * 0.5;
            }
        }

        score.max(0.0).min(100.0)
    }

    /// Update training metrics
    pub fn update_training_metrics(&mut self, training_metrics: TrainingMetrics) {
        if let Some(latest) = self.metrics_buffer.back_mut() {
            latest.training_metrics = training_metrics;
        }
    }

    /// Export metrics to file
    pub fn export_metrics(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.metrics_buffer)
            .map_err(MLError::SerializationError)?;
        
        std::fs::write(path, json)
            .map_err(MLError::IoError)?;
        
        info!("Exported metrics to {}", path);
        Ok(())
    }
}

/// Performance summary
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub avg_gpu_utilization: f32,
    pub avg_cpu_utilization: f32,
    pub avg_memory_usage_gb: f64,
    pub total_bottlenecks: usize,
    pub critical_bottlenecks: usize,
    pub high_bottlenecks: usize,
    pub performance_score: f32,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sampling_interval: Duration::from_secs(1),
            retention_period: Duration::from_secs(3600), // 1 hour
            detailed_timing: false,
            memory_profiling: true,
            network_profiling: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let config = ProfilingConfig::default();
        let profiler = Profiler::new(config);
        assert!(!profiler.profiling_active);
        assert!(profiler.metrics_buffer.is_empty());
    }

    #[test]
    fn test_metrics_storage() {
        let config = ProfilingConfig::default();
        let mut profiler = Profiler::new(config);
        
        let metrics = ProfilingMetrics {
            timestamp: 1000,
            gpu_utilization: vec![50.0],
            gpu_memory_usage: vec![4_000_000_000],
            cpu_utilization: 30.0,
            memory_usage: 8_000_000_000,
            io_read_bytes: 1000,
            io_write_bytes: 500,
            network_rx_bytes: 10000,
            network_tx_bytes: 5000,
            training_metrics: TrainingMetrics {
                batch_time: Some(Duration::from_millis(100)),
                forward_time: Some(Duration::from_millis(50)),
                backward_time: Some(Duration::from_millis(30)),
                optimizer_time: Some(Duration::from_millis(20)),
                data_loading_time: Some(Duration::from_millis(10)),
                loss: Some(0.5),
                learning_rate: Some(0.001),
                gradients_norm: Some(1.0),
            },
        };

        profiler.store_metrics(metrics);
        assert_eq!(profiler.metrics_buffer.len(), 1);
        assert!(profiler.baseline_metrics.is_some());
    }

    #[tokio::test]
    async fn test_bottleneck_detection() {
        let config = ProfilingConfig::default();
        let mut profiler = Profiler::new(config);
        
        // Create metrics with low GPU utilization (should trigger bottleneck)
        let metrics = ProfilingMetrics {
            timestamp: 1000,
            gpu_utilization: vec![10.0], // Low utilization
            gpu_memory_usage: vec![4_000_000_000],
            cpu_utilization: 30.0,
            memory_usage: 8_000_000_000,
            io_read_bytes: 1000,
            io_write_bytes: 500,
            network_rx_bytes: 10000,
            network_tx_bytes: 5000,
            training_metrics: TrainingMetrics {
                batch_time: None,
                forward_time: None,
                backward_time: None,
                optimizer_time: None,
                data_loading_time: None,
                loss: None,
                learning_rate: None,
                gradients_norm: None,
            },
        };

        profiler.store_metrics(metrics);
        profiler.detect_bottlenecks().await.unwrap();
        
        // Should detect GPU utilization bottleneck
        assert!(!profiler.bottlenecks.is_empty());
        assert!(profiler.bottlenecks.iter().any(|b| matches!(b.bottleneck_type, BottleneckType::GPU)));
    }

    #[test]
    fn test_performance_score_calculation() {
        let config = ProfilingConfig::default();
        let mut profiler = Profiler::new(config);
        
        // Add some bottlenecks
        profiler.bottlenecks.push(Bottleneck {
            bottleneck_type: BottleneckType::GPU,
            severity: BottleneckSeverity::High,
            description: "Test".to_string(),
            metric_name: "test".to_string(),
            current_value: 10.0,
            threshold: 50.0,
            recommendation: "Test".to_string(),
        });

        let score = profiler.calculate_performance_score();
        assert!(score < 100.0); // Should be penalized for bottleneck
        assert!(score >= 0.0);
    }

    #[test]
    fn test_metrics_cleanup() {
        let mut config = ProfilingConfig::default();
        config.retention_period = Duration::from_secs(2);
        config.sampling_interval = Duration::from_secs(1);
        
        let mut profiler = Profiler::new(config);
        
        // Add more metrics than retention allows
        for i in 0..5 {
            let metrics = ProfilingMetrics {
                timestamp: i,
                gpu_utilization: vec![50.0],
                gpu_memory_usage: vec![4_000_000_000],
                cpu_utilization: 30.0,
                memory_usage: 8_000_000_000,
                io_read_bytes: 1000,
                io_write_bytes: 500,
                network_rx_bytes: 10000,
                network_tx_bytes: 5000,
                training_metrics: TrainingMetrics {
                    batch_time: None,
                    forward_time: None,
                    backward_time: None,
                    optimizer_time: None,
                    data_loading_time: None,
                    loss: None,
                    learning_rate: None,
                    gradients_norm: None,
                },
            };
            profiler.store_metrics(metrics);
        }
        
        assert_eq!(profiler.metrics_buffer.len(), 5);
        
        profiler.cleanup_old_metrics();
        
        // Should keep only 2 metrics (retention_period / sampling_interval = 2)
        assert_eq!(profiler.metrics_buffer.len(), 2);
    }
}