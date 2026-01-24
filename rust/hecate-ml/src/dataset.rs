//! Dataset analysis and optimization utilities

use crate::error::{MLError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Dataset information for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub name: String,
    pub size: u64,           // Number of samples
    pub data_type: String,   // image, text, audio, tabular, etc.
    pub dimensions: Vec<u32>, // Shape information
}

/// Dataset loader configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLoaderConfig {
    pub batch_size: u32,
    pub num_workers: u32,
    pub prefetch_factor: u32,
    pub pin_memory: bool,
    pub drop_last: bool,
    pub shuffle: bool,
}

/// Data preprocessing recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingRecommendation {
    pub operation: String,
    pub parameters: HashMap<String, String>,
    pub rationale: String,
    pub expected_speedup: f64,
}

/// Dataset analyzer
pub struct DatasetAnalyzer {
    cache: HashMap<PathBuf, DatasetInfo>,
}

impl DatasetAnalyzer {
    /// Create a new dataset analyzer
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Analyze dataset from path
    pub fn analyze_dataset<P: AsRef<Path>>(&mut self, path: P) -> Result<DatasetInfo> {
        let path = path.as_ref();
        
        // Check cache first
        if let Some(cached) = self.cache.get(path) {
            debug!("Using cached dataset info for {}", path.display());
            return Ok(cached.clone());
        }

        info!("Analyzing dataset at {}", path.display());

        // Determine dataset type and characteristics
        let dataset_info = if path.is_file() {
            self.analyze_single_file(path)?
        } else if path.is_dir() {
            self.analyze_directory(path)?
        } else {
            return Err(MLError::DatasetError(
                format!("Path does not exist: {}", path.display())
            ));
        };

        // Cache the result
        self.cache.insert(path.to_path_buf(), dataset_info.clone());
        
        Ok(dataset_info)
    }

