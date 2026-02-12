# ClamAV 杀毒软件 for 飞牛 fnOS

基于 ClamAV 1.5.1 的病毒扫描和防护软件，专为飞牛 fnOS 设计的原生应用。通过 FFI (Foreign Function Interface) 方式调用 ClamAV 动态库，提供高效的病毒扫描、威胁检测和隔离功能。

## 功能特性

- **全盘扫描** - 扫描整个系统或指定目录
- **自定义扫描** - 选择特定路径进行扫描
- **实时保护** - 监控文件变化（规划中）
- **病毒库更新** - 通过 freshclam 更新病毒定义
- **威胁隔离** - 将受感染文件安全隔离
- **扫描历史** - 查看历史扫描记录
- **威胁管理** - 处理检测到的威胁

## 技术架构

### 核心组件

```
┌─────────────────────────────────────────────────────────────┐
│                      Web UI (Vue.js)                        │
├─────────────────────────────────────────────────────────────┤
│                      CGI (bash)                             │
├─────────────────────────────────────────────────────────────┤
│              Rust Daemon (Axum HTTP)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Scan      │  │   Update    │  │  Quarantine  │        │
│  │   Service   │  │   Service   │  │   Service    │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│         └────────────────┴────────────────┴─────────────┐  │
│                    │  ClamAV FFI Manager                 │  │
│                    └──────────────┬─────────────────────┘  │
├───────────────────────────────────┼────────────────────────┤
│              libclamav.so (FFI)    │                       │
└───────────────────────────────────┴────────────────────────┘
```

### 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue.js 3 + 原生 JavaScript |
| 后端 | Rust (Axum) |
| 杀毒引擎 | ClamAV 1.5.1 (FFI) |
| 数据库 | SQLite (rusqlite) |
| CGI | Bash |

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
├── clamAV/                  # ClamAV 源码 (git 子模块)
├── manifest                 # 应用清单
├── ICON.PNG                 # 64x64 图标
├── ICON_256.PNG             # 256x256 图标
├── build-ffi.sh             # FFI 版本打包脚本
└── build-clamav-shared.sh   # ClamAV 动态库编译脚本
```

## 快速开始

### 环境要求

- **操作系统**: Linux (Debian/Ubuntu)
- **Rust**: 1.70+
- **Node.js**: 16+ (仅用于前端开发)
- **ClamAV**: 1.5.1 (自动编译)

### 安装依赖

```bash
# 安装编译依赖
sudo ./install-deps.sh
```

### 编译 ClamAV 动态库

```bash
# 编译 libclamav.so 及相关依赖库
./build-clamav-shared.sh
```

编译产物将输出到 `app/lib/` 目录：
- `libclamav.so.12` - ClamAV 核心库
- `libclammspack.so.0` - 压缩文件支持
- `libclamunrar.so.12` - RAR 解压支持

### 编译 Rust 守护进程

```bash
cd rust-server
cargo build --release
```

编译产物：`rust-server/target/release/clamav-daemon`

### 打包应用

```bash
# 打包为 fnOS 应用包 (fpk)
./build-ffi.sh
```

打包产物：`dist/fnnas.clamav.fpk` (约 140MB)

## 安装和使用

### 在飞牛 fnOS 上安装

1. 将 `dist/fnnas.clamav.fpk` 上传到飞牛 fnOS
2. 在应用中心中安装应用
3. 启动应用

### 应用功能

#### 1. 全盘扫描

扫描整个存储卷，检测所有威胁。

#### 2. 自定义扫描

选择特定目录或文件进行扫描。

#### 3. 病毒库更新

通过 freshclam 更新病毒定义库。

#### 4. 隔离管理

查看、恢复或删除已隔离的受感染文件。

## 配置说明

### manifest (应用清单)

```ini
appname               = fnnas.clamav
version               = 1.0.0
display_name          = ClamAV 杀毒软件
platform              = x86
source                = thirdparty
maintainer            = fnOS
distributor           = ClamAV Team
desktop_uidir         = ui
desktop_applaunchname = fnnas.clamav.Application
category              = security
```

### config/privilege (权限配置)

```json
{
    "defaults": {
        "run-as": "package"
    }
}
```

应用以专用用户身份运行，系统自动使用 `appname` 创建用户和组。

### config/resource (资源配置)

```json
{
    "data-share": {
        "shares": [
            {
                "name": "fnnas.clamav",
                "permission": {
                    "rw": ["fnnas.clamav"]
                }
            }
        ]
    }
}
```

## API 文档

### 健康检查

```
GET /health
GET /api/status
```

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
GET  /api/update/history  # 更新历史
```

### 威胁处理

```
GET    /api/threats              # 威胁列表
POST   /api/threats/:id/handle   # 处理威胁
```

### 隔离管理

```
GET    /api/quarantine                # 隔离列表
POST   /api/quarantine/:uuid/restore # 恢复文件
DELETE /api/quarantine/:uuid          # 删除记录
POST   /api/quarantine/cleanup       # 清理隔离区
```

## 开发说明

### ClamAV FFI 绑定

ClamAV FFI 绑定位于 `rust-server/src/clamav/`：

- `ffi.rs` - FFI 原始绑定
- `engine.rs` - 扫描引擎封装
- `manager.rs` - 引擎生命周期管理

### 环境变量

应用运行时可用的环境变量（飞牛 fnOS 提供）：

| 变量 | 说明 |
|------|------|
| `TRIM_APPDEST` | 应用安装目录 |
| `TRIM_PKGVAR` | 应用数据目录 |
| `TRIM_PKGETC` | 应用配置目录 |
| `TRIM_USERNAME` | 应用专用用户名 |

### 故障排除

#### 1. 动态库找不到

```
error while loading shared libraries: libclamav.so.12
```

**解决方法**: 确保 `app/lib/` 目录包含所有必需的 `.so` 文件，并且启动脚本设置了 `LD_LIBRARY_PATH`。

#### 2. 引擎初始化失败

```
Failed to initialize ClamAV engine
```

**解决方法**: 检查病毒库是否存在于 `app/share/clamav/` 目录。

#### 3. 扫描引擎未启动

```
scan engine not started
```

**解决方法**: 确保 Rust 守护进程正确初始化了扫描引擎。

## 相关文档

- [飞牛 fnOS 开发指南](docs/feinuiu-dev-guide.md)
- [ClamAV 官方文档](https://www.clamav.net/documents.html)
- [ClamAV FFI API](https://github.com/Cisco-Talos/clamav)

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

### 1.0.0 (2025-02-12)

- 初始版本发布
- 支持 ClamAV 1.5.1 FFI
- 基础扫描功能
- 病毒库更新
- 威胁隔离
- Web UI
