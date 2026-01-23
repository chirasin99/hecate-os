# HecateOS Dashboard

Real-time system monitoring dashboard for HecateOS.

## Overview

Web-based dashboard that connects to `hecate-monitor` service via WebSocket to display real-time system metrics including CPU, memory, GPU, disk, and network usage.

## Features

- **Real-time Updates** - WebSocket connection for live metrics streaming
- **Responsive Design** - Works on desktop and mobile devices
- **GPU Monitoring** - NVIDIA GPU temperature, power, and VRAM usage
- **Process Tracking** - Top CPU and memory consuming processes
- **Network Activity** - Per-interface bandwidth monitoring
- **Disk Usage** - Storage utilization and I/O statistics

## Prerequisites

- Node.js 18+ or Bun runtime
- `hecate-monitor` service running on port 9313 (configurable)

## Installation

```bash
# Using Bun (recommended)
bun install

# Or using npm
npm install
```

## Configuration

Copy the environment template:

```bash
cp .env.example .env.local
```

Edit `.env.local` to configure:
- `NEXT_PUBLIC_HECATE_MONITOR_PORT` - Monitor service port (default: 9313)

## Development

```bash
# Start development server
bun run dev

# Dashboard will be available at http://localhost:3000
```

## Production Build

```bash
# Build for production
bun run build

# Start production server
bun run start
```

## Architecture

Built with:
- **Next.js 14** - React framework with App Router
- **Shadcn UI** - Component library (base components, not Radix)
- **Tailwind CSS** - Styling
- **WebSocket** - Real-time communication with monitor service

## Default Ports

- **3000** - Dashboard web interface (development)
- **9313** - Monitor service WebSocket connection

## Troubleshooting

### Cannot connect to monitor service

1. Ensure `hecate-monitor` is running:
   ```bash
   systemctl status hecate-monitor
   ```

2. Check the port configuration in `.env.local`

3. Verify firewall allows connection to port 9313

### Build errors with Bun

If you encounter issues with Bun, fallback to npm:
```bash
npm install
npm run dev
```

## License

MIT - Part of HecateOS project