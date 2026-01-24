//! Distributed training coordination and optimization

use crate::error::{MLError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{debug, info, warn, error, instrument};

/// Distributed training strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributedStrategy {
    /// Data parallel training
    DataParallel,
    /// Model parallel training  
    ModelParallel,
    /// Pipeline parallel training
    PipelineParallel,
    /// Hybrid strategy
    Hybrid {
        data_parallel: bool,
        model_parallel: bool,
        pipeline_parallel: bool,
    },
}

/// Node information in distributed cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub address: SocketAddr,
    pub gpu_count: u32,
    pub gpu_memory: Vec<u64>,
    pub cpu_cores: u32,
    pub memory: u64,
    pub bandwidth: Option<u64>, // Mbps
    pub role: NodeRole,
    pub status: NodeStatus,
}

/// Node role in distributed training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeRole {
    Master,
    Worker,
    ParameterServer,
}

/// Node status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Offline,
    Busy,
    Error,
}

/// Distributed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    pub strategy: DistributedStrategy,
    pub nodes: Vec<NodeInfo>,
    pub master_addr: String,
    pub master_port: u16,
    pub world_size: u32,
    pub backend: DistributedBackend,
    pub timeout: Duration,
}

/// Distributed backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributedBackend {
    NCCL,
    Gloo,
    MPI,
}

/// Communication pattern for distributed training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPattern {
    pub all_reduce_algorithm: AllReduceAlgorithm,
    pub compression: CompressionConfig,
    pub gradient_accumulation_steps: u32,
}

/// All-reduce algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllReduceAlgorithm {
    Ring,
    Tree,
    Butterfly,
    Hierarchical,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub compression_ratio: f32,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Quantization,
    Sparsification,
    LowRank,
}

/// Distributed coordinator
#[derive(Debug)]
pub struct DistributedCoordinator {
    config: DistributedConfig,
    nodes: HashMap<String, NodeInfo>,
    communication_stats: CommunicationStats,
}

/// Communication statistics
#[derive(Debug, Default)]
pub struct CommunicationStats {
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub average_latency: Duration,
    pub bandwidth_utilization: f64,
}

impl DistributedCoordinator {
    /// Create new distributed coordinator
    pub fn new(config: DistributedConfig) -> Self {
        let mut nodes = HashMap::new();
        for node in &config.nodes {
            nodes.insert(node.id.clone(), node.clone());
        }

        Self {
            config,
            nodes,
            communication_stats: CommunicationStats::default(),
        }
    }

    /// Initialize distributed training cluster
    #[instrument]
    pub async fn initialize_cluster(&mut self) -> Result<()> {
        info!("Initializing distributed training cluster with {} nodes", self.config.nodes.len());

        // Validate cluster configuration
        self.validate_cluster_config()?;

        // Test connectivity between all nodes
        self.test_connectivity().await?;

        // Setup communication backend
        self.setup_backend().await?;

        // Optimize communication patterns
        self.optimize_communication().await?;

        info!("Distributed cluster initialized successfully");
        Ok(())
    }

    /// Validate cluster configuration
    fn validate_cluster_config(&self) -> Result<()> {
        if self.config.nodes.is_empty() {
            return Err(MLError::DistributedError("No nodes configured".to_string()));
        }

        // Check for master node
        let master_count = self.config.nodes.iter()
            .filter(|node| matches!(node.role, NodeRole::Master))
            .count();

        if master_count != 1 {
            return Err(MLError::DistributedError(
                format!("Expected exactly 1 master node, found {}", master_count)
            ));
        }

        // Validate world size
        if self.config.world_size != self.config.nodes.len() as u32 {
            return Err(MLError::DistributedError(
                "World size doesn't match number of nodes".to_string()
            ));
        }

        // Check for duplicate node IDs
        let mut seen_ids = std::collections::HashSet::new();
        for node in &self.config.nodes {
            if !seen_ids.insert(&node.id) {
                return Err(MLError::DistributedError(
                    format!("Duplicate node ID: {}", node.id)
                ));
            }
        }

        debug!("Cluster configuration validation passed");
        Ok(())
    }

