//! GPU Manager para HecateOS
//! 
//! Te voy a explicar cada parte:

use anyhow::Result;
use nvml_wrapper::Nvml;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// ============================================================================
// ESTRUCTURAS DE DATOS
// ============================================================================

/// Información en tiempo real de una GPU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuStatus {
    pub index: u32,                // Índice de la GPU (0, 1, 2...)
    pub name: String,               // Nombre (ej: "NVIDIA RTX 4090")
    pub temperature: u32,           // Temperatura en Celsius
    pub power_draw: u32,           // Consumo actual en Watts
    pub power_limit: u32,          // Límite de potencia en Watts
    pub memory_used: u64,          // VRAM usada en bytes
    pub memory_total: u64,         // VRAM total en bytes
    pub utilization_gpu: u32,      // % de uso de GPU
    pub utilization_memory: u32,   // % de uso de memoria
    pub fan_speed: Option<u32>,    // Velocidad del ventilador (si está disponible)
    pub clock_graphics: u32,       // Frecuencia del core en MHz
    pub clock_memory: u32,         // Frecuencia de memoria en MHz
}

/// Configuración de optimización para una GPU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub power_mode: PowerMode,
    pub power_limit: Option<u32>,     // None = usar default
    pub temp_target: Option<u32>,     // Temperatura objetivo
    pub fan_curve: Option<FanCurve>,  // Curva de ventilador personalizada
}

/// Modos de energía
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PowerMode {
    MaxPerformance,  // Sin límites, máximo rendimiento
    Balanced,        // Balance entre rendimiento y consumo
    PowerSaver,      // Minimizar consumo
    Custom,          // Configuración manual
}

/// Curva de ventilador (temperatura -> velocidad)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurve {
    pub points: Vec<(u32, u32)>,  // (temperatura, velocidad %)
}

// ============================================================================
// MANAGER PRINCIPAL
// ============================================================================

/// Manager principal de GPUs
pub struct GpuManager {
    nvml: Arc<Nvml>,                           // NVIDIA Management Library
    gpus: Arc<RwLock<Vec<GpuDevice>>>,        // Lista de GPUs detectadas
    monitoring: Arc<RwLock<bool>>,            // ¿Está monitoreando?
}

/// Representa una GPU individual
struct GpuDevice {
    index: u32,
    device: nvml_wrapper::Device<'static>,
    config: GpuConfig,
}

