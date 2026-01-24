//! Error types for the ML optimization library

use thiserror::Error;

/// Main result type for ML operations
pub type Result<T> = std::result::Result<T, MLError>;

/// ML optimization error types
#[derive(Error, Debug)]
pub enum MLError {
    /// Framework not found or not supported
    #[error("Framework not found: {0}")]
    FrameworkNotFound(String),

    /// Framework detection failed
    #[error("Failed to detect framework: {0}")]
    FrameworkDetectionFailed(String),

    /// Optimization failed
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),

    /// System information gathering failed
    #[error("Failed to gather system information: {0}")]
    SystemInfoError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Resource constraint violation
    #[error("Resource constraint violated: {0}")]
    ResourceConstraint(String),

    /// Profiling error
    #[error("Profiling error: {0}")]
    ProfilingError(String),

    /// Dataset processing error
    #[error("Dataset processing error: {0}")]
    DatasetError(String),

    /// Distributed training error
    #[error("Distributed training error: {0}")]
    DistributedError(String),

    /// I/O error (file operations, network)
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Configuration parsing error
    #[error("Configuration parsing error: {0}")]
    ConfigParseError(#[from] toml::de::Error),

    /// Process execution error
    #[error("Process execution error: {0}")]
    ProcessError(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Timeout error
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Model loading error
    #[error("Model loading error: {0}")]
    ModelLoadError(String),

    /// Incompatible framework version
    #[error("Incompatible framework version: expected {expected}, found {found}")]
    IncompatibleVersion { expected: String, found: String },

    /// Missing dependency
    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    /// Hardware not supported
    #[error("Hardware not supported: {0}")]
    HardwareNotSupported(String),
}

impl From<walkdir::Error> for MLError {
    fn from(error: walkdir::Error) -> Self {
        Self::IoError(error.into())
    }
}

impl From<which::Error> for MLError {
    fn from(error: which::Error) -> Self {
        Self::FrameworkNotFound(error.to_string())
    }
}

impl MLError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Timeout(_) => true,
            Self::IoError(_) => true,
            Self::HttpError(_) => true,
            Self::ProcessError(_) => true,
            Self::CacheError(_) => true,
            _ => false,
        }
    }

    /// Get the severity level of the error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::FrameworkNotFound(_) => ErrorSeverity::High,
            Self::HardwareNotSupported(_) => ErrorSeverity::High,
            Self::ResourceConstraint(_) => ErrorSeverity::High,
            Self::InvalidConfiguration(_) => ErrorSeverity::Medium,
            Self::IncompatibleVersion { .. } => ErrorSeverity::Medium,
            Self::MissingDependency(_) => ErrorSeverity::Medium,
            Self::OptimizationFailed(_) => ErrorSeverity::Medium,
            Self::Timeout(_) => ErrorSeverity::Low,
            Self::CacheError(_) => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }

    /// Get suggested action for the error
    pub fn suggested_action(&self) -> String {
        match self {
            Self::FrameworkNotFound(name) => {
                format!("Install the {} framework using pip or conda", name)
            }
            Self::MissingDependency(dep) => {
                format!("Install missing dependency: {}", dep)
            }
            Self::InvalidConfiguration(msg) => {
                format!("Fix configuration: {}", msg)
            }
            Self::ResourceConstraint(msg) => {
                format!("Adjust resource allocation: {}", msg)
            }
            Self::IncompatibleVersion { expected, .. } => {
                format!("Upgrade/downgrade to version {}", expected)
            }
            Self::PermissionDenied(msg) => {
                format!("Check permissions: {}", msg)
            }
            Self::Timeout(_) => {
                "Retry the operation or increase timeout".to_string()
            }
            _ => "Check logs for more details".to_string(),
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Low severity - operation can continue with warnings
    Low,
    /// Medium severity - operation should be retried or adjusted
    Medium,
    /// High severity - operation must be aborted
    High,
}