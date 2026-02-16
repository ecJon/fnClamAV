# ClamAV 飞牛版

中文 | [English](README.md)

基于 ClamAV 1.5.1 的病毒扫描和防护软件，专为飞牛 fnOS 设计的原生应用。通过 FFI (Foreign Function Interface) 方式调用 ClamAV 动态库，提供高效的病毒扫描、威胁检测和隔离功能。

## 功能特性

- **全盘扫描** - 扫描整个系统或指定目录
- **自定义扫描** - 选择特定路径进行扫描
- **实时进度推送** - WebSocket 实时推送扫描进度
- **病毒库更新** - 通过 freshclam 更新病毒定义
- **威胁隔离** - 将受感染文件安全隔离
- **扫描历史** - 查看历史扫描记录
- **威胁管理** - 处理检测到的威胁（支持批量操作）

## 技术架构

```
┌─────────────────────────────────────────────────────────┐
│                   Web UI (Vue.js)                       │
│                (WebSocket Client)                       │
├─────────────────────────────────────────────────────────┤
│                    CGI (bash)                           │
├─────────────────────────────────────────────────────────┤
│           Rust Daemon (Axum + WebSocket)                │
│                                                         │
│    Scan Service    Update Service    Quarantine Service │
│         │               │                  │            │
│         └───────────────┼──────────────────┘            │
│                         ▼                               │
│               ClamAV FFI Manager                        │
├─────────────────────────────────────────────────────────┤
│                 libclamav.so (FFI)                      │
└─────────────────────────────────────────────────────────┘
```

### 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue.js 3 |
| CGI | Bash (转发) / Rust CGI (备用) |
| 后端 | Rust (Axum) |
| 杀毒引擎 | ClamAV 1.5.1 (FFI) |
| 数据库 | SQLite (rusqlite) |

## 项目结构

```
.
├── app/                      # 应用文件目录
│   ├── bin/                  # freshclam 二进制
│   ├── lib/                  # ClamAV 动态库
│   │   ├── libclamav.so.12*
│   │   ├── libclammspack.so.0*
│   │   └── libclamunrar*.so*
│   ├── server/              # Rust 守护进程
│   ├── share/               # 病毒库和配置
│   ├── ui/                  # Web UI 和 CGI
│   └── www/                 # 前端静态文件
├── cmd/                     # 应用生命周期脚本
│   └── main                 # 启动/停止/状态检查
├── config/                  # 应用配置
│   ├── privilege            # 权限配置
│   └── resource             # 资源配置
├── rust-server/             # Rust 守护进程源码
│   ├── src/
│   │   ├── clamav/          # ClamAV FFI 绑定
│   │   ├── services/        # 业务服务层
│   │   ├── handlers/        # HTTP 处理器
│   │   └── models/          # 数据模型
│   └── Cargo.toml
├── manifest                 # 应用清单
├── ICON.PNG                 # 64x64 图标
├── ICON_256.PNG             # 256x256 图标
└── build.sh                 # 统一构建脚本
```

## 快速开始

### 环境要求

- **操作系统**: Linux (Debian/Ubuntu)
- **Rust**: 1.70+
- **Node.js**: 16+ (仅用于前端开发)

### 一键构建

```bash
# 完整构建（包含 ClamAV 动态库）
./build.sh

# 清理缓存后重新构建
./build.sh --clean

# 跳过 ClamAV 构建（假设已存在）
./build.sh --skip-clamav
```

### 安装构建依赖

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

## 安装和使用

### 在飞牛 fnOS 上安装

1. 从 [Releases](https://github.com/ecJon/fnClamAV/releases) 下载 FPK 包
2. 将 `fnnas.clamav.fpk` 上传到飞牛 fnOS
3. 在应用中心选择手动安装

### 应用功能

#### 1. 全盘扫描

扫描整个存储卷，实时显示扫描进度。

#### 2. 自定义扫描

选择特定目录或文件进行扫描。

#### 3. 病毒库更新

通过 freshclam 更新病毒定义库。

#### 4. 隔离管理

查看、恢复或删除已隔离的受感染文件。

## API 文档

### WebSocket 连接

```
WS /api/ws
```

实时推送扫描进度和完成事件。

### 扫描相关

```
POST /api/scan/start      # 开始扫描
POST /api/scan/stop       # 停止扫描
GET  /api/scan/status     # 扫描状态
GET  /api/scan/history    # 扫描历史
```

### 更新相关

```
POST /api/update/start    # 开始更新
GET  /api/update/status   # 更新状态
GET  /api/update/version  # 当前版本
```

### 隔离管理

```
GET    /api/quarantine                # 隔离列表
POST   /api/quarantine/:uuid/restore  # 恢复文件
DELETE /api/quarantine/:uuid          # 删除记录
```

## 许可证

本应用使用以下许可：

- **ClamAV**: GPL-2.0-or-later
- **Rust 守护进程**: MIT
- **Web UI**: MIT

## 致谢

- [ClamAV](https://www.clamav.net/) - 开源杀毒引擎
- [飞牛 fnOS](https://www.fnnas.com/) - NAS 操作系统
- Cisco Talos - ClamAV 维护者

## 版本历史

### 1.3.1 (2026-02-15)

- 新增威胁列表批量操作功能
- 添加 GitHub Actions 自动构建发布
- 使用 Debian 12 容器编译以兼容飞牛系统
- 启用 UNRAR 支持

### 1.3.0 (2026-02-14)

- 更新品牌为"ClamAV飞牛版"
- 更新应用图标

### 1.2.0 (2026-02-14)

- 实现双线程扫描模式
- 添加 EMA 速率计算
- 修复威胁列表显示和实时更新
- 流式扫描模式优化

### 1.1.0 (2026-02-13)

- 新增 WebSocket 实时进度推送
- 整合构建脚本
- 修复进度条更新问题

### 1.0.0 (2026-02-12)

- 初始版本发布
- 支持 ClamAV 1.5.1 FFI
- 基础扫描功能
- 病毒库更新
- 威胁隔离
- Web UI
