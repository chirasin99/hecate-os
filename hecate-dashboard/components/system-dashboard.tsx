'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { 
  CpuHigh02Icon, 
  Memory01Icon,
  HardDrive01Icon,
  Wifi01Icon,
  Activity01Icon,
  ServerIcon,
  TemperatureColdIcon,
  SpeedometerIcon,
  ChartIcon
} from '@hugeicons/react';
import CpuMetrics from './metrics/cpu-metrics';
import MemoryMetrics from './metrics/memory-metrics';
import GpuMetrics from './metrics/gpu-metrics';
import NetworkMetrics from './metrics/network-metrics';
import ProcessList from './metrics/process-list';
import DiskMetrics from './metrics/disk-metrics';

interface SystemMetrics {
  timestamp: string;
  cpu: {
    usage_percent: number;
    per_core: number[];
    temperature: number | null;
    frequency: number;
    load_avg: [number, number, number];
  };
  memory: {
    total_gb: number;
    used_gb: number;
    available_gb: number;
    swap_total_gb: number;
    swap_used_gb: number;
    cache_gb: number;
  };
  gpu: Array<{
    index: number;
    name: string;
    temperature: number;
    power_w: number;
    memory_used_mb: number;
    memory_total_mb: number;
    utilization: number;
  }>;
  disks: Array<{
    name: string;
    mount_point: string;
    total_gb: number;
    used_gb: number;
    read_mb_s: number;
    write_mb_s: number;
  }>;
  network: {
    interfaces: Array<{
      name: string;
      rx_mb_s: number;
      tx_mb_s: number;
    }>;
    total_rx_mb_s: number;
    total_tx_mb_s: number;
  };
  processes: {
    total_count: number;
    running_count: number;
    top_by_cpu: Array<{
      pid: number;
      name: string;
      cpu_percent: number;
      memory_mb: number;
    }>;
    top_by_memory: Array<{
      pid: number;
      name: string;
      cpu_percent: number;
      memory_mb: number;
    }>;
  };
}

