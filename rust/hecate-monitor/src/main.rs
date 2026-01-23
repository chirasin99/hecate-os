//! HecateOS Real-time Monitoring Server
//! 
//! Servidor WebSocket que transmite métricas del sistema en tiempo real
//! Accesible desde navegador web para dashboard

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt, ProcessExt};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

// ============================================================================
// TIPOS DE DATOS
// ============================================================================

/// Métricas del sistema en un momento dado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub gpu: Vec<GpuMetrics>,
    pub disks: Vec<DiskMetrics>,
    pub network: NetworkMetrics,
    pub processes: ProcessMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f32,          // Uso total de CPU
    pub per_core: Vec<f32>,         // Uso por core
    pub temperature: Option<f32>,    // Temperatura si está disponible
    pub frequency: u64,              // Frecuencia actual en MHz
    pub load_avg: [f32; 3],         // Load average (1, 5, 15 min)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub swap_total_gb: f64,
    pub swap_used_gb: f64,
    pub cache_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub index: u32,
    pub name: String,
    pub temperature: u32,
    pub power_w: u32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub utilization: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub read_mb_s: f64,
    pub write_mb_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub interfaces: Vec<NetworkInterface>,
    pub total_rx_mb_s: f64,
    pub total_tx_mb_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_mb_s: f64,
    pub tx_mb_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetrics {
    pub total_count: usize,
    pub running_count: usize,
    pub top_by_cpu: Vec<ProcessInfo>,
    pub top_by_memory: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
}

// ============================================================================
// ESTADO COMPARTIDO
// ============================================================================

/// Estado compartido entre todas las conexiones
#[derive(Clone)]
struct AppState {
    metrics: Arc<RwLock<SystemMetrics>>,
    clients: Arc<RwLock<HashMap<String, tokio::sync::mpsc::Sender<SystemMetrics>>>>,
    system: Arc<RwLock<System>>,
}

impl AppState {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(system)),
        }
    }
}

// ============================================================================
// RECOLECTOR DE MÉTRICAS
// ============================================================================

/// Recolecta métricas del sistema periódicamente
async fn metrics_collector(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        // Actualizar system info
        let mut system = state.system.write().await;
        system.refresh_all();
        
        // Recolectar métricas
        let metrics = collect_metrics(&system).await;
        
        // Guardar métricas actuales
        {
            let mut current = state.metrics.write().await;
            *current = metrics.clone();
        }
        
        // Enviar a todos los clientes conectados
        let clients = state.clients.read().await;
        let mut disconnected = Vec::new();
        
        for (id, tx) in clients.iter() {
            if tx.send(metrics.clone()).await.is_err() {
                disconnected.push(id.clone());
            }
        }
        
        // Eliminar clientes desconectados
        if !disconnected.is_empty() {
            drop(clients);
            let mut clients = state.clients.write().await;
            for id in disconnected {
                clients.remove(&id);
                info!("Client {} disconnected", id);
            }
        }
    }
}

/// Recolecta todas las métricas del sistema
async fn collect_metrics(system: &System) -> SystemMetrics {
    // CPU Metrics
    let cpu = CpuMetrics {
        usage_percent: system.global_cpu_info().cpu_usage(),
        per_core: system.cpus().iter().map(|cpu| cpu.cpu_usage()).collect(),
        temperature: read_cpu_temperature(),
        frequency: system.cpus().first()
            .map(|cpu| cpu.frequency())
            .unwrap_or(0),
        load_avg: {
            let load = system.load_average();
            [load.one as f32, load.five as f32, load.fifteen as f32]
        },
    };
    
    // Memory Metrics
    let memory = MemoryMetrics {
        total_gb: bytes_to_gb(system.total_memory()),
        used_gb: bytes_to_gb(system.used_memory()),
        available_gb: bytes_to_gb(system.available_memory()),
        swap_total_gb: bytes_to_gb(system.total_swap()),
        swap_used_gb: bytes_to_gb(system.used_swap()),
        cache_gb: bytes_to_gb(system.available_memory() - system.free_memory()),
    };
    
    // GPU Metrics (si está disponible el módulo)
    let gpu = collect_gpu_metrics().await;
    
    // Disk Metrics
    let disks: Vec<DiskMetrics> = system.disks()
        .iter()
        .map(|disk| DiskMetrics {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_gb: bytes_to_gb(disk.total_space()),
            used_gb: bytes_to_gb(disk.total_space() - disk.available_space()),
            read_mb_s: 0.0,  // TODO: Calcular velocidad real
            write_mb_s: 0.0,
        })
        .collect();
    
    // Network Metrics
    let mut interfaces = Vec::new();
    let mut total_rx = 0.0;
    let mut total_tx = 0.0;
    
    for (name, data) in system.networks() {
        let rx_mb_s = data.received() as f64 / 1024.0 / 1024.0;
        let tx_mb_s = data.transmitted() as f64 / 1024.0 / 1024.0;
        
        interfaces.push(NetworkInterface {
            name: name.clone(),
            rx_mb_s,
            tx_mb_s,
        });
        
        total_rx += rx_mb_s;
        total_tx += tx_mb_s;
    }
    
    let network = NetworkMetrics {
        interfaces,
        total_rx_mb_s: total_rx,
        total_tx_mb_s: total_tx,
    };
    
    // Process Metrics
    let mut processes: Vec<_> = system.processes()
        .values()
        .map(|p| ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            cpu_percent: p.cpu_usage(),
            memory_mb: p.memory() / 1024,
        })
        .collect();
    
    // Ordenar por CPU
    processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
    let top_by_cpu: Vec<ProcessInfo> = processes.iter().take(5).cloned().collect();
    
    // Ordenar por memoria
    processes.sort_by(|a, b| b.memory_mb.cmp(&a.memory_mb));
    let top_by_memory: Vec<ProcessInfo> = processes.iter().take(5).cloned().collect();
    
    let running_count = system.processes()
        .values()
        .filter(|p| p.status() == sysinfo::ProcessStatus::Run)
        .count();
    
    let processes = ProcessMetrics {
        total_count: system.processes().len(),
        running_count,
        top_by_cpu,
        top_by_memory,
    };
    
    SystemMetrics {
        timestamp: Utc::now(),
        cpu,
        memory,
        gpu,
        disks,
        network,
        processes,
    }
}