    /// Analyze a single file dataset
    fn analyze_single_file(&self, path: &Path) -> Result<DatasetInfo> {
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let extension = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "csv" | "tsv" => self.analyze_tabular_file(path, &name),
            "json" | "jsonl" => self.analyze_json_file(path, &name),
            "txt" => self.analyze_text_file(path, &name),
            "parquet" => self.analyze_parquet_file(path, &name),
            "h5" | "hdf5" => self.analyze_hdf5_file(path, &name),
            _ => Err(MLError::DatasetError(
                format!("Unsupported file format: {}", extension)
            )),
        }
    }

    /// Analyze a directory dataset
    fn analyze_directory(&self, path: &Path) -> Result<DatasetInfo> {
        let name = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Walk through directory and categorize files
        let mut file_types = HashMap::new();
        let mut total_files = 0;

        for entry in WalkDir::new(path).max_depth(3) {
            let entry = entry.map_err(|e| MLError::IoError(e.into()))?;
            
            if entry.file_type().is_file() {
                total_files += 1;
                
                if let Some(extension) = entry.path().extension().and_then(|s| s.to_str()) {
                    *file_types.entry(extension.to_lowercase()).or_insert(0) += 1;
                }
            }
        }

        // Determine dataset type based on file extensions
        let data_type = self.infer_data_type_from_extensions(&file_types);
        
        // Estimate dimensions based on data type
        let dimensions = self.estimate_dimensions_for_type(&data_type, path)?;

        Ok(DatasetInfo {
            name,
            size: total_files,
            data_type,
            dimensions,
        })
    }

    /// Analyze tabular file (CSV, TSV)
    fn analyze_tabular_file(&self, path: &Path, name: &str) -> Result<DatasetInfo> {
        // Read first few lines to determine structure
        let content = std::fs::read_to_string(path)
            .map_err(MLError::IoError)?;
        
        let lines: Vec<&str> = content.lines().take(10).collect();
        
        if lines.is_empty() {
            return Err(MLError::DatasetError("Empty CSV file".to_string()));
        }

        // Estimate number of rows
        let total_lines = content.lines().count();
        let first_line = lines.first().unwrap_or(&"");
        
        // Check if first line looks like a header (more letters than digits, or starts with letters)
        let letter_count = first_line.chars().filter(|c| c.is_alphabetic()).count();
        let digit_count = first_line.chars().filter(|c| c.is_numeric()).count();
        let has_header = first_line.contains(',') && 
                        letter_count > 0 && 
                        (letter_count > digit_count || first_line.trim().chars().next().map_or(false, |c| c.is_alphabetic()));
        
        let num_rows = if has_header {
            total_lines - 1 // Has header
        } else {
            total_lines
        };

        // Count columns
        let separator = if path.extension().and_then(|s| s.to_str()) == Some("tsv") { '\t' } else { ',' };
        let num_cols = lines.first()
            .map(|line| line.split(separator).count())
            .unwrap_or(0) as u32;

        Ok(DatasetInfo {
            name: name.to_string(),
            size: num_rows as u64,
            data_type: "tabular".to_string(),
            dimensions: vec![num_cols],
        })
    }

    /// Analyze JSON file
    fn analyze_json_file(&self, path: &Path, name: &str) -> Result<DatasetInfo> {
        let content = std::fs::read_to_string(path)
            .map_err(MLError::IoError)?;

        // Try to determine if it's JSON Lines or single JSON
        let lines: Vec<&str> = content.lines().collect();
        
        let (size, dimensions) = if lines.len() > 1 && lines.iter().all(|line| line.trim().starts_with('{')) {
            // JSON Lines format
            let sample_json: serde_json::Value = serde_json::from_str(lines[0])
                .map_err(MLError::SerializationError)?;
            
            let field_count = count_json_fields(&sample_json);
            (lines.len() as u64, vec![field_count])
        } else {
            // Single JSON object or array
            let json: serde_json::Value = serde_json::from_str(&content)
                .map_err(MLError::SerializationError)?;
            
            match &json {
                serde_json::Value::Array(arr) => {
                    let field_count = arr.first()
                        .map(count_json_fields)
                        .unwrap_or(0);
                    (arr.len() as u64, vec![field_count])
                }
                _ => {
                    let field_count = count_json_fields(&json);
                    (1, vec![field_count])
                }
            }
        };

        Ok(DatasetInfo {
            name: name.to_string(),
            size,
            data_type: "json".to_string(),
            dimensions,
        })
    }

    /// Analyze text file
    fn analyze_text_file(&self, path: &Path, name: &str) -> Result<DatasetInfo> {
        let content = std::fs::read_to_string(path)
            .map_err(MLError::IoError)?;
        
        let lines = content.lines().count() as u64;
        let avg_line_length = if lines > 0 {
            (content.len() / lines as usize) as u32
        } else {
            0
        };

        Ok(DatasetInfo {
            name: name.to_string(),
            size: lines,
            data_type: "text".to_string(),
            dimensions: vec![avg_line_length],
        })
    }

    /// Analyze Parquet file
    fn analyze_parquet_file(&self, _path: &Path, name: &str) -> Result<DatasetInfo> {
        // Would require parquet library to properly analyze
        warn!("Parquet analysis not fully implemented, using estimates");
        
        Ok(DatasetInfo {
            name: name.to_string(),
            size: 100000, // Estimate
            data_type: "parquet".to_string(),
            dimensions: vec![50], // Estimate 50 columns
        })
    }

    /// Analyze HDF5 file
    fn analyze_hdf5_file(&self, _path: &Path, name: &str) -> Result<DatasetInfo> {
        // Would require HDF5 library to properly analyze
        warn!("HDF5 analysis not fully implemented, using estimates");
        
        Ok(DatasetInfo {
            name: name.to_string(),
            size: 50000, // Estimate
            data_type: "hdf5".to_string(),
            dimensions: vec![224, 224, 3], // Common image dimensions
        })
    }

    /// Infer data type from file extensions
    fn infer_data_type_from_extensions(&self, file_types: &HashMap<String, u32>) -> String {
        let image_exts = ["jpg", "jpeg", "png", "bmp", "tiff", "webp"];
        let audio_exts = ["wav", "mp3", "flac", "ogg", "aac"];
        let video_exts = ["mp4", "avi", "mkv", "mov", "wmv"];
        let text_exts = ["txt", "json", "csv"];

        let mut image_count = 0;
        let mut audio_count = 0;
        let mut video_count = 0;
        let mut text_count = 0;

        for (ext, count) in file_types {
            if image_exts.contains(&ext.as_str()) {
                image_count += count;
            } else if audio_exts.contains(&ext.as_str()) {
                audio_count += count;
            } else if video_exts.contains(&ext.as_str()) {
                video_count += count;
            } else if text_exts.contains(&ext.as_str()) {
                text_count += count;
            }
        }

        // Return the most common type
        let max_count = [
            ("image", image_count),
            ("audio", audio_count),
            ("video", video_count),
            ("text", text_count),
        ].into_iter().max_by_key(|(_, count)| *count);

        max_count.map(|(data_type, _)| data_type.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Estimate dimensions for data type
    fn estimate_dimensions_for_type(&self, data_type: &str, path: &Path) -> Result<Vec<u32>> {
        match data_type {
            "image" => {
                // Try to analyze a sample image
                self.sample_image_dimensions(path)
            }
            "audio" => Ok(vec![16000]), // 16kHz sample rate
            "video" => Ok(vec![1920, 1080, 30]), // 1080p at 30fps
            "text" => Ok(vec![512]), // Average sequence length
            _ => Ok(vec![1]), // Default
        }
    }

    /// Sample image dimensions from directory
    fn sample_image_dimensions(&self, path: &Path) -> Result<Vec<u32>> {
        // Find first image file and try to get dimensions
        for entry in WalkDir::new(path).max_depth(2) {
            let entry = entry.map_err(|e| MLError::IoError(e.into()))?;
            
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if ["jpg", "jpeg", "png", "bmp"].contains(&ext.to_lowercase().as_str()) {
                    // For a real implementation, would use image library
                    // For now, return common dimensions
                    return Ok(vec![224, 224, 3]); // Common ImageNet size
                }
            }
        }
        
        // Default if no images found
        Ok(vec![224, 224, 3])
    }

    /// Generate data loader recommendations
    pub fn recommend_dataloader_config(
        &self,
        dataset_info: &DatasetInfo,
        available_memory: u64,
        cpu_cores: u32,
    ) -> DataLoaderConfig {
        // Calculate optimal batch size
        let memory_per_sample = self.estimate_memory_per_sample(dataset_info);
        let max_batch_size = (available_memory / 2 / memory_per_sample).max(1) as u32;
        let batch_size = max_batch_size.min(128).next_power_of_two();

        // Calculate optimal number of workers
        let num_workers = match dataset_info.data_type.as_str() {
            "image" | "audio" | "video" => (cpu_cores / 2).min(8).max(1),
            "text" | "tabular" => (cpu_cores / 4).min(4).max(1),
            _ => 2,
        };

        // Prefetch factor based on data type
        let prefetch_factor = match dataset_info.data_type.as_str() {
            "image" | "video" => 4, // Higher for visual data
            "audio" => 3,
            _ => 2,
        };

        // Pin memory if dataset is large enough to benefit
        let pin_memory = dataset_info.size > 1000 && memory_per_sample > 1_000_000;

        DataLoaderConfig {
            batch_size,
            num_workers,
            prefetch_factor,
            pin_memory,
            drop_last: dataset_info.size % (batch_size as u64) < (batch_size as u64) / 4,
            shuffle: true, // Generally beneficial for training
        }
    }

    /// Estimate memory per sample
    fn estimate_memory_per_sample(&self, dataset_info: &DatasetInfo) -> u64 {
        match dataset_info.data_type.as_str() {
            "image" => {
                let pixels: u64 = dataset_info.dimensions.iter().map(|&d| d as u64).product();
                pixels * 4 // 4 bytes per pixel (float32)
            }
            "text" => {
                let seq_len = dataset_info.dimensions.first().unwrap_or(&512);
                *seq_len as u64 * 4 // 4 bytes per token
            }
            "audio" => {
                let samples = dataset_info.dimensions.first().unwrap_or(&16000);
                *samples as u64 * 4 // 4 bytes per sample
            }
            "tabular" => {
                let features = dataset_info.dimensions.first().unwrap_or(&10);
                *features as u64 * 8 // 8 bytes per feature (float64)
            }
            _ => 1_000_000, // 1MB default
        }
    }

    /// Generate preprocessing recommendations
    pub fn recommend_preprocessing(&self, dataset_info: &DatasetInfo) -> Vec<PreprocessingRecommendation> {
        let mut recommendations = Vec::new();

        match dataset_info.data_type.as_str() {
            "image" => {
                recommendations.extend(self.image_preprocessing_recommendations(dataset_info));
            }
            "text" => {
                recommendations.extend(self.text_preprocessing_recommendations(dataset_info));
            }
            "audio" => {
                recommendations.extend(self.audio_preprocessing_recommendations(dataset_info));
            }
            "tabular" => {
                recommendations.extend(self.tabular_preprocessing_recommendations(dataset_info));
            }
            _ => {}
        }

        recommendations
    }

    /// Image preprocessing recommendations
    fn image_preprocessing_recommendations(&self, dataset_info: &DatasetInfo) -> Vec<PreprocessingRecommendation> {
        let mut recommendations = Vec::new();

        // Resize recommendation
        if dataset_info.dimensions.len() >= 2 {
            let width = dataset_info.dimensions[0];
            let height = dataset_info.dimensions[1];
            
            if width > 256 || height > 256 {
                let mut params = HashMap::new();
                params.insert("size".to_string(), "224".to_string());
                
                recommendations.push(PreprocessingRecommendation {
                    operation: "resize".to_string(),
                    parameters: params,
                    rationale: "Resize to standard 224x224 for better performance".to_string(),
                    expected_speedup: 1.5,
                });
            }
        }

        // Normalization
        let mut norm_params = HashMap::new();
        norm_params.insert("mean".to_string(), "[0.485, 0.456, 0.406]".to_string());
        norm_params.insert("std".to_string(), "[0.229, 0.224, 0.225]".to_string());
        
        recommendations.push(PreprocessingRecommendation {
            operation: "normalize".to_string(),
            parameters: norm_params,
            rationale: "ImageNet normalization for better convergence".to_string(),
            expected_speedup: 1.1,
        });

        recommendations
    }

    /// Text preprocessing recommendations  
    fn text_preprocessing_recommendations(&self, dataset_info: &DatasetInfo) -> Vec<PreprocessingRecommendation> {
        let mut recommendations = Vec::new();

        // Tokenization
        let mut tok_params = HashMap::new();
        let max_length = dataset_info.dimensions.first().unwrap_or(&512);
        tok_params.insert("max_length".to_string(), max_length.to_string());
        
        recommendations.push(PreprocessingRecommendation {
            operation: "tokenize".to_string(),
            parameters: tok_params,
            rationale: "Tokenize text with appropriate max length".to_string(),
            expected_speedup: 1.2,
        });

        recommendations
    }

    /// Audio preprocessing recommendations
    fn audio_preprocessing_recommendations(&self, _dataset_info: &DatasetInfo) -> Vec<PreprocessingRecommendation> {
        let mut recommendations = Vec::new();

        // Resampling
        let mut resample_params = HashMap::new();
        resample_params.insert("sample_rate".to_string(), "16000".to_string());
        
        recommendations.push(PreprocessingRecommendation {
            operation: "resample".to_string(),
            parameters: resample_params,
            rationale: "Resample to 16kHz for consistency".to_string(),
            expected_speedup: 1.3,
        });

        recommendations
    }

    /// Tabular preprocessing recommendations
    fn tabular_preprocessing_recommendations(&self, _dataset_info: &DatasetInfo) -> Vec<PreprocessingRecommendation> {
        let mut recommendations = Vec::new();

        // Scaling
        let scale_params = HashMap::new();
        
        recommendations.push(PreprocessingRecommendation {
            operation: "standard_scale".to_string(),
            parameters: scale_params,
            rationale: "Standardize features for better training".to_string(),
            expected_speedup: 1.1,
        });

        recommendations
    }
}

