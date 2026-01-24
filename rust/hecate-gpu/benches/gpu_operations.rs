//! Benchmarks for GPU operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hecate_gpu::*;
use std::time::Duration;

// Helper function to create test GPU status
fn create_test_gpu(index: u32) -> GpuStatus {
    GpuStatus {
        index,
        name: format!("Benchmark GPU {}", index),
        vendor: GpuVendor::NVIDIA,
        gpu_type: GpuType::Discrete,
        temperature: 75,
        power_draw: 250,
        power_limit: 350,
        memory_used: 4_294_967_296, // 4GB
        memory_total: 12_884_901_888, // 12GB
        utilization_gpu: 60,
        utilization_memory: 50,
        fan_speed: Some(65),
        clock_graphics: 1755,
        clock_memory: 8001,
        driver_version: Some("525.105.17".to_string()),
        pci_info: PciInfo {
            domain: 0,
            bus: 1,
            device: 0,
            function: 0,
            vendor_id: 0x10DE,
            device_id: 0x2204,
        },
        power_state: PowerState::Active,
    }
}

fn benchmark_efficiency_calculation(c: &mut Criterion) {
    let gpu = create_test_gpu(0);
    
    c.bench_function("calculate_efficiency_score", |b| {
        b.iter(|| {
            black_box(calculate_efficiency_score(black_box(&gpu)))
        })
    });
}

fn benchmark_format_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_bytes");
    
    let test_sizes = [
        1024u64,
        1024 * 1024,
        1024 * 1024 * 1024,
        1024u64.pow(4),
    ];
    
    for &size in &test_sizes {
        group.bench_with_input(
            BenchmarkId::new("bytes", size),
            &size,
            |b, &size| {
                b.iter(|| format_bytes(black_box(size)))
            },
        );
    }
    
    group.finish();
}

fn benchmark_fan_curve_calculation(c: &mut Criterion) {
    let aggressive_curve = FanCurve::aggressive();
    let quiet_curve = FanCurve::quiet();
    
    let mut group = c.benchmark_group("fan_curve_calculation");
    
    group.bench_function("aggressive_curve", |b| {
        b.iter(|| {
            for temp in 30..90 {
                black_box(aggressive_curve.calculate_fan_speed(black_box(temp)));
            }
        })
    });
    
    group.bench_function("quiet_curve", |b| {
        b.iter(|| {
            for temp in 30..90 {
                black_box(quiet_curve.calculate_fan_speed(black_box(temp)));
            }
        })
    });
    
    group.finish();
}

fn benchmark_gpu_summary_generation(c: &mut Criterion) {
    let gpu = create_test_gpu(0);
    
    c.bench_function("gpu_summary", |b| {
        b.iter(|| {
            black_box(gpu_summary(black_box(&gpu)))
        })
    });
}

fn benchmark_load_balancer_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("load_balancer");
    
    // Create test GPUs with different utilizations
    let gpus = vec![
        GpuStatus {
            utilization_gpu: 20,
            temperature: 65,
            ..create_test_gpu(0)
        },
        GpuStatus {
            utilization_gpu: 80,
            temperature: 85,
            ..create_test_gpu(1)
        },
        GpuStatus {
            utilization_gpu: 45,
            temperature: 70,
            ..create_test_gpu(2)
        },
        GpuStatus {
            utilization_gpu: 10,
            temperature: 60,
            ..create_test_gpu(3)
        },
    ];
    
    group.bench_function("load_balancer_creation", |b| {
        b.iter(|| {
            black_box(load_balancer::LoadBalancer::new(black_box(gpus.clone())))
        })
    });
    
    let mut lb = load_balancer::LoadBalancer::new(gpus.clone());
    rt.block_on(lb.enable());
    rt.block_on(lb.update_gpu_status(gpus.clone()));
    
    group.bench_function("assignment_least_utilized", |b| {
        b.to_async(&rt).iter(|| async {
            lb.set_strategy(load_balancer::LoadBalanceStrategy::LeastUtilized);
            black_box(lb.assign_workload().await.unwrap())
        })
    });
    
    group.bench_function("assignment_thermal_optimized", |b| {
        b.to_async(&rt).iter(|| async {
            lb.set_strategy(load_balancer::LoadBalanceStrategy::ThermalOptimized);
            black_box(lb.assign_workload().await.unwrap())
        })
    });
    
    group.bench_function("assignment_power_efficient", |b| {
        b.to_async(&rt).iter(|| async {
            lb.set_strategy(load_balancer::LoadBalanceStrategy::PowerEfficient);
            black_box(lb.assign_workload().await.unwrap())
        })
    });
    
    group.finish();
}