// ============================================================================
// WEBSOCKET HANDLERS
// ============================================================================

/// Maneja conexiones WebSocket
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Maneja un socket individual
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let client_id = uuid::Uuid::new_v4().to_string();
    
    info!("New client connected: {}", client_id);
    
    // Canal para enviar métricas a este cliente
    let (tx, mut rx) = tokio::sync::mpsc::channel::<SystemMetrics>(10);
    
    // Registrar cliente
    {
        let mut clients = state.clients.write().await;
        clients.insert(client_id.clone(), tx);
    }
    
    // Enviar métricas iniciales
    {
        let metrics = state.metrics.read().await;
        let msg = serde_json::to_string(&*metrics).unwrap();
        let _ = sender.send(Message::Text(msg)).await;
    }
    
    // Spawn task para enviar métricas
    let mut send_task = tokio::spawn(async move {
        while let Some(metrics) = rx.recv().await {
            let msg = serde_json::to_string(&metrics).unwrap();
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    
    // Recibir mensajes del cliente
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Procesar comandos del cliente si los hay
                    info!("Received from client: {}", text);
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });
    
    // Esperar a que alguna tarea termine
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
    
    // Limpiar cliente
    {
        let mut clients = state.clients.write().await;
        clients.remove(&client_id);
    }
    
    info!("Client {} disconnected", client_id);
}

// ============================================================================
// SERVIDOR HTTP
// ============================================================================

/// Página HTML con dashboard
async fn dashboard() -> impl IntoResponse {
    axum::response::Html(include_str!("dashboard.html"))
}

/// Health check endpoint
async fn health() -> impl IntoResponse {
    "OK"
}

// ============================================================================
// FUNCIONES AUXILIARES
// ============================================================================

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

fn read_cpu_temperature() -> Option<f32> {
    // Intentar leer temperatura del CPU
    if let Ok(temp) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        if let Ok(millidegrees) = temp.trim().parse::<i32>() {
            return Some(millidegrees as f32 / 1000.0);
        }
    }
    None
}

async fn collect_gpu_metrics() -> Vec<GpuMetrics> {
    // TODO: Integrar con hecate-gpu module
    vec![]
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            cpu: CpuMetrics {
                usage_percent: 0.0,
                per_core: vec![],
                temperature: None,
                frequency: 0,
                load_avg: [0.0; 3],
            },
            memory: MemoryMetrics {
                total_gb: 0.0,
                used_gb: 0.0,
                available_gb: 0.0,
                swap_total_gb: 0.0,
                swap_used_gb: 0.0,
                cache_gb: 0.0,
            },
            gpu: vec![],
            disks: vec![],
            network: NetworkMetrics {
                interfaces: vec![],
                total_rx_mb_s: 0.0,
                total_tx_mb_s: 0.0,
            },
            processes: ProcessMetrics {
                total_count: 0,
                running_count: 0,
                top_by_cpu: vec![],
                top_by_memory: vec![],
            },
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("HecateOS Monitor Server starting...");
    
    // Crear estado compartido
    let state = AppState::new();
    
    // Iniciar recolector de métricas
    let collector_state = state.clone();
    tokio::spawn(async move {
        metrics_collector(collector_state).await;
    });
    
    // Configurar rutas
    let app = Router::new()
        .route("/", get(dashboard))
        .route("/health", get(health))
        .route("/ws", get(websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);
    
    // Iniciar servidor
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on http://{}", addr);
    info!("Dashboard: http://localhost:3000");
    info!("WebSocket: ws://localhost:3000/ws");
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Para uuid
mod uuid {
    use std::fmt;
    
    pub struct Uuid([u8; 16]);
    
    impl Uuid {
        pub fn new_v4() -> Self {
            let mut bytes = [0u8; 16];
            for byte in &mut bytes {
                *byte = rand::random();
            }
            Self(bytes)
        }
    }
    
    impl fmt::Display for Uuid {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            for (i, byte) in self.0.iter().enumerate() {
                if i == 4 || i == 6 || i == 8 || i == 10 {
                    write!(f, "-")?;
                }
                write!(f, "{:02x}", byte)?;
            }
            Ok(())
        }
    }
    
    mod rand {
        pub fn random() -> u8 {
            // Simple random para ejemplo
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos() as u8
        }
    }
}