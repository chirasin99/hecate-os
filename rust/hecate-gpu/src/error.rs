//! Error types for the GPU management library

use thiserror::Error;

/// Main result type for GPU operations
pub type Result<T> = std::result::Result<T, GpuError>;

/// GPU management error types
#[derive(Error, Debug)]
pub enum GpuError {
    /// GPU with the specified index was not found
    #[error("GPU with index {0} not found")]
    GpuNotFound(u32),

    /// Backend for the specified vendor is not available
    #[error("Backend for vendor {0:?} is not available")]
    BackendNotAvailable(crate::GpuVendor),

    /// Operation is not supported by the current GPU or driver
    #[error("Operation not supported: {0}")]
    OperationNotSupported(String),

    /// NVIDIA NVML error
    #[error("NVIDIA NVML error: {0}")]
    NvmlError(String),

    /// AMD DRM error
    #[error("AMD DRM error: {0}")]
    DrmError(String),

    /// System/OS error
    #[error("System error: {0}")]
    SystemError(String),

    /// I/O error (file operations, device access)
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Driver not found or not installed
    #[error("Driver not found: {0}")]
    DriverNotFound(String),

    /// Invalid configuration parameter
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Timeout occurred during operation
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Load balancer is not available (no multi-GPU setup)
    #[error("Load balancer is not available")]
    LoadBalancerNotAvailable,

    /// GPU is in an invalid state for the requested operation
    #[error("GPU {0} is in invalid state: {1}")]
    InvalidState(u32, String),

    /// Power management error
    #[error("Power management error: {0}")]
    PowerError(String),

    /// Thermal management error
    #[error("Thermal management error: {0}")]
    ThermalError(String),

    /// Memory allocation error
    #[error("Memory allocation error: {0}")]
    MemoryError(String),

    /// PCI device error
    #[error("PCI device error: {0}")]
    PciError(String),

    /// Configuration parsing error
    #[error("Configuration parsing error: {0}")]
    ConfigParseError(#[from] toml::de::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[cfg(feature = "nvidia")]
impl From<nvml_wrapper::error::NvmlError> for GpuError {
    fn from(error: nvml_wrapper::error::NvmlError) -> Self {
        Self::NvmlError(error.to_string())
    }
}

impl From<nix::Error> for GpuError {
    fn from(error: nix::Error) -> Self {
        Self::SystemError(error.to_string())
    }
}

impl From<which::Error> for GpuError {
    fn from(error: which::Error) -> Self {
        Self::DriverNotFound(error.to_string())
    }
}

impl GpuError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Timeout(_) => true,
            Self::IoError(_) => true,
            Self::SystemError(_) => true,
            Self::InvalidState(_, _) => true,
            _ => false,
        }
    }

    /// Get the severity level of the error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::GpuNotFound(_) => ErrorSeverity::High,
            Self::BackendNotAvailable(_) => ErrorSeverity::High,
            Self::PermissionDenied(_) => ErrorSeverity::High,
            Self::DriverNotFound(_) => ErrorSeverity::High,
            Self::OperationNotSupported(_) => ErrorSeverity::Medium,
            Self::InvalidConfig(_) => ErrorSeverity::Medium,
            Self::InvalidState(_, _) => ErrorSeverity::Medium,
            Self::PowerError(_) => ErrorSeverity::Medium,
            Self::ThermalError(_) => ErrorSeverity::High,
            Self::MemoryError(_) => ErrorSeverity::High,
            Self::Timeout(_) => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Low severity - operation can continue
    Low,
    /// Medium severity - operation should be retried
    Medium,
    /// High severity - operation must be aborted
    High,
}