impl GpuManager {
    /// Crear un nuevo manager
    pub fn new() -> Result<Self> {
        // Inicializar NVML (NVIDIA Management Library)
        let nvml = Nvml::init()?;
        info!("NVML initialized successfully");
        
        Ok(Self {
            nvml: Arc::new(nvml),
            gpus: Arc::new(RwLock::new(Vec::new())),
            monitoring: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Detectar todas las GPUs NVIDIA
    pub async fn detect_gpus(&self) -> Result<Vec<GpuStatus>> {
        let device_count = self.nvml.device_count()?;
        info!("Found {} NVIDIA GPU(s)", device_count);
        
        let mut gpu_list = Vec::new();
        let mut statuses = Vec::new();
        
        for i in 0..device_count {
            // Obtener dispositivo
            let device = self.nvml.device_by_index(i)?;
            
            // Obtener información
            let status = self.get_gpu_status(&device, i)?;
            statuses.push(status);
            
            // Guardar dispositivo
            gpu_list.push(GpuDevice {
                index: i,
                device,
                config: GpuConfig {
                    power_mode: PowerMode::Balanced,
                    power_limit: None,
                    temp_target: Some(83),  // Target 83°C
                    fan_curve: None,
                },
            });
        }
        
        // Guardar lista de GPUs
        let mut gpus = self.gpus.write().await;
        *gpus = gpu_list;
        
        Ok(statuses)
    }
    
    /// Obtener estado actual de una GPU
    fn get_gpu_status(&self, device: &nvml_wrapper::Device, index: u32) -> Result<GpuStatus> {
        use nvml_wrapper::enum_wrappers::device::{TemperatureSensor, Clock};
        
        // Nombre de la GPU
        let name = device.name()?;
        
        // Temperatura
        let temperature = device.temperature(TemperatureSensor::Gpu)?;
        
        // Potencia
        let power_draw = device.power_usage()? / 1000;  // mW a W
        let power_limit = device.power_management_limit()? / 1000;
        
        // Memoria
        let mem_info = device.memory_info()?;
        
        // Utilización
        let utilization = device.utilization_rates()?;
        
        // Velocidad del ventilador (puede no estar disponible)
        let fan_speed = device.fan_speed(0).ok();
        
        // Frecuencias
        let clock_graphics = device.clock_info(Clock::Graphics)?;
        let clock_memory = device.clock_info(Clock::Memory)?;
        
        Ok(GpuStatus {
            index,
            name,
            temperature,
            power_draw,
            power_limit,
            memory_used: mem_info.used,
            memory_total: mem_info.total,
            utilization_gpu: utilization.gpu,
            utilization_memory: utilization.memory,
            fan_speed,
            clock_graphics,
            clock_memory,
        })
    }
    
    /// Aplicar configuración a una GPU
    pub async fn apply_config(&self, gpu_index: u32, config: GpuConfig) -> Result<()> {
        let gpus = self.gpus.read().await;
        
        let gpu = gpus
            .iter()
            .find(|g| g.index == gpu_index)
            .ok_or_else(|| anyhow::anyhow!("GPU {} not found", gpu_index))?;
        
        match config.power_mode {
            PowerMode::MaxPerformance => {
                self.set_max_performance(&gpu.device).await?;
            }
            PowerMode::Balanced => {
                self.set_balanced(&gpu.device).await?;
            }
            PowerMode::PowerSaver => {
                self.set_power_saver(&gpu.device).await?;
            }
            PowerMode::Custom => {
                if let Some(limit) = config.power_limit {
                    gpu.device.set_power_management_limit(limit * 1000)?;  // W a mW
                }
            }
        }
        
        info!("Applied {:?} mode to GPU {}", config.power_mode, gpu_index);
        Ok(())
    }
    
    /// Configurar para máximo rendimiento
    async fn set_max_performance(&self, device: &nvml_wrapper::Device) -> Result<()> {
        // Desactivar límites de potencia
        let max_limit = device.power_management_limit_constraints()?.max_limit;
        device.set_power_management_limit(max_limit)?;
        
        // Modo de persistencia (mantiene driver cargado)
        device.set_persistent_mode(true)?;
        
        // Auto boost al máximo
        device.set_auto_boosted_clocks(true)?;
        
        info!("GPU set to maximum performance mode");
        Ok(())
    }
    
    /// Configurar modo balanceado
    async fn set_balanced(&self, device: &nvml_wrapper::Device) -> Result<()> {
        // Límite de potencia al 90% del máximo
        let constraints = device.power_management_limit_constraints()?;
        let balanced_limit = (constraints.max_limit * 90) / 100;
        device.set_power_management_limit(balanced_limit)?;
        
        info!("GPU set to balanced mode");
        Ok(())
    }
    
    /// Configurar modo ahorro de energía
    async fn set_power_saver(&self, device: &nvml_wrapper::Device) -> Result<()> {
        // Límite de potencia al 70% del máximo
        let constraints = device.power_management_limit_constraints()?;
        let eco_limit = (constraints.max_limit * 70) / 100;
        device.set_power_management_limit(eco_limit)?;
        
        info!("GPU set to power saver mode");
        Ok(())
    }
    
    /// Iniciar monitoreo en tiempo real
    pub async fn start_monitoring(&self) {
        let mut monitoring = self.monitoring.write().await;
        *monitoring = true;
        
        let nvml = Arc::clone(&self.nvml);
        let gpus = Arc::clone(&self.gpus);
        let monitoring_flag = Arc::clone(&self.monitoring);
        
        // Spawn tarea async para monitoreo
        tokio::spawn(async move {
            loop {
                // Verificar si seguimos monitoreando
                if !*monitoring_flag.read().await {
                    break;
                }
                
                // Leer estado de cada GPU
                let gpu_list = gpus.read().await;
                for gpu in gpu_list.iter() {
                    if let Ok(status) = Self::get_gpu_status_static(&gpu.device, gpu.index) {
                        // Verificar temperatura
                        if status.temperature > 85 {
                            warn!("GPU {} temperature high: {}°C", gpu.index, status.temperature);
                        }
                        
                        // Verificar VRAM
                        let mem_percent = (status.memory_used * 100) / status.memory_total;
                        if mem_percent > 90 {
                            warn!("GPU {} VRAM usage high: {}%", gpu.index, mem_percent);
                        }
                    }
                }
                
                // Esperar 1 segundo antes de la siguiente lectura
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }
    
    // Versión estática para usar en el spawn
    fn get_gpu_status_static(device: &nvml_wrapper::Device, index: u32) -> Result<GpuStatus> {
        use nvml_wrapper::enum_wrappers::device::{TemperatureSensor, Clock};
        
        Ok(GpuStatus {
            index,
            name: device.name()?,
            temperature: device.temperature(TemperatureSensor::Gpu)?,
            power_draw: device.power_usage()? / 1000,
            power_limit: device.power_management_limit()? / 1000,
            memory_used: device.memory_info()?.used,
            memory_total: device.memory_info()?.total,
            utilization_gpu: device.utilization_rates()?.gpu,
            utilization_memory: device.utilization_rates()?.memory,
            fan_speed: device.fan_speed(0).ok(),
            clock_graphics: device.clock_info(Clock::Graphics)?,
            clock_memory: device.clock_info(Clock::Memory)?,
        })
    }
}

// ============================================================================
// FUNCIONES HELPER
// ============================================================================

/// Formatear bytes a formato legible
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Crear resumen de estado
pub fn gpu_summary(status: &GpuStatus) -> String {
    format!(
        "{}: {}°C, {}W/{W}, GPU: {}%, VRAM: {}/{} ({}%)",
        status.name,
        status.temperature,
        status.power_draw,
        status.power_limit,
        status.utilization_gpu,
        format_bytes(status.memory_used),
        format_bytes(status.memory_total),
        (status.memory_used * 100) / status.memory_total
    )
}