    /// Test connectivity between all nodes
    #[instrument]
    async fn test_connectivity(&mut self) -> Result<()> {
        info!("Testing connectivity between nodes");

        for node in &self.config.nodes {
            match self.ping_node(&node.address).await {
                Ok(latency) => {
                    info!("Node {} reachable (latency: {:?})", node.id, latency);
                }
                Err(e) => {
                    warn!("Failed to reach node {}: {}", node.id, e);
                    // Update node status
                    if let Some(node_info) = self.nodes.get_mut(&node.id) {
                        node_info.status = NodeStatus::Offline;
                    }
                }
            }
        }

        // Check if we have enough online nodes
        let online_nodes = self.nodes.values()
            .filter(|node| matches!(node.status, NodeStatus::Online))
            .count();

        if online_nodes < 2 {
            return Err(MLError::DistributedError(
                "Insufficient online nodes for distributed training".to_string()
            ));
        }

        Ok(())
    }

    /// Ping a node to test connectivity
    async fn ping_node(&self, address: &SocketAddr) -> Result<Duration> {
        let start = std::time::Instant::now();
        
        match timeout(Duration::from_secs(5), TcpStream::connect(address)).await {
            Ok(Ok(_)) => Ok(start.elapsed()),
            Ok(Err(e)) => Err(MLError::DistributedError(
                format!("Connection failed: {}", e)
            )),
            Err(_) => Err(MLError::Timeout(Duration::from_secs(5))),
        }
    }

    /// Setup communication backend
    async fn setup_backend(&self) -> Result<()> {
        match self.config.backend {
            DistributedBackend::NCCL => {
                info!("Setting up NCCL backend for GPU communication");
                self.setup_nccl_backend().await
            }
            DistributedBackend::Gloo => {
                info!("Setting up Gloo backend for CPU communication");
                self.setup_gloo_backend().await
            }
            DistributedBackend::MPI => {
                info!("Setting up MPI backend");
                self.setup_mpi_backend().await
            }
        }
    }

    /// Setup NCCL backend
    async fn setup_nccl_backend(&self) -> Result<()> {
        // Check if all nodes have GPUs
        for node in &self.config.nodes {
            if node.gpu_count == 0 {
                return Err(MLError::DistributedError(
                    format!("Node {} has no GPUs but NCCL backend requires GPUs", node.id)
                ));
            }
        }

        // NCCL initialization would happen here
        // For now, we'll just validate the configuration
        debug!("NCCL backend configuration validated");
        Ok(())
    }

    /// Setup Gloo backend
    async fn setup_gloo_backend(&self) -> Result<()> {
        // Gloo can work with CPU-only nodes
        debug!("Gloo backend ready for CPU-based communication");
        Ok(())
    }

    /// Setup MPI backend
    async fn setup_mpi_backend(&self) -> Result<()> {
        // MPI setup would require external MPI installation
        warn!("MPI backend setup requires external MPI runtime");
        Ok(())
    }

    /// Optimize communication patterns
    async fn optimize_communication(&mut self) -> Result<()> {
        info!("Optimizing communication patterns for distributed training");

        // Analyze network topology
        let topology = self.analyze_network_topology().await?;
        
        // Choose optimal all-reduce algorithm
        let algorithm = self.choose_allreduce_algorithm(&topology);
        
        // Configure compression if beneficial
        let compression = self.configure_compression(&topology);

        info!("Selected all-reduce algorithm: {:?}", algorithm);
        info!("Compression config: {:?}", compression);

        Ok(())
    }

    /// Analyze network topology
    async fn analyze_network_topology(&self) -> Result<NetworkTopology> {
        let mut bandwidth_matrix = HashMap::new();
        let mut latency_matrix = HashMap::new();

        // Measure bandwidth and latency between all node pairs
        for node1 in &self.config.nodes {
            for node2 in &self.config.nodes {
                if node1.id != node2.id {
                    let key = (node1.id.clone(), node2.id.clone());
                    
                    // Simplified measurement (in practice would do actual network tests)
                    let latency = self.estimate_latency(&node1.address, &node2.address).await?;
                    let bandwidth = self.estimate_bandwidth(&node1.address, &node2.address).await?;
                    
                    latency_matrix.insert(key.clone(), latency);
                    bandwidth_matrix.insert(key, bandwidth);
                }
            }
        }

        Ok(NetworkTopology {
            latency_matrix,
            bandwidth_matrix,
        })
    }

    /// Estimate latency between nodes
    async fn estimate_latency(&self, _addr1: &SocketAddr, _addr2: &SocketAddr) -> Result<Duration> {
        // Simplified estimation - in practice would do actual ping tests
        Ok(Duration::from_millis(1))
    }

    /// Estimate bandwidth between nodes
    async fn estimate_bandwidth(&self, _addr1: &SocketAddr, _addr2: &SocketAddr) -> Result<u64> {
        // Simplified estimation - in practice would do actual bandwidth tests
        Ok(1000) // 1 Gbps
    }

