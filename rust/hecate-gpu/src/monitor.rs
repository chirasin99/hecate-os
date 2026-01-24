//! GPU monitoring and alerting system

use crate::{error::Result, GpuEvent, GpuStatus};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tracing::{debug, info, instrument, warn};

/// GPU monitoring system
pub struct GpuMonitor {
    /// Historical metrics storage
    metrics_history: HashMap<u32, VecDeque<MetricsPoint>>,
    /// Alert configuration
    alert_config: AlertConfig,
    /// Event sender
    event_sender: broadcast::Sender<GpuEvent>,
    /// Monitoring statistics
    stats: MonitoringStats,
    /// Maximum history length per GPU
    max_history_length: usize,
}

/// Single metrics point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPoint {
    pub timestamp: u64,
    pub temperature: u32,
    pub power_draw: u32,
    pub utilization_gpu: u32,
    pub utilization_memory: u32,
    pub memory_used: u64,
    pub clock_graphics: u32,
    pub clock_memory: u32,
    pub fan_speed: Option<u32>,
}

impl From<&GpuStatus> for MetricsPoint {
    fn from(status: &GpuStatus) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            temperature: status.temperature,
            power_draw: status.power_draw,
            utilization_gpu: status.utilization_gpu,
            utilization_memory: status.utilization_memory,
            memory_used: status.memory_used,
            clock_graphics: status.clock_graphics,
            clock_memory: status.clock_memory,
            fan_speed: status.fan_speed,
        }
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub temperature_warning: u32,
    pub temperature_critical: u32,
    pub power_usage_warning: u32, // Percentage of power limit
    pub memory_usage_warning: u32, // Percentage of total memory
    pub utilization_sustained_threshold: u32,
    pub utilization_sustained_duration: Duration,
    pub enable_performance_alerts: bool,
    pub enable_thermal_alerts: bool,
    pub enable_power_alerts: bool,
    pub enable_memory_alerts: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            temperature_warning: 80,
            temperature_critical: 90,
            power_usage_warning: 90,
            memory_usage_warning: 85,
            utilization_sustained_threshold: 95,
            utilization_sustained_duration: Duration::from_secs(300), // 5 minutes
            enable_performance_alerts: true,
            enable_thermal_alerts: true,
            enable_power_alerts: true,
            enable_memory_alerts: true,
        }
    }
}

/// Monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStats {
    pub total_metrics_collected: u64,
    pub alerts_triggered: u64,
    pub uptime_seconds: u64,
    pub last_collection_time: Option<u64>,
    pub gpu_count: usize,
    pub average_collection_interval: f64,
}

impl Default for MonitoringStats {
    fn default() -> Self {
        Self {
            total_metrics_collected: 0,
            alerts_triggered: 0,
            uptime_seconds: 0,
            last_collection_time: None,
            gpu_count: 0,
            average_collection_interval: 0.0,
        }
    }
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub gpu_index: u32,
    pub period_minutes: u32,
    pub average_temperature: f32,
    pub peak_temperature: u32,
    pub average_utilization: f32,
    pub peak_utilization: u32,
    pub average_power: f32,
    pub peak_power: u32,
    pub efficiency_score: f32,
    pub trend_direction: TrendDirection,
}

/// Trend direction enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub gpu_index: u32,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub detected_at: u64,
    pub current_value: f64,
    pub expected_range: (f64, f64),
}

/// Types of anomalies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AnomalyType {
    TemperatureSpike,
    PowerDrop,
    UtilizationStuck,
    ClockDrift,
    MemoryLeak,
    PerformanceDegradation,
}