/// Count fields in JSON value
fn count_json_fields(value: &serde_json::Value) -> u32 {
    match value {
        serde_json::Value::Object(map) => map.len() as u32,
        serde_json::Value::Array(arr) => arr.len() as u32,
        _ => 1,
    }
}

impl Default for DatasetAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_dataset_analyzer_creation() {
        let analyzer = DatasetAnalyzer::new();
        assert!(analyzer.cache.is_empty());
    }

    #[test]
    fn test_csv_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");
        
        fs::write(&csv_path, "col1,col2,col3\n1,2,3\n4,5,6").unwrap();
        
        let mut analyzer = DatasetAnalyzer::new();
        let result = analyzer.analyze_dataset(&csv_path);
        
        assert!(result.is_ok());
        let dataset_info = result.unwrap();
        assert_eq!(dataset_info.data_type, "tabular");
        assert_eq!(dataset_info.size, 2); // 2 data rows
        assert_eq!(dataset_info.dimensions, vec![3]); // 3 columns
    }

    #[test]
    fn test_json_lines_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("test.jsonl");
        
        fs::write(&json_path, "{\"a\": 1, \"b\": 2}\n{\"a\": 3, \"b\": 4}").unwrap();
        
        let mut analyzer = DatasetAnalyzer::new();
        let result = analyzer.analyze_dataset(&json_path);
        
        assert!(result.is_ok());
        let dataset_info = result.unwrap();
        assert_eq!(dataset_info.data_type, "json");
        assert_eq!(dataset_info.size, 2);
        assert_eq!(dataset_info.dimensions, vec![2]); // 2 fields
    }

    #[test]
    fn test_dataloader_config_recommendation() {
        let dataset_info = DatasetInfo {
            name: "test".to_string(),
            size: 10000,
            data_type: "image".to_string(),
            dimensions: vec![224, 224, 3],
        };
        
        let analyzer = DatasetAnalyzer::new();
        let config = analyzer.recommend_dataloader_config(&dataset_info, 8_000_000_000, 8);
        
        assert!(config.batch_size > 0);
        assert!(config.num_workers > 0);
        assert!(config.prefetch_factor > 0);
    }

    #[test]
    fn test_memory_per_sample_estimation() {
        let analyzer = DatasetAnalyzer::new();
        
        let image_dataset = DatasetInfo {
            name: "images".to_string(),
            size: 1000,
            data_type: "image".to_string(),
            dimensions: vec![224, 224, 3],
        };
        
        let memory = analyzer.estimate_memory_per_sample(&image_dataset);
        assert_eq!(memory, 224 * 224 * 3 * 4); // width * height * channels * 4 bytes
    }

    #[test] 
    fn test_preprocessing_recommendations() {
        let analyzer = DatasetAnalyzer::new();
        
        let image_dataset = DatasetInfo {
            name: "images".to_string(),
            size: 1000,
            data_type: "image".to_string(),
            dimensions: vec![512, 512, 3],
        };
        
        let recommendations = analyzer.recommend_preprocessing(&image_dataset);
        assert!(!recommendations.is_empty());
        
        // Should recommend resize for large images
        assert!(recommendations.iter().any(|r| r.operation == "resize"));
        // Should recommend normalization
        assert!(recommendations.iter().any(|r| r.operation == "normalize"));
    }
}