# ClamAV for fnOS

[中文](README_CN.md) | English

A native antivirus application for fnOS (Flying Bull NAS), built on ClamAV 1.5.1. It uses FFI (Foreign Function Interface) to call ClamAV shared libraries directly, providing efficient virus scanning, threat detection, and file quarantine capabilities.

## Features

- **Full System Scan** - Scan the entire system or specified directories
- **Custom Scan** - Select specific paths to scan
- **Real-time Progress** - WebSocket-based real-time scan progress updates
- **Virus Database Updates** - Update virus definitions via freshclam
- **Threat Quarantine** - Safely isolate infected files
- **Scan History** - View historical scan records
- **Threat Management** - Handle detected threats with batch operations

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | Vue.js 3 |
| CGI | Bash (proxy) |
| Backend | Rust (Axum) |
| Antivirus Engine | ClamAV 1.5.1 (FFI) |
| Database | SQLite (rusqlite) |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Web UI (Vue.js)                        │
│                   (WebSocket Client)                        │
├─────────────────────────────────────────────────────────────┤
│                      CGI (bash)                             │
├─────────────────────────────────────────────────────────────┤
│              Rust Daemon (Axum HTTP + WebSocket)            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Scan      │  │   Update    │  │  Quarantine  │        │
│  │   Service   │  │   Service   │  │   Service    │        │
│  └──────────────┘  └─────────────┘  └─────────────┘        │
│                    ClamAV FFI Manager                       │
├─────────────────────────────────────────────────────────────┤
│              libclamav.so (FFI)                             │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
├── app/                      # Application files
│   ├── bin/                  # freshclam binary
│   ├── lib/                  # ClamAV shared libraries
│   ├── server/               # Rust daemon
│   ├── share/                # Virus database and configs
│   ├── ui/                   # Web UI and CGI
│   └── www/                  # Frontend static files
├── cmd/                      # Lifecycle scripts
├── config/                   # Application config
├── rust-server/              # Rust daemon source
├── manifest                  # Application manifest
├── ICON.PNG                  # 64x64 icon
├── ICON_256.PNG              # 256x256 icon
└── build.sh                  # Build script
```

## Quick Start

### Requirements

- **OS**: Linux (Debian/Ubuntu)
- **Rust**: 1.70+
- **Node.js**: 16+ (frontend development only)

### Build

```bash
# Full build (includes ClamAV shared library)
./build.sh

# Clean build
./build.sh --clean

# Skip ClamAV build (if already exists)
./build.sh --skip-clamav
```

### Install Dependencies

```bash
sudo apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    curl \
    git \
    libssl-dev \
    libcurl4-openssl-dev \
    libpcre2-dev \
    libjson-c-dev \
    zlib1g-dev \
    libxml2-dev \
    libncurses-dev
```

## Installation on fnOS

1. Download the FPK package from [Releases](https://github.com/ecJon/fnClamAV/releases)
2. Upload `fnnas.clamav.fpk` to your fnOS device
3. Install via App Store -> Manual Install

## API Reference

### WebSocket

```
WS /api/ws
```

Real-time scan progress and completion events.

### Scan Endpoints

```
POST /api/scan/start      # Start scan
POST /api/scan/stop       # Stop scan
GET  /api/scan/status     # Scan status
GET  /api/scan/history    # Scan history
```

### Update Endpoints

```
POST /api/update/start    # Start update
GET  /api/update/status   # Update status
GET  /api/update/version  # Current version
```

### Quarantine Endpoints

```
GET    /api/quarantine                # Quarantine list
POST   /api/quarantine/:uuid/restore  # Restore file
DELETE /api/quarantine/:uuid          # Delete record
```

## License

- **ClamAV**: GPL-2.0-or-later
- **Rust Daemon**: MIT
- **Web UI**: MIT

## Acknowledgments

- [ClamAV](https://www.clamav.net/) - Open source antivirus engine
- [fnOS](https://www.fnnas.com/) - NAS operating system
- Cisco Talos - ClamAV maintainers

## Changelog

### 1.3.1 (2026-02-15)

- Added batch operations for threat list
- Added GitHub Actions auto-build and release
- Use Debian 12 container for fnOS compatibility
- Enabled UNRAR support

### 1.3.0 (2026-02-14)

- Rebranded as "ClamAV for fnOS"
- Updated application icons

### 1.2.0 (2026-02-14)

- Implemented dual-thread scanning mode
- Added EMA rate calculation
- Fixed threat list display and real-time updates
- Stream scanning optimization

### 1.1.0 (2026-02-13)

- Added WebSocket real-time progress updates
- Integrated build scripts
- Fixed progress bar updates

### 1.0.0 (2026-02-12)

- Initial release
- ClamAV 1.5.1 FFI support
- Basic scanning functionality
- Virus database updates
- Threat quarantine
- Web UI