/// Anomaly severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl GpuMonitor {
    /// Create a new GPU monitor
    pub fn new(event_sender: broadcast::Sender<GpuEvent>) -> Self {
        Self {
            metrics_history: HashMap::new(),
            alert_config: AlertConfig::default(),
            event_sender,
            stats: MonitoringStats::default(),
            max_history_length: 1440, // 24 hours at 1-minute intervals
        }
    }

    /// Update alert configuration
    pub fn set_alert_config(&mut self, config: AlertConfig) {
        self.alert_config = config;
        info!("Alert configuration updated");
    }

    /// Record metrics for a GPU
    #[instrument(skip(self, status))]
    pub async fn record_metrics(&mut self, gpu_index: u32, status: &GpuStatus) -> Result<()> {
        let metrics_point = MetricsPoint::from(status);
        
        // Store metrics
        let history = self.metrics_history
            .entry(gpu_index)
            .or_insert_with(VecDeque::new);
            
        history.push_back(metrics_point.clone());
        
        // Maintain history size limit
        while history.len() > self.max_history_length {
            history.pop_front();
        }
        
        // Update statistics
        self.stats.total_metrics_collected += 1;
        self.stats.last_collection_time = Some(metrics_point.timestamp);
        self.stats.gpu_count = self.metrics_history.len();
        
        // Check for alerts
        self.check_alerts(gpu_index, status).await?;
        
        debug!("Recorded metrics for GPU {}", gpu_index);
        Ok(())
    }

    /// Check and trigger alerts based on current status
    async fn check_alerts(&mut self, gpu_index: u32, status: &GpuStatus) -> Result<()> {
        // Temperature alerts
        if self.alert_config.enable_thermal_alerts {
            if status.temperature >= self.alert_config.temperature_critical {
                self.send_alert(GpuEvent::TemperatureAlert {
                    gpu_index,
                    temperature: status.temperature,
                    threshold: self.alert_config.temperature_critical,
                }).await;
            } else if status.temperature >= self.alert_config.temperature_warning {
                self.send_alert(GpuEvent::TemperatureAlert {
                    gpu_index,
                    temperature: status.temperature,
                    threshold: self.alert_config.temperature_warning,
                }).await;
            }
        }

        // Power alerts
        if self.alert_config.enable_power_alerts {
            let power_percentage = (status.power_draw * 100) / status.power_limit.max(1);
            if power_percentage >= self.alert_config.power_usage_warning {
                self.send_alert(GpuEvent::PowerAlert {
                    gpu_index,
                    power_draw: status.power_draw,
                    power_limit: status.power_limit,
                }).await;
            }
        }

        // Memory alerts
        if self.alert_config.enable_memory_alerts {
            let memory_percentage = ((status.memory_used * 100) / status.memory_total.max(1)) as u32;
            if memory_percentage >= self.alert_config.memory_usage_warning {
                self.send_alert(GpuEvent::VramAlert {
                    gpu_index,
                    used_percent: memory_percentage,
                    threshold: self.alert_config.memory_usage_warning,
                }).await;
            }
        }

        // Performance alerts (sustained high utilization)
        if self.alert_config.enable_performance_alerts {
            self.check_sustained_utilization_alert(gpu_index, status).await?;
        }

        Ok(())
    }

    /// Check for sustained high utilization
    async fn check_sustained_utilization_alert(&mut self, gpu_index: u32, status: &GpuStatus) -> Result<()> {
        if let Some(history) = self.metrics_history.get(&gpu_index) {
            let threshold_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - self.alert_config.utilization_sustained_duration.as_secs();

            let sustained_high_util = history
                .iter()
                .rev()
                .take_while(|point| point.timestamp >= threshold_time)
                .all(|point| point.utilization_gpu >= self.alert_config.utilization_sustained_threshold);

            if sustained_high_util && history.len() >= 5 { // At least 5 data points
                self.send_alert(GpuEvent::PerformanceDegraded {
                    gpu_index,
                    expected_score: 0.7,
                    actual_score: status.utilization_gpu as f32 / 100.0,
                }).await;
            }
        }

        Ok(())
    }

    /// Send alert event
    async fn send_alert(&mut self, event: GpuEvent) {
        if let Err(e) = self.event_sender.send(event) {
            warn!("Failed to send alert event: {}", e);
        } else {
            self.stats.alerts_triggered += 1;
        }
    }

    /// Get historical metrics for a GPU
    pub fn get_metrics_history(&self, gpu_index: u32) -> Option<&VecDeque<MetricsPoint>> {
        self.metrics_history.get(&gpu_index)
    }

    /// Get metrics for a specific time range
    pub fn get_metrics_range(
        &self,
        gpu_index: u32,
        start_time: u64,
        end_time: u64,
    ) -> Vec<MetricsPoint> {
        if let Some(history) = self.metrics_history.get(&gpu_index) {
            history
                .iter()
                .filter(|point| point.timestamp >= start_time && point.timestamp <= end_time)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Analyze performance trends for a GPU
    #[instrument(skip(self))]
    pub fn analyze_performance_trend(&self, gpu_index: u32, period_minutes: u32) -> Option<PerformanceTrend> {
        let history = self.metrics_history.get(&gpu_index)?;
        
        let period_seconds = period_minutes as u64 * 60;
        let threshold_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - period_seconds;

        let recent_metrics: Vec<&MetricsPoint> = history
            .iter()
            .filter(|point| point.timestamp >= threshold_time)
            .collect();

        if recent_metrics.is_empty() {
            return None;
        }

        // Calculate averages and peaks
        let count = recent_metrics.len() as f32;
        let average_temperature = recent_metrics.iter().map(|m| m.temperature).sum::<u32>() as f32 / count;
        let peak_temperature = recent_metrics.iter().map(|m| m.temperature).max().unwrap_or(0);
        let average_utilization = recent_metrics.iter().map(|m| m.utilization_gpu).sum::<u32>() as f32 / count;
        let peak_utilization = recent_metrics.iter().map(|m| m.utilization_gpu).max().unwrap_or(0);
        let average_power = recent_metrics.iter().map(|m| m.power_draw).sum::<u32>() as f32 / count;
        let peak_power = recent_metrics.iter().map(|m| m.power_draw).max().unwrap_or(0);

        // Calculate efficiency score
        let efficiency_score = self.calculate_efficiency_score(&recent_metrics);

        // Determine trend direction
        let trend_direction = self.determine_trend_direction(&recent_metrics);

        Some(PerformanceTrend {
            gpu_index,
            period_minutes,
            average_temperature,
            peak_temperature,
            average_utilization,
            peak_utilization,
            average_power,
            peak_power,
            efficiency_score,
            trend_direction,
        })
    }

    /// Calculate efficiency score from metrics
    fn calculate_efficiency_score(&self, metrics: &[&MetricsPoint]) -> f32 {
        if metrics.is_empty() {
            return 0.0;
        }

        let avg_utilization = metrics.iter().map(|m| m.utilization_gpu).sum::<u32>() as f32 / metrics.len() as f32;
        let avg_temperature = metrics.iter().map(|m| m.temperature).sum::<u32>() as f32 / metrics.len() as f32;
        let avg_power = metrics.iter().map(|m| m.power_draw).sum::<u32>() as f32 / metrics.len() as f32;

        // Higher utilization is good, lower temperature and power are good
        let utilization_score = avg_utilization / 100.0;
        let thermal_score = (100.0 - avg_temperature.min(100.0)) / 100.0;
        let power_score = (300.0 - avg_power.min(300.0)) / 300.0; // Assuming 300W max

        (utilization_score + thermal_score + power_score) / 3.0
    }

    /// Determine performance trend direction
    fn determine_trend_direction(&self, metrics: &[&MetricsPoint]) -> TrendDirection {
        if metrics.len() < 5 {
            return TrendDirection::Unknown;
        }

        let half = metrics.len() / 2;
        let first_half = &metrics[..half];
        let second_half = &metrics[half..];

        let first_efficiency = self.calculate_efficiency_score(first_half);
        let second_efficiency = self.calculate_efficiency_score(second_half);

        let difference = second_efficiency - first_efficiency;

        if difference > 0.05 {
            TrendDirection::Improving
        } else if difference < -0.05 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        }
    }

    /// Detect anomalies in GPU behavior
    #[instrument(skip(self))]
    pub fn detect_anomalies(&self, gpu_index: u32, lookback_minutes: u32) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        
        let history = match self.metrics_history.get(&gpu_index) {
            Some(history) => history,
            None => return anomalies,
        };

        let lookback_seconds = lookback_minutes as u64 * 60;
        let threshold_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - lookback_seconds;

        let recent_metrics: Vec<&MetricsPoint> = history
            .iter()
            .filter(|point| point.timestamp >= threshold_time)
            .collect();

        if recent_metrics.len() < 10 {
            return anomalies; // Need sufficient data
        }

        // Temperature spike detection
        if let Some(anomaly) = self.detect_temperature_spike(&recent_metrics, gpu_index) {
            anomalies.push(anomaly);
        }

        // Power drop detection
        if let Some(anomaly) = self.detect_power_drop(&recent_metrics, gpu_index) {
            anomalies.push(anomaly);
        }

        // Utilization stuck detection
        if let Some(anomaly) = self.detect_utilization_stuck(&recent_metrics, gpu_index) {
            anomalies.push(anomaly);
        }

        // Clock drift detection
        if let Some(anomaly) = self.detect_clock_drift(&recent_metrics, gpu_index) {
            anomalies.push(anomaly);
        }

        anomalies
    }

    /// Detect temperature spikes
    fn detect_temperature_spike(&self, metrics: &[&MetricsPoint], gpu_index: u32) -> Option<Anomaly> {
        let temperatures: Vec<u32> = metrics.iter().map(|m| m.temperature).collect();
        let avg_temp = temperatures.iter().sum::<u32>() as f64 / temperatures.len() as f64;
        let max_temp = *temperatures.iter().max().unwrap() as f64;

        // Detect if max temperature is significantly higher than average
        if max_temp > avg_temp + 20.0 && max_temp > 85.0 {
            return Some(Anomaly {
                gpu_index,
                anomaly_type: AnomalyType::TemperatureSpike,
                severity: if max_temp > 95.0 {
                    AnomalySeverity::Critical
                } else if max_temp > 90.0 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                },
                description: format!("Temperature spike detected: {}°C (avg: {:.1}°C)", max_temp, avg_temp),
                detected_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                current_value: max_temp,
                expected_range: (avg_temp - 10.0, avg_temp + 10.0),
            });
        }

        None
    }

    /// Detect power drops
    fn detect_power_drop(&self, metrics: &[&MetricsPoint], gpu_index: u32) -> Option<Anomaly> {
        if metrics.len() < 20 {
            return None;
        }

        let recent_power: Vec<u32> = metrics.iter().rev().take(5).map(|m| m.power_draw).collect();
        let baseline_power: Vec<u32> = metrics.iter().take(10).map(|m| m.power_draw).collect();

        let recent_avg = recent_power.iter().sum::<u32>() as f64 / recent_power.len() as f64;
        let baseline_avg = baseline_power.iter().sum::<u32>() as f64 / baseline_power.len() as f64;

        // Detect significant power drop
        if baseline_avg > 100.0 && recent_avg < baseline_avg * 0.5 {
            return Some(Anomaly {
                gpu_index,
                anomaly_type: AnomalyType::PowerDrop,
                severity: AnomalySeverity::Medium,
                description: format!("Power drop detected: {:.1}W (expected: {:.1}W)", recent_avg, baseline_avg),
                detected_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                current_value: recent_avg,
                expected_range: (baseline_avg * 0.8, baseline_avg * 1.2),
            });
        }

        None
    }

    /// Detect stuck utilization
    fn detect_utilization_stuck(&self, metrics: &[&MetricsPoint], gpu_index: u32) -> Option<Anomaly> {
        if metrics.len() < 10 {
            return None;
        }

        let recent_utilizations: Vec<u32> = metrics.iter().rev().take(10).map(|m| m.utilization_gpu).collect();
        
        // Check if utilization has been exactly the same for too long
        let first_util = recent_utilizations[0];
        let all_same = recent_utilizations.iter().all(|&u| u == first_util);

        if all_same && (first_util == 0 || first_util == 100) {
            return Some(Anomaly {
                gpu_index,
                anomaly_type: AnomalyType::UtilizationStuck,
                severity: AnomalySeverity::Medium,
                description: format!("GPU utilization stuck at {}%", first_util),
                detected_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                current_value: first_util as f64,
                expected_range: (10.0, 90.0),
            });
        }

        None
    }

    /// Detect clock drift
    fn detect_clock_drift(&self, metrics: &[&MetricsPoint], gpu_index: u32) -> Option<Anomaly> {
        if metrics.len() < 20 {
            return None;
        }

        let graphics_clocks: Vec<u32> = metrics.iter().map(|m| m.clock_graphics).collect();
        let avg_clock = graphics_clocks.iter().sum::<u32>() as f64 / graphics_clocks.len() as f64;
        let min_clock = *graphics_clocks.iter().min().unwrap() as f64;

        // Detect significant clock drop
        if avg_clock > 1000.0 && min_clock < avg_clock * 0.7 {
            return Some(Anomaly {
                gpu_index,
                anomaly_type: AnomalyType::ClockDrift,
                severity: AnomalySeverity::Medium,
                description: format!("Clock drift detected: {:.0}MHz (expected: {:.0}MHz)", min_clock, avg_clock),
                detected_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                current_value: min_clock,
                expected_range: (avg_clock * 0.9, avg_clock * 1.1),
            });
        }

        None
    }

    /// Get monitoring statistics
    pub fn get_stats(&self) -> &MonitoringStats {
        &self.stats
    }

    /// Clear historical data for a GPU
    pub fn clear_history(&mut self, gpu_index: u32) {
        if let Some(history) = self.metrics_history.get_mut(&gpu_index) {
            history.clear();
            info!("Cleared history for GPU {}", gpu_index);
        }
    }

    /// Clear all historical data
    pub fn clear_all_history(&mut self) {
        self.metrics_history.clear();
        self.stats = MonitoringStats::default();
        info!("Cleared all GPU monitoring history");
    }

    /// Export metrics to JSON
    pub fn export_metrics(&self, gpu_index: u32) -> Result<String> {
        let history = self.metrics_history.get(&gpu_index)
            .ok_or_else(|| crate::error::GpuError::GpuNotFound(gpu_index))?;

        serde_json::to_string_pretty(history)
            .map_err(crate::error::GpuError::SerializationError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert_eq!(config.temperature_warning, 80);
        assert_eq!(config.temperature_critical, 90);
        assert!(config.enable_thermal_alerts);
    }

    #[test]
    fn test_metrics_point_creation() {
        let metrics = MetricsPoint {
            timestamp: 1000000000,
            temperature: 75,
            power_draw: 200,
            utilization_gpu: 80,
            utilization_memory: 70,
            memory_used: 4_294_967_296, // 4GB
            clock_graphics: 1500,
            clock_memory: 7000,
            fan_speed: Some(60),
        };
        
        assert_eq!(metrics.temperature, 75);
        assert_eq!(metrics.utilization_gpu, 80);
    }

    #[tokio::test]
    async fn test_monitor_creation() {
        let (tx, _rx) = broadcast::channel(100);
        let monitor = GpuMonitor::new(tx);
        
        assert_eq!(monitor.metrics_history.len(), 0);
        assert_eq!(monitor.stats.total_metrics_collected, 0);
    }

    #[test]
    fn test_anomaly_severity() {
        let anomaly = Anomaly {
            gpu_index: 0,
            anomaly_type: AnomalyType::TemperatureSpike,
            severity: AnomalySeverity::High,
            description: "Test anomaly".to_string(),
            detected_at: 1000000000,
            current_value: 95.0,
            expected_range: (70.0, 80.0),
        };
        
        assert_eq!(anomaly.severity, AnomalySeverity::High);
        assert_eq!(anomaly.anomaly_type, AnomalyType::TemperatureSpike);
    }
}