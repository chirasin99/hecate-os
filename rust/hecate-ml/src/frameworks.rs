//! ML framework detection and integration

use crate::error::{MLError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn};
use which::which;

/// Framework types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FrameworkType {
    PyTorch,
    TensorFlow,
    ONNX,
    JAX,
    MXNet,
    HuggingFace,
}

/// Framework information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkInfo {
    pub framework_type: FrameworkType,
    pub version: String,
    pub path: String,
    pub features: Vec<String>,
    pub python_version: Option<String>,
}

// ============================================================================
// DETECTION FUNCTIONS
// ============================================================================

/// Detect PyTorch
pub async fn detect_pytorch() -> Result<FrameworkInfo> {
    let python_path = which("python3")
        .or_else(|_| which("python"))
        .map_err(|e| MLError::FrameworkNotFound(format!("Python not found: {}", e)))?;

    let output = AsyncCommand::new(&python_path)
        .args(&["-c", "import torch; print(torch.__version__); print(torch.__file__)"])
        .output()
        .await
        .map_err(|e| MLError::FrameworkDetectionFailed(format!("Failed to run Python: {}", e)))?;

    if !output.status.success() {
        return Err(MLError::FrameworkNotFound("PyTorch not available".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    if lines.len() < 2 {
        return Err(MLError::FrameworkDetectionFailed("Invalid PyTorch output".to_string()));
    }

    let version = lines[0].to_string();
    let path = lines[1].to_string();

    // Detect features
    let mut features = vec!["python".to_string()];
    
    // Check for CUDA support
    let cuda_check = AsyncCommand::new(&python_path)
        .args(&["-c", "import torch; print(torch.cuda.is_available())"])
        .output()
        .await;
        
    if let Ok(cuda_output) = cuda_check {
        if String::from_utf8_lossy(&cuda_output.stdout).trim() == "True" {
            features.push("cuda".to_string());
        }
    }

    // Check for distributed support
    let dist_check = AsyncCommand::new(&python_path)
        .args(&["-c", "import torch.distributed; print('True')"])
        .output()
        .await;
        
    if dist_check.is_ok() {
        features.push("distributed".to_string());
    }

    // Check for AMP support
    features.push("amp".to_string()); // PyTorch 1.6+ has AMP

    Ok(FrameworkInfo {
        framework_type: FrameworkType::PyTorch,
        version,
        path,
        features,
        python_version: get_python_version(&python_path).await,
    })
}

/// Detect TensorFlow
pub async fn detect_tensorflow() -> Result<FrameworkInfo> {
    let python_path = which("python3")
        .or_else(|_| which("python"))
        .map_err(|e| MLError::FrameworkNotFound(format!("Python not found: {}", e)))?;

    let output = AsyncCommand::new(&python_path)
        .args(&["-c", "import tensorflow as tf; print(tf.__version__); print(tf.__file__)"])
        .output()
        .await
        .map_err(|e| MLError::FrameworkDetectionFailed(format!("Failed to run Python: {}", e)))?;

    if !output.status.success() {
        return Err(MLError::FrameworkNotFound("TensorFlow not available".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    if lines.len() < 2 {
        return Err(MLError::FrameworkDetectionFailed("Invalid TensorFlow output".to_string()));
    }

    let version = lines[0].to_string();
    let path = lines[1].to_string();

    // Detect features
    let mut features = vec!["python".to_string()];
    
    // Check for GPU support
    let gpu_check = AsyncCommand::new(&python_path)
        .args(&["-c", "import tensorflow as tf; print(len(tf.config.list_physical_devices('GPU')) > 0)"])
        .output()
        .await;
        
    if let Ok(gpu_output) = gpu_check {
        if String::from_utf8_lossy(&gpu_output.stdout).trim() == "True" {
            features.push("gpu".to_string());
        }
    }

    // TensorFlow 2.x has mixed precision and XLA
    features.push("mixed_precision".to_string());
    features.push("xla".to_string());

    Ok(FrameworkInfo {
        framework_type: FrameworkType::TensorFlow,
        version,
        path,
        features,
        python_version: get_python_version(&python_path).await,
    })
}

/// Detect ONNX Runtime
pub async fn detect_onnx() -> Result<FrameworkInfo> {
    let python_path = which("python3")
        .or_else(|_| which("python"))
        .map_err(|e| MLError::FrameworkNotFound(format!("Python not found: {}", e)))?;

    let output = AsyncCommand::new(&python_path)
        .args(&["-c", "import onnxruntime as ort; print(ort.__version__); print(ort.__file__)"])
        .output()
        .await
        .map_err(|e| MLError::FrameworkDetectionFailed(format!("Failed to run Python: {}", e)))?;

    if !output.status.success() {
        return Err(MLError::FrameworkNotFound("ONNX Runtime not available".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    if lines.len() < 2 {
        return Err(MLError::FrameworkDetectionFailed("Invalid ONNX output".to_string()));
    }

    let version = lines[0].to_string();
    let path = lines[1].to_string();

    let features = vec!["python".to_string(), "inference".to_string()];

    Ok(FrameworkInfo {
        framework_type: FrameworkType::ONNX,
        version,
        path,
        features,
        python_version: get_python_version(&python_path).await,
    })
}

/// Detect Hugging Face Transformers
pub async fn detect_huggingface() -> Result<FrameworkInfo> {
    let python_path = which("python3")
        .or_else(|_| which("python"))
        .map_err(|e| MLError::FrameworkNotFound(format!("Python not found: {}", e)))?;

    let output = AsyncCommand::new(&python_path)
        .args(&["-c", "import transformers; print(transformers.__version__); print(transformers.__file__)"])
        .output()
        .await
        .map_err(|e| MLError::FrameworkDetectionFailed(format!("Failed to run Python: {}", e)))?;

    if !output.status.success() {
        return Err(MLError::FrameworkNotFound("Hugging Face Transformers not available".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    if lines.len() < 2 {
        return Err(MLError::FrameworkDetectionFailed("Invalid Transformers output".to_string()));
    }

    let version = lines[0].to_string();
    let path = lines[1].to_string();

    let features = vec![
        "python".to_string(),
        "transformers".to_string(),
        "tokenizers".to_string(),
    ];

    Ok(FrameworkInfo {
        framework_type: FrameworkType::HuggingFace,
        version,
        path,
        features,
        python_version: get_python_version(&python_path).await,
    })
}

/// Detect JAX
pub async fn detect_jax() -> Result<FrameworkInfo> {
    let python_path = which("python3")
        .or_else(|_| which("python"))
        .map_err(|e| MLError::FrameworkNotFound(format!("Python not found: {}", e)))?;

    let output = AsyncCommand::new(&python_path)
        .args(&["-c", "import jax; print(jax.__version__); print(jax.__file__)"])
        .output()
        .await
        .map_err(|e| MLError::FrameworkDetectionFailed(format!("Failed to run Python: {}", e)))?;

    if !output.status.success() {
        return Err(MLError::FrameworkNotFound("JAX not available".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    if lines.len() < 2 {
        return Err(MLError::FrameworkDetectionFailed("Invalid JAX output".to_string()));
    }

    let version = lines[0].to_string();
    let path = lines[1].to_string();

    let features = vec!["python".to_string(), "jit".to_string(), "xla".to_string()];

    Ok(FrameworkInfo {
        framework_type: FrameworkType::JAX,
        version,
        path,
        features,
        python_version: get_python_version(&python_path).await,
    })
}

/// Detect MXNet
pub async fn detect_mxnet() -> Result<FrameworkInfo> {
    let python_path = which("python3")
        .or_else(|_| which("python"))
        .map_err(|e| MLError::FrameworkNotFound(format!("Python not found: {}", e)))?;

    let output = AsyncCommand::new(&python_path)
        .args(&["-c", "import mxnet as mx; print(mx.__version__); print(mx.__file__)"])
        .output()
        .await
        .map_err(|e| MLError::FrameworkDetectionFailed(format!("Failed to run Python: {}", e)))?;

    if !output.status.success() {
        return Err(MLError::FrameworkNotFound("MXNet not available".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    if lines.len() < 2 {
        return Err(MLError::FrameworkDetectionFailed("Invalid MXNet output".to_string()));
    }

    let version = lines[0].to_string();
    let path = lines[1].to_string();

    let features = vec!["python".to_string(), "gluon".to_string()];

    Ok(FrameworkInfo {
        framework_type: FrameworkType::MXNet,
        version,
        path,
        features,
        python_version: get_python_version(&python_path).await,
    })
}

/// Get Python version
async fn get_python_version(python_path: &PathBuf) -> Option<String> {
    let output = AsyncCommand::new(python_path)
        .args(&["-c", "import sys; print('.'.join(map(str, sys.version_info[:3])))"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Detect all available frameworks
pub async fn detect_all_frameworks() -> Result<Vec<FrameworkInfo>> {
    let mut frameworks = Vec::new();
    
    // Try to detect each framework type individually
    let detectors = [
        ("PyTorch", FrameworkType::PyTorch),
        ("TensorFlow", FrameworkType::TensorFlow),
        ("ONNX", FrameworkType::ONNX),
        ("Hugging Face", FrameworkType::HuggingFace),
        ("JAX", FrameworkType::JAX),
        ("MXNet", FrameworkType::MXNet),
    ];

    for (name, framework_type) in detectors {
        let result = match framework_type {
            FrameworkType::PyTorch => detect_pytorch().await,
            FrameworkType::TensorFlow => detect_tensorflow().await,
            FrameworkType::ONNX => detect_onnx().await,
            FrameworkType::HuggingFace => detect_huggingface().await,
            FrameworkType::JAX => detect_jax().await,
            FrameworkType::MXNet => detect_mxnet().await,
        };
        
        match result {
            Ok(info) => {
                info!("Detected {}: v{}", name, info.version);
                frameworks.push(info);
            }
            Err(e) => {
                debug!("Failed to detect {}: {}", name, e);
            }
        }
    }
    
    Ok(frameworks)
}

/// Get framework-specific optimization recommendations
pub fn get_framework_optimizations(framework_type: FrameworkType) -> Vec<String> {
    match framework_type {
        FrameworkType::PyTorch => vec![
            "Use DataLoader with num_workers > 0".to_string(),
            "Enable mixed precision training with autocast".to_string(),
            "Use torch.compile for model optimization".to_string(),
            "Set TORCH_BACKENDS_CUDNN_BENCHMARK=true for fixed input sizes".to_string(),
        ],
        FrameworkType::TensorFlow => vec![
            "Enable XLA compilation with tf.function(jit_compile=True)".to_string(),
            "Use mixed precision with policy.set_global('mixed_float16')".to_string(),
            "Optimize data pipeline with tf.data prefetch and parallel processing".to_string(),
            "Use TensorRT for inference optimization".to_string(),
        ],
        FrameworkType::ONNX => vec![
            "Use ONNX Runtime with optimized execution providers".to_string(),
            "Enable graph optimizations".to_string(),
            "Use quantization for inference speedup".to_string(),
        ],
        FrameworkType::HuggingFace => vec![
            "Use torch.compile with Transformers models".to_string(),
            "Enable gradient checkpointing for memory efficiency".to_string(),
            "Use dynamic padding in DataCollator".to_string(),
        ],
        FrameworkType::JAX => vec![
            "Use jit compilation with @jax.jit decorator".to_string(),
            "Enable XLA optimizations".to_string(),
            "Use vectorized operations with vmap".to_string(),
        ],
        FrameworkType::MXNet => vec![
            "Use symbolic API for graph optimization".to_string(),
            "Enable hybridization with .hybridize()".to_string(),
            "Use DataLoader with multiple workers".to_string(),
        ],
    }
}