    /// Choose optimal all-reduce algorithm
    fn choose_allreduce_algorithm(&self, topology: &NetworkTopology) -> AllReduceAlgorithm {
        let node_count = self.config.nodes.len();
        
        // Algorithm selection based on cluster size and topology
        match node_count {
            2..=4 => AllReduceAlgorithm::Ring,
            5..=16 => {
                // Check if we have hierarchical structure (e.g., multiple racks)
                if self.has_hierarchical_structure(topology) {
                    AllReduceAlgorithm::Hierarchical
                } else {
                    AllReduceAlgorithm::Tree
                }
            }
            17..=64 => AllReduceAlgorithm::Butterfly,
            _ => AllReduceAlgorithm::Hierarchical,
        }
    }

    /// Check if cluster has hierarchical structure
    fn has_hierarchical_structure(&self, _topology: &NetworkTopology) -> bool {
        // Simplified heuristic - in practice would analyze network topology
        self.config.nodes.len() > 8
    }

    /// Configure compression settings
    fn configure_compression(&self, topology: &NetworkTopology) -> CompressionConfig {
        // Calculate average bandwidth
        let avg_bandwidth = topology.bandwidth_matrix.values()
            .sum::<u64>() as f64 / topology.bandwidth_matrix.len() as f64;

        // Enable compression for low-bandwidth networks
        if avg_bandwidth < 100.0 { // Less than 100 Mbps
            CompressionConfig {
                enabled: true,
                algorithm: CompressionAlgorithm::Quantization,
                compression_ratio: 0.5, // 50% compression
            }
        } else {
            CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::None,
                compression_ratio: 1.0,
            }
        }
    }

    /// Get optimal strategy for given model size
    pub fn recommend_strategy(&self, model_parameters: u64, dataset_size: u64) -> DistributedStrategy {
        let total_gpu_memory: u64 = self.config.nodes.iter()
            .flat_map(|node| &node.gpu_memory)
            .sum();
        
        let model_size_gb = model_parameters * 4 / (1024 * 1024 * 1024); // Assume 4 bytes per parameter
        let gpu_memory_gb = total_gpu_memory / (1024 * 1024 * 1024);

        // Strategy selection based on model size relative to available memory
        if model_size_gb * 2 > gpu_memory_gb {
            // Model doesn't fit in single GPU memory
            if self.config.nodes.len() > 2 && model_size_gb > gpu_memory_gb / 2 {
                DistributedStrategy::Hybrid {
                    data_parallel: true,
                    model_parallel: true,
                    pipeline_parallel: false,
                }
            } else {
                DistributedStrategy::ModelParallel
            }
        } else if dataset_size > 100_000 && self.config.nodes.len() > 1 {
            // Large dataset benefits from data parallelism
            DistributedStrategy::DataParallel
        } else {
            DistributedStrategy::DataParallel
        }
    }

    /// Monitor cluster health
    #[instrument]
    pub async fn monitor_cluster(&mut self) -> Result<ClusterHealth> {
        let mut online_nodes = 0;
        let mut total_gpu_memory = 0;
        let mut total_cpu_cores = 0;

        // Clone node addresses to avoid borrow conflicts
        let node_addresses: Vec<(String, SocketAddr)> = self.nodes.iter()
            .map(|(id, node)| (id.clone(), node.address))
            .collect();

        for (node_id, address) in node_addresses {
            match self.ping_node(&address).await {
                Ok(_) => {
                    if let Some(node) = self.nodes.get_mut(&node_id) {
                        node.status = NodeStatus::Online;
                        online_nodes += 1;
                        total_gpu_memory += node.gpu_memory.iter().sum::<u64>();
                        total_cpu_cores += node.cpu_cores;
                    }
                }
                Err(_) => {
                    if let Some(node) = self.nodes.get_mut(&node_id) {
                        node.status = NodeStatus::Offline;
                    }
                }
            }
        }

        Ok(ClusterHealth {
            online_nodes,
            total_nodes: self.config.nodes.len() as u32,
            total_gpu_memory,
            total_cpu_cores,
            cluster_utilization: self.calculate_cluster_utilization(),
        })
    }

    /// Calculate cluster utilization
    fn calculate_cluster_utilization(&self) -> f64 {
        // Simplified calculation - in practice would monitor actual resource usage
        let online_ratio = self.nodes.values()
            .filter(|node| matches!(node.status, NodeStatus::Online))
            .count() as f64 / self.nodes.len() as f64;

        online_ratio * 0.8 // Assume 80% utilization when online
    }

    /// Start distributed training coordinator service
    pub async fn start_coordinator_service(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.master_addr, self.config.master_port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| MLError::DistributedError(format!("Failed to bind to {}: {}", addr, e)))?;

        info!("Coordinator service listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("Connection from {}", peer_addr);
                    tokio::spawn(async move {
                        if let Err(e) = handle_coordinator_connection(stream).await {
                            error!("Error handling connection from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Get communication statistics
    pub fn get_communication_stats(&self) -> &CommunicationStats {
        &self.communication_stats
    }

    /// Update node status
    pub fn update_node_status(&mut self, node_id: &str, status: NodeStatus) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = status;
            debug!("Updated node {} status to {:?}", node_id, status);
        }
    }
}

/// Network topology information
#[derive(Debug)]
struct NetworkTopology {
    latency_matrix: HashMap<(String, String), Duration>,
    bandwidth_matrix: HashMap<(String, String), u64>,
}

/// Cluster health information
#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterHealth {
    pub online_nodes: u32,
    pub total_nodes: u32,
    pub total_gpu_memory: u64,
    pub total_cpu_cores: u32,
    pub cluster_utilization: f64,
}

/// Handle coordinator connection
async fn handle_coordinator_connection(_stream: TcpStream) -> Result<()> {
    // Handle coordinator protocol messages
    // This would implement the actual distributed training coordination protocol
    debug!("Handling coordinator connection");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_config() -> DistributedConfig {
        let master = NodeInfo {
            id: "master".to_string(),
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 29500),
            gpu_count: 2,
            gpu_memory: vec![8_000_000_000, 8_000_000_000],
            cpu_cores: 16,
            memory: 64_000_000_000,
            bandwidth: Some(1000),
            role: NodeRole::Master,
            status: NodeStatus::Online,
        };

        let worker = NodeInfo {
            id: "worker1".to_string(),
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 29501),
            gpu_count: 2,
            gpu_memory: vec![8_000_000_000, 8_000_000_000],
            cpu_cores: 16,
            memory: 64_000_000_000,
            bandwidth: Some(1000),
            role: NodeRole::Worker,
            status: NodeStatus::Online,
        };

        DistributedConfig {
            strategy: DistributedStrategy::DataParallel,
            nodes: vec![master, worker],
            master_addr: "127.0.0.1".to_string(),
            master_port: 29500,
            world_size: 2,
            backend: DistributedBackend::NCCL,
            timeout: Duration::from_secs(30),
        }
    }

    #[test]
    fn test_coordinator_creation() {
        let config = create_test_config();
        let coordinator = DistributedCoordinator::new(config);
        assert_eq!(coordinator.nodes.len(), 2);
    }

    #[test]
    fn test_config_validation() {
        let config = create_test_config();
        let coordinator = DistributedCoordinator::new(config);
        assert!(coordinator.validate_cluster_config().is_ok());
    }

    #[test]
    fn test_strategy_recommendation() {
        let config = create_test_config();
        let coordinator = DistributedCoordinator::new(config);

        // Small model should use data parallel
        let strategy = coordinator.recommend_strategy(1_000_000, 100_000);
        assert!(matches!(strategy, DistributedStrategy::DataParallel));

        // Large model should use model parallel or hybrid
        let strategy = coordinator.recommend_strategy(10_000_000_000, 100_000);
        assert!(matches!(strategy, DistributedStrategy::ModelParallel | DistributedStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_allreduce_algorithm_selection() {
        let config = create_test_config();
        let coordinator = DistributedCoordinator::new(config);
        
        let topology = NetworkTopology {
            latency_matrix: HashMap::new(),
            bandwidth_matrix: HashMap::new(),
        };
        
        let algorithm = coordinator.choose_allreduce_algorithm(&topology);
        // With 2 nodes, should choose Ring
        assert!(matches!(algorithm, AllReduceAlgorithm::Ring));
    }

    #[test]
    fn test_compression_configuration() {
        let config = create_test_config();
        let coordinator = DistributedCoordinator::new(config);
        
        // High bandwidth topology
        let mut bandwidth_matrix = HashMap::new();
        bandwidth_matrix.insert(("master".to_string(), "worker1".to_string()), 1000);
        bandwidth_matrix.insert(("worker1".to_string(), "master".to_string()), 1000);
        
        let topology = NetworkTopology {
            latency_matrix: HashMap::new(),
            bandwidth_matrix,
        };
        
        let compression = coordinator.configure_compression(&topology);
        assert!(!compression.enabled); // High bandwidth, no compression needed
    }
}