export default function SystemDashboard() {
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [history, setHistory] = useState<SystemMetrics[]>([]);
  const [connected, setConnected] = useState(false);
  const [wsError, setWsError] = useState<string | null>(null);

  useEffect(() => {
    let ws: WebSocket | null = null;
    let reconnectTimeout: NodeJS.Timeout | null = null;

    const connect = () => {
      try {
        // HecateOS monitoring port: 9313 (mystical number)
        // Can be overridden via NEXT_PUBLIC_HECATE_MONITOR_PORT env var
        const port = process.env.NEXT_PUBLIC_HECATE_MONITOR_PORT || '9313';
        const wsUrl = `ws://localhost:${port}/ws`;
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          console.log('Connected to HecateOS Monitor');
          setConnected(true);
          setWsError(null);
        };

        ws.onmessage = (event) => {
          try {
            const data: SystemMetrics = JSON.parse(event.data);
            setMetrics(data);
            setHistory(prev => {
              const newHistory = [...prev, data];
              return newHistory.slice(-60);
            });
          } catch (error) {
            console.error('Failed to parse metrics:', error);
          }
        };

        ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          setConnected(false);
          setWsError('Connection error');
        };

        ws.onclose = () => {
          console.log('Disconnected from HecateOS Monitor');
          setConnected(false);
          
          // Auto-reconnect after 5 seconds
          reconnectTimeout = setTimeout(() => {
            connect();
          }, 5000);
        };
      } catch (error) {
        console.error('Failed to create WebSocket:', error);
        setWsError('Failed to connect');
        setConnected(false);
      }
    };

    connect();

    return () => {
      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
      }
      if (ws) {
        ws.close();
      }
    };
  }, []);

  if (!metrics) {
    return (
      <div className="flex h-screen items-center justify-center">
        <Card className="w-96">
          <CardHeader>
            <CardTitle className="text-center">HecateOS Monitor</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col items-center space-y-4">
              <ServerIcon className="h-12 w-12 animate-pulse text-muted-foreground" />
              <p className="text-center text-muted-foreground">
                {wsError || 'Connecting to monitoring service...'}
              </p>
              {!connected && (
                <p className="text-xs text-center text-muted-foreground">
                  Make sure hecate-monitor is running on port 3000
                </p>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  const cpuUsageColor = metrics.cpu.usage_percent > 90 ? 'destructive' : 
                        metrics.cpu.usage_percent > 70 ? 'secondary' : 'default';
  
  const memUsagePercent = (metrics.memory.used_gb / metrics.memory.total_gb) * 100;
  const memUsageColor = memUsagePercent > 90 ? 'destructive' : 
                        memUsagePercent > 80 ? 'secondary' : 'default';

  return (
    <div className="min-h-screen bg-background p-4">
      <div className="mx-auto max-w-7xl space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-2">
              <ServerIcon className="h-8 w-8" />
              HecateOS System Monitor
            </h1>
            <p className="text-muted-foreground">Real-time system metrics and performance monitoring</p>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant={connected ? "default" : "destructive"}>
              {connected ? "● Connected" : "○ Disconnected"}
            </Badge>
            <Button variant="outline" size="sm" onClick={() => window.location.reload()}>
              Refresh
            </Button>
          </div>
        </div>

        <Separator />

        {/* Overview Cards */}
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">CPU Usage</CardTitle>
              <CpuHigh02Icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{metrics.cpu.usage_percent.toFixed(1)}%</div>
              <Progress value={metrics.cpu.usage_percent} className="mt-2" />
              <div className="mt-2 flex justify-between text-xs text-muted-foreground">
                <span>{metrics.cpu.frequency} MHz</span>
                <span>{metrics.cpu.temperature?.toFixed(1) || '--'}°C</span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Memory</CardTitle>
              <Memory01Icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {metrics.memory.used_gb.toFixed(1)}/{metrics.memory.total_gb.toFixed(1)} GB
              </div>
              <Progress value={memUsagePercent} className="mt-2" />
              <div className="mt-2 text-xs text-muted-foreground">
                Available: {metrics.memory.available_gb.toFixed(1)} GB
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Network</CardTitle>
              <Wifi01Icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="space-y-1">
                <div className="text-sm">
                  <span className="text-green-600">↓</span> {metrics.network.total_rx_mb_s.toFixed(2)} MB/s
                </div>
                <div className="text-sm">
                  <span className="text-blue-600">↑</span> {metrics.network.total_tx_mb_s.toFixed(2)} MB/s
                </div>
              </div>
              <div className="mt-2 text-xs text-muted-foreground">
                {metrics.network.interfaces.length} interfaces
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Processes</CardTitle>
              <Activity01Icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{metrics.processes.total_count}</div>
              <div className="text-sm text-muted-foreground">
                {metrics.processes.running_count} running
              </div>
              <div className="mt-2 text-xs text-muted-foreground">
                Load: {metrics.cpu.load_avg.map(l => l.toFixed(2)).join(' ')}
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Main Content Tabs */}
        <Tabs defaultValue="overview" className="space-y-4">
          <TabsList className="grid w-full grid-cols-6">
            <TabsTrigger value="overview">Overview</TabsTrigger>
            <TabsTrigger value="cpu">CPU</TabsTrigger>
            <TabsTrigger value="memory">Memory</TabsTrigger>
            <TabsTrigger value="gpu">GPU</TabsTrigger>
            <TabsTrigger value="disk">Storage</TabsTrigger>
            <TabsTrigger value="processes">Processes</TabsTrigger>
          </TabsList>

          <TabsContent value="overview" className="space-y-4">
            <div className="grid gap-4 lg:grid-cols-2">
              <CpuMetrics metrics={metrics.cpu} history={history.map(h => h.cpu)} />
              <MemoryMetrics metrics={metrics.memory} history={history.map(h => h.memory)} />
              <NetworkMetrics metrics={metrics.network} history={history.map(h => h.network)} />
              <DiskMetrics disks={metrics.disks} />
            </div>
          </TabsContent>

          <TabsContent value="cpu">
            <CpuMetrics metrics={metrics.cpu} history={history.map(h => h.cpu)} detailed />
          </TabsContent>

          <TabsContent value="memory">
            <MemoryMetrics metrics={metrics.memory} history={history.map(h => h.memory)} detailed />
          </TabsContent>

          <TabsContent value="gpu">
            <GpuMetrics gpus={metrics.gpu} />
          </TabsContent>

          <TabsContent value="disk">
            <DiskMetrics disks={metrics.disks} detailed />
          </TabsContent>

          <TabsContent value="processes">
            <ProcessList processes={metrics.processes} />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}