fn benchmark_monitoring_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("monitoring");
    
    let (tx, _rx) = tokio::sync::broadcast::channel(1000);
    let mut monitor = monitor::GpuMonitor::new(tx);
    
    let test_gpu = create_test_gpu(0);
    
    group.bench_function("record_metrics", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(monitor.record_metrics(black_box(0), black_box(&test_gpu)).await.unwrap())
        })
    });
    
    // Add some history for trend analysis
    rt.block_on(async {
        for i in 0..100 {
            let gpu = GpuStatus {
                utilization_gpu: (i % 100) as u32,
                temperature: 70 + (i % 20) as u32,
                ..test_gpu.clone()
            };
            let _ = monitor.record_metrics(0, &gpu).await;
        }
    });
    
    group.bench_function("analyze_performance_trend", |b| {
        b.iter(|| {
            black_box(monitor.analyze_performance_trend(black_box(0), black_box(60)))
        })
    });
    
    group.bench_function("detect_anomalies", |b| {
        b.iter(|| {
            black_box(monitor.detect_anomalies(black_box(0), black_box(30)))
        })
    });
    
    group.finish();
}

fn benchmark_serialization(c: &mut Criterion) {
    let gpu = create_test_gpu(0);
    let config = GpuConfig::max_performance();
    
    let mut group = c.benchmark_group("serialization");
    
    group.bench_function("serialize_gpu_status", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(black_box(&gpu)).unwrap())
        })
    });
    
    group.bench_function("serialize_gpu_config", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(black_box(&config)).unwrap())
        })
    });
    
    let gpu_json = serde_json::to_string(&gpu).unwrap();
    let config_json = serde_json::to_string(&config).unwrap();
    
    group.bench_function("deserialize_gpu_status", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<GpuStatus>(black_box(&gpu_json)).unwrap())
        })
    });
    
    group.bench_function("deserialize_gpu_config", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<GpuConfig>(black_box(&config_json)).unwrap())
        })
    });
    
    group.finish();
}

fn benchmark_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    // Test memory allocation patterns
    group.bench_function("create_large_gpu_list", |b| {
        b.iter(|| {
            let gpus: Vec<GpuStatus> = (0..1000)
                .map(|i| create_test_gpu(i))
                .collect();
            black_box(gpus)
        })
    });
    
    // Test memory usage of historical data
    group.bench_function("historical_metrics_storage", |b| {
        b.iter(|| {
            let metrics: Vec<monitor::MetricsPoint> = (0..10000)
                .map(|i| monitor::MetricsPoint {
                    timestamp: i as u64,
                    temperature: 70 + (i % 20) as u32,
                    power_draw: 200 + (i % 100) as u32,
                    utilization_gpu: (i % 100) as u32,
                    utilization_memory: (i % 100) as u32,
                    memory_used: 4_294_967_296 + (i % 1000) as u64 * 1_000_000,
                    clock_graphics: 1500 + (i % 500) as u32,
                    clock_memory: 7000 + (i % 1000) as u32,
                    fan_speed: Some(50 + (i % 50) as u32),
                })
                .collect();
            black_box(metrics)
        })
    });
    
    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    group.bench_function("concurrent_efficiency_calculations", |b| {
        b.to_async(&rt).iter(|| async {
            let gpus: Vec<_> = (0..100).map(create_test_gpu).collect();
            
            let handles: Vec<_> = gpus
                .iter()
                .map(|gpu| {
                    let gpu = gpu.clone();
                    tokio::spawn(async move {
                        calculate_efficiency_score(&gpu)
                    })
                })
                .collect();
            
            let results: Vec<f32> = futures::future::join_all(handles)
                .await
                .into_iter()
                .map(|r| r.unwrap())
                .collect();
            
            black_box(results)
        })
    });
    
    group.finish();
}

// Custom criterion configuration
fn custom_criterion() -> Criterion {
    Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100)
        .warm_up_time(Duration::from_secs(3))
        .with_plots()
}

criterion_group!(
    name = gpu_benches;
    config = custom_criterion();
    targets = 
        benchmark_efficiency_calculation,
        benchmark_format_bytes,
        benchmark_fan_curve_calculation,
        benchmark_gpu_summary_generation,
        benchmark_load_balancer_operations,
        benchmark_monitoring_operations,
        benchmark_serialization,
        benchmark_memory_operations,
        benchmark_concurrent_operations
);

criterion_main!(gpu_benches);