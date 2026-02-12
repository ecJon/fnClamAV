# 飞牛 ClamAV 杀毒应用 - 系统设计文档

**版本**: 1.0
**日期**: 2026-02-13
**状态**: 设计完成，待实现

---

## 1. 项目概述

### 1.1 目标
在飞牛OS系统上开发一款杀毒应用，通过内嵌的ClamAV引擎提供病毒扫描和病毒库管理功能。

### 1.2 核心价值
- 为飞牛用户提供安全防护能力
- 简洁易用的用户界面
- 可靠的后台扫描服务

---

## 2. 需求规格

### 2.1 功能需求

| 功能模块 | 需求详情 |
|----------|----------|
| **病毒库更新** | - 定时任务自动更新（默认每天一次）<br>- 用户可配置更新频率<br>- 手动触发更新 |
| **扫描功能** | - 全盘扫描：扫描所有挂载的硬盘数据<br>- 自定义扫描：用户选择/输入路径<br>- 扫描进度实时显示：百分比 + 当前文件 + 威胁数 |
| **扫描控制** | - 后台扫描：关闭窗口不打断<br>- 支持停止操作<br>- 重新进入应用可恢复扫描状态 |
| **威胁处理** | - 设置中可选：自动隔离 / 自动删除<br>- 自动处理发现的威胁文件 |
| **历史记录** | - 保存扫描历史记录<br>- 默认保留3个月<br>- 可查看历史扫描详情 |

### 2.2 非功能需求

| 需求类型 | 说明 |
|----------|------|
| **性能** | 扫描过程不影响系统正常使用 |
| **安全** | 隔离区安全存储，威胁文件不可恢复 |
| **易用性** | 界面简洁，操作直观 |
| **可靠性** | 后台服务稳定，异常自动恢复 |

### 2.3 暂不实现
- 通知功能

---

## 3. 系统架构

### 3.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                        飞牛桌面                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │         WebView (Vue 前端 - dist 静态文件)            │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │  │
│  │  │ 首页/   │ │ 扫描/   │ │ 历史/   │ │ 设置/   │    │  │
│  │  │ 仪表盘  │ │ 进度    │ │ 记录    │ │ 配置    │    │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘    │  │
│  └───────┼──────────┼──────────┼──────────┼────────────┘  │
└──────────┼──────────┼──────────┼──────────┼───────────────┘
           │          │          │          │
           ▼          ▼          ▼          ▼
┌────────────────────────────────────────────────────────────┐
│                   index.cgi (Shell)                         │
│              转发请求 → Rust HTTP 服务                       │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│              Rust 守护进程 (systemd 服务)                  │
│              HTTP Server (Unix Socket / localhost)          │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  REST API / WebSocket                                 │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │ │
│  │  │ 扫描管理  │  │ 更新管理  │  │ 状态管理  │          │ │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘          │ │
│  │       │             │             │                 │ │
│  │  ┌────▼─────────────────▼──────────▼────┐          │ │
│  │  │         ClamAV FFI 绑定层            │          │ │
│  │  └──────────────────┬────────────────────┘          │ │
│  └─────────────────────┼───────────────────────────────┘ │
└────────────────────────┼─────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│                    内嵌 ClamAV 引擎                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │libclamav │  │病毒库    │  │扫描器    │                 │
│  │(C + Rust)│  │(本地)    │  │          │                 │
│  └──────────┘  └──────────┘  └──────────┘                 │
└────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

| 组件 | 技术 | 职责 |
|------|------|------|
| **前端** | Vue.js | 用户界面，简洁风格 |
| **CGI 层** | Shell | 飞牛桌面与 Rust 服务通信桥梁（轻量级转发） |
| **Rust 服务** | Rust + HTTP | 后台扫描、更新管理、状态持久 |
| **ClamAV** | C + Rust FFI | 病毒扫描引擎 |

### 3.3 技术栈

| 层级 | 技术选择 |
|------|----------|
| 前端 | Vue.js 3 + Vite |
| CGI | Shell Script |
| 后端 | Rust (Axum/Warp) |
| 数据库 | SQLite |
| 杀毒引擎 | ClamAV (libclamav.so + FFI) |

---

## 3.4 ClamAV 集成方案

### 3.4.1 方案选择：FFI 动态库调用

**采用方案B：Rust FFI 调用 libclamav.so**

| 对比项 | 方案A（未采用） | 方案B（采用） |
|--------|----------------|---------------|
| **集成方式** | Rust 调用 clamscan/freshclam 二进制 | 动态链接 libclamav.so |
| **进程模型** | 每次扫描启动新进程 | 进程内单例引擎 |
| **第三方依赖** | 无 | 自写 FFI 绑定 |
| **实现复杂度** | 简单 | 中等 |
| **ClamAV更新** | 替换二进制即可 | 替换 .so 即可 |
| **性能** | 每次启动新进程，有开销 | 无进程启动开销，即时响应 |
| **进度反馈** | 解析命令输出，有延迟 | 实时回调，事件驱动 |
| **扫描控制** | 强制终止进程 | 优雅取消（设置标志） |
| **内存占用** | 进程隔离，总内存较高 | 内存共享，占用更少 |

**决策理由**：
- 扫描启动无延迟，用户体验更好
- 实时进度回调，可精确显示当前文件和进度
- 支持暂停/恢复扫描（通过控制扫描循环）
- 内存占用更小，资源共享更高效

### 3.4.2 文件布局

```
TRIM_APPDEST/                    # 应用安装目录
├── lib/                         # 动态库文件
│   ├── libclamav.so             # ClamAV 核心引擎库
│   ├── libclammspack.so         # 压缩包支持库
│   └── libclamunrar.so          # RAR 包支持库
├── bin/
│   └── freshclam               # 病毒库更新工具（保留）
└── lib/clamav/                 # 预置病毒库（首次启动用）
    ├── daily.cvd
    ├── main.cvd
    └── bytecode.cvd

$DATA_SHARE/clamav/             # 运行时病毒库（可更新）
├── daily.cld
├── main.cld
└── bytecode.cld
```

**说明**：`$DATA_SHARE` 表示从 `TRIM_DATA_SHARE_PATHS` 环境变量解析出的共享目录路径。

### 3.4.3 扫描流程设计

**FFI 方式的扫描流程**：

```
┌─────────────────────────────────────────────────────────────┐
│                     扫描启动                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │ 初始化 ClamAV 引擎    │
                │ 加载病毒库            │
                │ 编译引擎              │
                └───────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │ 遍历扫描路径          │
                │ 注册进度回调           │
                └───────────────────────┘
                            │
                    ┌───────┴────────┐
                    ▼                ▼
              发现威胁          文件正常
                    │                │
                    ▼                ▼
              记录威胁        继续下一个
              触发处理
```

### 3.4.4 扫描控制设计

**支持的操作**：开始扫描、停止扫描、暂停扫描、恢复扫描

| 操作 | 功能描述 | 实现方式 |
|------|----------|----------|
| 开始扫描 | 启动新的扫描任务 | 初始化引擎，开始遍历文件 |
| 停止扫描 | 终止当前扫描 | 设置取消标志，引擎优雅退出 |
| 暂停扫描 | 暂停扫描进度 | 在扫描循环中检查暂停标志 |
| 恢复扫描 | 从暂停位置继续 | 清除暂停标志，继续循环 |

**取消标志机制**：
```
┌──────────────┐       取消请求      ┌──────────────┐
│  扫描中      │ ──────────────▶│  取消标志     │
└──────────────┘                    └──────────────┘
       ▲                                  │
       │                                  ▼
       │                          ┌──────────────┐
       │                          │ 标志=true    │
       │                          └──────────────┘
       │                                  │
       │                                  ▼
 检查标志                          ┌──────────────┐
       │                          │  优雅退出    │
       └───────────────────────────▶│  清理资源    │
                                  └──────────────┘
```

### 3.4.5 进度反馈设计

**实时回调机制**：

```
扫描引擎 ──────回调─────▶ Rust FFI 层 ──────事件─────▶ 前端

回调事件类型:
├── on_file_start(file_path)       - 开始扫描文件
├── on_file_complete(file_path)     - 文件扫描完成
├── on_threat_found(file_path, virus_name) - 发现威胁
├── on_progress(percent, file_path) - 进度更新
└── on_scan_complete(result)       - 扫描完成
```

**进度计算**：
- 引擎回调提供当前文件和已完成数量
- 无需预先估算文件总数
- 可实现准确的实时进度显示

### 3.4.6 病毒库更新

**保留 freshclam 二进制**：
- freshclam 功能独立，无需改为 FFI
- 继续使用二进制方式调用病毒库更新

### 3.4.7 扫描任务队列设计

**设计目标**：确保同一时间只有一个扫描任务执行，多请求排队处理

**队列模型**：

```
┌─────────────────────────────────────────────────────────────┐
│                     扫描请求                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │   任务队列检查        │
                └───────────────────────┘
                            │
            ┌───────────┴───────────┐
            │                       │
      引擎空闲                  引擎忙碌
            │                       │
            ▼                       ▼
      立即开始              ┌──────────────┐
                            │ 加入等待队列  │
                            └──────────────┘
                                    │
                                    ▼
                            ┌──────────────┐
                            │ 返回排队信息  │
                            │ (排队位置)   │
                            └──────────────┘
```

**队列状态**：

| 状态 | 说明 | 行为 |
|------|------|------|
| `idle` | 引擎空闲 | 新请求立即开始扫描 |
| `scanning` | 正在扫描 | 新请求加入队列，返回排队位置 |
| `paused` | 已暂停 | 恢复后继续，或停止后处理下一个 |
| `error` | 引擎异常 | 拒绝新请求，提示错误 |

**队列响应示例**：

```json
// 引擎忙碌时
{
  "success": false,
  "error": {
    "code": "SCAN_ENGINE_BUSY",
    "message": "扫描任务正在进行中",
    "details": {
      "current_scan_id": "scan_20250213_120000",
      "queue_position": 0  // 0 表示当前扫描完成后自动开始
    }
  }
}

// 已加入队列
{
  "success": true,
  "message": "扫描任务已加入队列",
  "queue_position": 1,  // 队列中的位置
  "estimated_wait_seconds": 120
}
```

### 3.4.8 异常恢复设计

**设计目标**：引擎异常时自动检测并恢复，确保服务可用性

**异常场景**：

| 异常类型 | 检测方式 | 恢复策略 |
|----------|-----------|----------|
| 引擎初始化失败 | 启动时检测 | 回退到预置病毒库，记录错误 |
| 扫描中崩溃 | 进程信号/超时 | 重启引擎，标记扫描失败 |
| 病毒库损坏 | 加载失败 | 触发 freshclam 更新 |
| 内存溢出 | 监控/回调 | 限制单次扫描文件数，分批处理 |

**恢复流程**：

```
┌─────────────────────────────────────────────────────────────┐
│                   引擎运行中                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │ 检测到异常    │
                    │ (信号/错误/超时)│
                    └───────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │   1. 记录异常信息    │
                │   2. 清理引擎资源    │
                │   3. 标记当前任务失败  │
                └───────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │   4. 重新初始化引擎  │
                │   5. 加载病毒库      │
                └───────────────────────┘
                            │
                    ┌───────┴────────┐
                    ▼                ▼
              恢复成功          恢复失败
                    │                │
                    ▼                ▼
              继续处理队列      标记服务不可用
              通知管理员
```

**健康检查机制**：

- 定期心跳检测（每 30 秒）
- 引擎状态监控（内存使用、扫描任务数）
- 自动重启机制（连续失败 3 次后）

**降级策略**：

```
引擎异常不可用
        │
        ▼
┌───────────────────┐
│  降级模式        │
│  - 暂停新扫描    │
│  - 返回 503 错误 │
│  - 记录详细日志   │
└───────────────────┘
        │
        ▼
   管理员介入恢复
```

---

## 3.5 隔离区管理方案

### 3.5.1 隔离区目录结构

```
$DATA_SHARE/quarantine/
├── metadata/              # 隔离文件元数据
│   ├── <uuid>.json       # 每个隔离文件对应一个元数据文件
│   └── ...
└── files/                # 隔离文件存储
    ├── <uuid>            # 重命名后的隔离文件（无扩展名）
    └── ...
```

**说明**：`$DATA_SHARE` 表示从 `TRIM_DATA_SHARE_PATHS` 环境变量解析出的共享目录路径。

### 3.5.2 隔离操作流程

```
发现威胁文件
    │
    ▼
┌─────────────────────────────────────────┐
│ 1. 生成 UUID 作为隔离标识                │
│ 2. 读取文件信息（路径、大小、权限）       │
│ 3. 移动文件到 quarantine/files/<uuid>    │
│ 4. 写入元数据到 quarantine/metadata/     │
│ 5. 更新数据库记录                        │
└─────────────────────────────────────────┘
```

### 3.5.3 元数据格式

`quarantine/metadata/<uuid>.json`：
```json
{
  "uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "original_path": "/data/downloads/infected.exe",
  "original_name": "infected.exe",
  "file_size": 1024000,
  "file_hash": "sha256:abc123...",
  "quarantined_at": 1707553200,
  "virus_name": "Trojan.Generic",
  "scan_id": "scan_20250210_143022"
}
```

### 3.5.4 威胁处理模式

| 模式 | 行为 |
|------|------|
| **quarantine** | 隔离文件到隔离区（默认推荐） |
| **delete** | 直接删除威胁文件 |
| **none** | 仅记录，不处理 |

### 3.5.5 自动处理流程

```
┌─────────────────────────────────────────────────────────────┐
│                    扫描发现威胁                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │ 读取配置：auto_action  │
                └───────────────────────┘
                            │
            ┌───────────────┴───────────────┐
            │                               │
      auto_action=true               auto_action=false
            │                               │
            ▼                               ▼
    ┌───────────────┐              ┌───────────────┐
    │ 根据 action   │              │ 记录到威胁列表 │
    │ 执行相应操作   │              │ 等待用户处理   │
    └───────────────┘              └───────────────┘
            │
    ┌───────┴───────┐
    │               │
action=quarantine  action=delete
    │               │
    ▼               ▼
隔离文件          删除文件
```

### 3.5.6 隔离区安全措施

| 措施 | 说明 |
|------|------|
| **重命名** | 隔离文件使用 UUID 重命名，无扩展名 |
| **权限限制** | 隔离区目录权限设置为 0600（仅所有者可读写） |
| **元数据分离** | 文件和元数据分离存储 |
| **加密存储** | 可选：对隔离文件进行加密存储 |
| **定期清理** | 超过保留期的隔离文件自动删除 |

---

## 3.6 定时更新任务实现方案

### 3.6.1 方案选择

**采用 Rust 内置调度器**（tokio-cron-scheduler 或类似）

| 方案 | 优点 | 缺点 |
|------|------|------|
| Rust 内置调度 | 无需外部依赖，配置简单 | 需要应用进程保持运行 |
| systemd timer | 系统级调度，更可靠 | 跨平台兼容性差 |
| crontab | 成熟稳定 | 需要用户系统配置 |

**决策**：使用 Rust 内置调度

### 3.6.2 配置格式

```json
{
  "update": {
    "frequency": "daily",        // "daily" | "weekly" | "manual"
    "schedule_time": "03:30",    // 执行时间
    "timezone": "Asia/Shanghai"  // 时区
  }
}
```

### 3.6.3 Cron 表达式映射

| 频率 | Cron 表达式 |
|------|-------------|
| daily 03:30 | `30 3 * * *` |
| weekly 03:30 | `30 3 * * 0` |

### 3.6.4 实现流程

```
┌─────────────────────────────────────────────────────────────┐
│                    应用启动                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │  加载更新配置          │
                │  读取 schedule_time    │
                └───────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │  注册定时任务          │
                │  (内置调度器)           │
                └───────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │  等待触发时间          │
                └───────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │  执行 freshclam 更新   │
                │  记录更新结果          │
                └───────────────────────┘
```

### 3.6.5 手动触发与自动触发互斥

手动触发时检查是否有更新任务正在进行，避免冲突。

---

## 3.7 全盘扫描路径获取方案

### 3.7.1 路径获取方式

**方案**：读取 `/proc/mounts` 获取已挂载的文件系统

### 3.7.2 路径过滤规则

| 路径类型 | 是否扫描 | 说明 |
|----------|----------|------|
| /proc | ❌ | 进程虚拟文件系统 |
| /sys | ❌ | 内核虚拟文件系统 |
| /dev | ❌ | 设备文件 |
| /run | ❌ | 运行时数据 |
| /tmp | ⚠️ | 可选，临时文件 |
| /boot | ⚠️ | 可选，系统引导 |
| /home, /data, /mnt | ✅ | 用户数据 |

### 3.7.3 全盘扫描路径列表

```json
{
  "full_scan_paths": ["/", "/home", "/data", "/mnt"],
  "exclude_paths": ["/proc", "/sys", "/dev", "/run"]
}
```

---

## 3.8 ClamAV 编译与打包流程

### 3.8.1 ClamAV 动态库编译

**编译目标**：生成 libclamav.so 及相关依赖库

```bash
# 下载 ClamAV 源码
wget https://www.clamav.net/downloads/production/clamav-1.0.0.tar.gz

# 解压
tar -xzf clamav-1.0.0.tar.gz
cd clamav-1.0.0

# 配置编译选项（生成动态库）
./configure \
    --prefix=/usr/local \
    --disable-clamav \
    --disable-milter \
    --enable-shared \
    --disable-static

# 编译
make -j$(nproc)

# 提取动态库到打包目录
cp .libs/libclamav.so* $TRIM_APPDEST/lib/
cp .libs/libclammspack.so* $TRIM_APPDEST/lib/   # 可选
cp .libs/libclamunrar_iface.so* $TRIM_APPDEST/lib/  # 可选

# 保留 freshclam 二进制
cp cli/freshclam $TRIM_APPDEST/bin/
```

### 3.8.2 飞牛应用打包结构

```
fnnas.clamav/                          # 应用根目录（使用 fnpack create 创建）
├── app/                              # 应用可执行文件目录 (TRIM_APPDEST)
│   ├── lib/                          # ClamAV 动态库文件
│   │   ├── libclamav.so            # ClamAV 核心引擎库
│   │   ├── libclammspack.so        # 压缩包支持库
│   │   └── libclamunrar.so         # RAR 包支持库
│   ├── bin/                          # 可执行文件
│   │   ├── freshclam               # ClamAV 更新命令
│   │   └── clamav-daemon           # Rust 守护进程
│   ├── ui/                           # Vue 前端文件
│   │   ├── index.html
│   │   ├── assets/
│   │   └── config                    # 前端配置
│   └── images/                       # 图标
│       ├── icon_64.png
│       └── icon_256.png
├── manifest                          # 应用清单文件
├── cmd/                              # 生命周期脚本
│   ├── main                          # 启停脚本
│   ├── install_init                  # 安装初始化
│   ├── install_callback              # 安装回调
│   ├── uninstall_init                # 卸载初始化
│   ├── uninstall_callback            # 卸载回调
│   ├── upgrade_init                  # 升级初始化
│   ├── upgrade_callback              # 升级回调
│   ├── config_init                   # 配置初始化
│   └── config_callback               # 配置回调
├── config/                           # 配置文件
│   ├── privilege                     # 应用权限
│   └── resource                      # 应用资源声明 (data-share)
├── wizard/                           # 用户向导（可选）
├── LICENSE                           # 许可证
├── ICON.PNG                          # 应用图标
└── ICON_256.PNG                      # 应用图标(大)
```

### 3.8.3 manifest 文件示例

```
appname=fnnas.clamav
version=1.0.0
platform=x86
display_name=ClamAV 杀毒
desc=基于 ClamAV 的病毒扫描应用
maintainer=Your Name
distributor=Your Company
desktop_uidir=ui
desktop_applaunchname=fnnas.clamav.Application
service_port=8080
source=thirdparty
```

**说明**：
- `platform`: x86（arch 已废弃，使用 platform 替代）
- `source`: 必须设置为 `thirdparty`

### 3.8.4 config/privilege 文件示例

```json
{
  "defaults": {
    "run-as": "package"
  },
  "username": "fnnas.clamav",
  "groupname": "fnnas.clamav"
}
```

### 3.8.5 app/ui/config 文件示例（应用入口配置）

```json
{
  ".url": {
    "fnnas.clamav.Application": {
      "title": "ClamAV 杀毒",
      "icon": "images/icon-{0}.png",
      "type": "url",
      "protocol": "http",
      "port": "8080",
      "url": "/",
      "allUsers": true
    }
  }
}
```

**说明**：
- 入口 key 必须使用 appname 为前缀：`fnnas.clamav.Application`
- 对应 manifest 中的 `desktop_applaunchname=fnnas.clamav.Application`
- 图标格式：`icon-{0}.png` → `icon-64.png` 或 `icon-256.png`

### 3.8.6 病毒库初始打包

### 3.9 FFI 引擎生命周期管理

**引擎状态管理**：

```
┌──────────┐    初始化     ┌──────────┐
│  未初始化  │ ──────────────▶│  已初始化  │
└──────────┘                └──────────┘
    ▲                            │
    │                            │ 释放
    │                            ▼
    │                    ┌──────────┐
    │                    │  已释放    │
    │                    └──────────┘
    │
    └──────────── 服务重启时复用引擎
```

**设计要点**：
- 引擎单例模式，服务启动时初始化
- 支持病毒库热重载（无需重启服务）
- 扫描任务排队，共享引擎实例
- 引擎异常时自动恢复机制

### 3.10 更新后的整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                        飞牛桌面                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │         WebView (Vue 前端 - dist 静态文件)            │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │  │
│  │  │ 首页/   │ │ 扫描/   │ │ 历史/   │ │ 设置/   │    │  │
│  │  │ 仪表盘  │ │ 进度    │ │ 记录    │ │ 配置    │    │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘    │  │
│  └───────┼──────────┼──────────┼──────────┼────────────┘  │
└──────────┼──────────┼──────────┼──────────┼───────────────┘
           │          │          │          │
           ▼          ▼          ▼          ▼
┌────────────────────────────────────────────────────────────┐
│                   index.cgi (Shell)                         │
│              转发请求 → Rust HTTP 服务                       │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│              Rust 守护进程 (systemd 服务)                  │
│              HTTP Server (Unix Socket / localhost)          │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  REST API / WebSocket                                 │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │ │
│  │  │ 扫描管理  │  │ 更新管理  │  │ 配置管理  │          │ │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘          │ │
│  │       │             │             │                 │ │
│  │  ┌────▼─────────────────▼──────────▼────┐          │ │
│  │  │        ClamAV FFI 绑定层            │          │ │
│  │  │        - 引擎生命周期管理              │          │ │
│  │  │        - 扫描任务队列                │          │ │
│  │  │        - 进度回调处理                │          │ │
│  │  └──────────────────┬────────────────────┘          │ │
│  └─────────────────────┼───────────────────────────────┘ │
└────────────────────────┼─────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│                    libclamav.so                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │病毒库加载  │  │文件扫描   │  │进度回调   │          │
│  │(内存)    │  │          │  │          │          │
│  └──────────┘  └──────────┘  └──────────┘          │
└────────────────────────────────────────────────────────────┘
```

---

## 4. 页面结构
│                        飞牛桌面                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │         WebView (Vue 前端 - dist 静态文件)            │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │  │
│  │  │ 首页/   │ │ 扫描/   │ │ 历史/   │ │ 设置/   │    │  │
│  │  │ 仪表盘  │ │ 进度    │ │ 记录    │ │ 配置    │    │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘    │  │
│  └───────┼──────────┼──────────┼──────────┼────────────┘  │
└──────────┼──────────┼──────────┼──────────┼───────────────┘
           │          │          │          │
           ▼          ▼          ▼          ▼
┌────────────────────────────────────────────────────────────┐
│                   index.cgi (Shell)                         │
│              转发请求 → Rust HTTP 服务                       │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│              Rust 守护进程 (systemd 服务)                  │
│              HTTP Server (Unix Socket / localhost)          │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  REST API                                            │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │ │
│  │  │ 扫描管理  │  │ 更新管理  │  │ 配置管理  │          │ │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘          │ │
│  │       │             │             │                 │ │
│  │  ┌────▼─────────────────▼──────────▼────┐          │ │
│  │  │        定时调度器                      │          │ │
│  │  └────┬──────────────────────────────────┘          │ │
│  │       │                                            │ │
│  │  ┌────▼─────────────────────────────────────┐      │ │
│  │  │       二进制调用层 (std::process)          │      │ │
│  │  └────┬─────────────────────────────────────┘      │ │
│  └───────┼───────────────────────────────────────────┘ │
└──────────┼───────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────────────┐
│                    ClamAV 二进制                           │
│  ┌──────────┐           ┌──────────┐                     │
│  │ clamscan │           │ freshclam │                     │
│  │ 扫描命令  │           │ 更新命令  │                     │
│  └──────────┘           └──────────┘                     │
└────────────────────────────────────────────────────────────┘
```

---

## 4. 页面结构

| 页面 | 功能 | 设计原则 |
|------|------|----------|
| **首页/仪表盘** | 显示当前状态、上次扫描时间、病毒库版本、快捷操作 | 简洁直观 |
| **扫描页面** | 扫描进度展示、控制按钮（暂停/继续/停止） | 信息密度适中 |
| **历史记录** | 扫描历史列表、详情查看 | 清晰易读 |
| **设置页面** | 更新频率、威胁处理方式等配置 | 简单明了 |

### 4.1 扫描进度显示

**简洁版（采用）**：
- 进度百分比
- 当前扫描文件
- 发现威胁数

---

## 5. 服务生命周期管理

### 5.1 飞牛应用启停集成

```
安装 → install_callback → 注册 systemd 服务
启用 → cmd/main start → 启动 systemd 服务
停用 → cmd/main stop → 停止 systemd 服务
卸载 → uninstall_callback → 注销 systemd 服务
```

### 5.2 脚本职责

| 脚本 | 功能 |
|------|------|
| `cmd/install_callback` | 注册 systemd 服务、创建数据目录 |
| `cmd/uninstall_callback` | 停止服务、注销 systemd、清理数据 |
| `cmd/main start` | 启动 Rust 守护进程 |
| `cmd/main stop` | 停止 Rust 守护进程 |
| `cmd/main status` | 检查服务运行状态 |

---

## 6. 数据存储规范

### 6.1 环境变量映射（飞牛系统提供）

| 环境变量 | 说明 | 应用内用途 |
|----------|------|-----------|
| `TRIM_APPDEST` | 应用可执行文件目录 (target) | 存放 clamscan、freshclam、Rust 守护进程 |
| `TRIM_PKGETC` | 配置文件目录 (etc) | 应用配置文件 |
| `TRIM_PKGVAR` | 动态数据目录 (var) | 扫描状态、PID 文件、日志 |
| `TRIM_PKGTMP` | 临时文件目录 (tmp) | 临时扫描文件 |
| `TRIM_PKGHOME` | 用户数据目录 (home) | 用户相关数据 |
| `TRIM_PKGMETA` | 元数据目录 (meta) | 元数据文件 |
| `TRIM_DATA_SHARE_PATHS` | 数据共享路径列表（冒号分隔） | 病毒库、隔离区、历史记录等 |

### 6.2 存储位置映射

| 数据类型 | 存储位置 | 环境变量/路径 |
|----------|----------|---------------|
| **应用可执行文件** | `TRIM_APPDEST/` | clamscan、freshclam、daemon |
| **应用配置** | `TRIM_PKGETC/settings.json` | 静态配置 |
| **扫描状态** | `TRIM_PKGVAR/scan_state.json` | 动态状态 |
| **历史记录** | `$DATA_SHARE/history.db` | 从 `TRIM_DATA_SHARE_PATHS` 获取 |
| **病毒库** | `$DATA_SHARE/clamav/` | 从 `TRIM_DATA_SHARE_PATHS` 获取 |
| **隔离区** | `$DATA_SHARE/quarantine/` | 从 `TRIM_DATA_SHARE_PATHS` 获取 |
| **前端文件** | `TRIM_APPDEST/ui/` | Vue 静态文件 |

**说明**：`$DATA_SHARE` 表示从 `TRIM_DATA_SHARE_PATHS` 环境变量解析出的第一个共享目录路径。

### 6.3 共享目录配置 (config/resource)

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

**应用名称**：`fnnas.clamav`（遵循飞牛命名规范）

### 6.4 应用启停脚本中的路径解析

```bash
# cmd/main 示例
#!/bin/bash

# 获取数据共享目录路径（第一个路径）
DATA_DIR="${TRIM_DATA_SHARE_PATHS%%:*}"

# ClamAV 二进制路径
CLAMSCAN_BIN="${TRIM_APPDEST}/bin/clamscan"
FRESHCLAM_BIN="${TRIM_APPDEST}/bin/freshclam"

# 病毒库路径
CLAMAV_DB_DIR="${DATA_DIR}/clamav"

# 隔离区路径
QUARANTINE_DIR="${DATA_DIR}/quarantine"

# 历史记录数据库
HISTORY_DB="${DATA_DIR}/history.db"

# 配置文件
SETTINGS_FILE="${TRIM_PKGETC}/settings.json"

# 状态文件
STATE_FILE="${TRIM_PKGVAR}/scan_state.json"

# Rust 守护进程
DAEMON_BIN="${TRIM_APPDEST}/bin/clamav-daemon"
```

---

## 6.5 数据库表结构设计

数据库文件：从 `TRIM_DATA_SHARE_PATHS` 解析的共享目录下的 `history.db`

### 6.3.1 扫描历史表 (scan_history)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 自增主键 |
| scan_id | TEXT UNIQUE | 扫描任务ID（如：scan_20250210_143022） |
| scan_type | TEXT | 扫描类型：full / custom |
| paths | TEXT | 扫描路径列表（JSON数组字符串） |
| status | TEXT | 状态：scanning / completed / stopped / error |
| start_time | INTEGER | 开始时间（Unix时间戳） |
| end_time | INTEGER | 结束时间（Unix时间戳） |
| total_files | INTEGER | 扫描文件总数 |
| scanned_files | INTEGER | 已扫描文件数 |
| threats_found | INTEGER | 发现威胁数 |
| error_message | TEXT | 错误信息（如有） |

### 6.3.2 威胁记录表 (threat_records)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 自增主键 |
| scan_id | TEXT | 关联的扫描ID |
| file_path | TEXT | 感染文件路径 |
| virus_name | TEXT | 病毒名称 |
| action_taken | TEXT | 处理方式：quarantined / deleted / ignored |
| action_time | INTEGER | 处理时间（Unix时间戳） |
| original_location | TEXT | 原始位置（隔离文件时记录） |
| file_hash | TEXT | 文件哈希（SHA-256，可选） |

### 6.3.3 更新历史表 (update_history)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 自增主键 |
| start_time | INTEGER | 开始时间（Unix时间戳） |
| end_time | INTEGER | 结束时间（Unix时间戳） |
| result | TEXT | 结果：success / failed / no_update |
| old_version | TEXT | 更新前版本（JSON字符串） |
| new_version | TEXT | 更新后版本（JSON字符串） |
| error_message | TEXT | 错误信息（如有） |

### 6.3.4 隔离区记录表 (quarantine_records)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 自增主键 |
| threat_id | INTEGER | 关联威胁记录ID（外键到 threat_records） |
| quarantine_path | TEXT | 隔离文件存储路径 |
| original_path | TEXT | 原始文件路径 |
| quarantined_time | INTEGER | 隔离时间（Unix时间戳） |
| file_size | INTEGER | 文件大小（字节） |
| restored | BOOLEAN | 是否已恢复 |
| restored_time | INTEGER | 恢复时间（如有） |

### 6.3.5 索引设计

```sql
-- 扫描历史索引
CREATE INDEX idx_scan_history_start_time ON scan_history(start_time DESC);
CREATE INDEX idx_scan_history_status ON scan_history(status);

-- 威胁记录索引
CREATE INDEX idx_threat_records_scan_id ON threat_records(scan_id);
CREATE INDEX idx_threat_records_action_time ON threat_records(action_time DESC);

-- 更新历史索引
CREATE INDEX idx_update_history_start_time ON update_history(start_time DESC);

-- 隔离记录索引
CREATE INDEX idx_quarantine_quarantined_time ON quarantine_records(quarantined_time DESC);
```

---

---

## 7. API 接口设计（草稿）

### 7.1 扫描相关

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /api/scan/start | 开始扫描（全盘/自定义路径） |
| POST | /api/scan/stop | 停止扫描 |
| POST | /api/scan/pause | 暂停扫描 |
| POST | /api/scan/resume | 恢复扫描 |
| GET | /api/scan/status | 获取扫描状态和进度 |
| GET | /api/scan/history | 获取扫描历史记录 |

#### POST /api/scan/pause

**功能**：暂停当前正在进行的扫描任务

**响应**：
```json
{
  "success": true,
  "message": "扫描已暂停",
  "paused_at": "2025-02-13T14:30:00Z",
  "current_progress": {
    "percent": 45,
    "current_file": "/data/documents/file.pdf",
    "scanned": 1234
  }
}
```

**错误响应**：
```json
{
  "success": false,
  "error": {
    "code": "SCAN_NOT_RUNNING",
    "message": "没有正在进行的扫描任务"
  }
}
```

#### POST /api/scan/resume

**功能**：恢复已暂停的扫描任务

**响应**：
```json
{
  "success": true,
  "message": "扫描已恢复",
  "resumed_at": "2025-02-13T14:35:00Z"
}
```

**错误响应**：
```json
{
  "success": false,
  "error": {
    "code": "SCAN_NOT_PAUSED",
    "message": "没有已暂停的扫描任务"
  }
}
```

### 7.1.5 威胁处理相关

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/threats | 获取威胁记录列表 |
| POST | /api/threats/{id}/handle | 处理单个威胁（隔离/删除/忽略） |
| POST | /api/threats/batch-handle | 批量处理威胁 |

#### GET /api/threats

**查询参数**：
- `scan_id`: 可选，筛选指定扫描的威胁
- `status`: 可选，筛选状态（pending / quarantined / deleted / ignored）
- `page`: 页码，默认 1
- `page_size`: 每页数量，默认 20

**响应**：
```json
{
  "total": 15,
  "items": [
    {
      "id": 1,
      "scan_id": "scan_20250210_143022",
      "file_path": "/data/downloads/infected.exe",
      "virus_name": "Trojan.Generic",
      "action_taken": "quarantined",
      "quarantine_uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "action_time": "2025-02-10T14:35:00Z"
    },
    {
      "id": 2,
      "scan_id": "scan_20250210_143022",
      "file_path": "/tmp/suspicious.doc",
      "virus_name": "Heuristics.Phishing",
      "action_taken": "none",
      "action_time": null
    }
  ]
}
```

#### POST /api/threats/{id}/handle

**请求**：
```json
{
  "action": "quarantine"   // "quarantine" | "delete" | "ignore"
}
```

**响应**：
```json
{
  "success": true,
  "threat": {
    "id": 2,
    "action_taken": "quarantined",
    "quarantine_uuid": "b2c3d4e5-f6a7-8901-bcde-f12345678901"
  }
}
```

### 7.1.6 隔离区管理相关

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/quarantine | 获取隔离文件列表 |
| GET | /api/quarantine/{uuid} | 获取隔离文件详情 |
| POST | /api/quarantine/{uuid}/restore | 恢复隔离文件 |
| DELETE | /api/quarantine/{uuid} | 永久删除隔离文件 |
| POST | /api/quarantine/cleanup | 清理过期隔离文件 |

#### GET /api/quarantine

**响应**：
```json
{
  "total": 5,
  "total_size_bytes": 5242880,
  "items": [
    {
      "uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "original_path": "/data/downloads/infected.exe",
      "original_name": "infected.exe",
      "file_size": 1024000,
      "virus_name": "Trojan.Generic",
      "quarantined_at": "2025-02-10T14:35:00Z",
      "scan_id": "scan_20250210_143022"
    }
  ]
}
```

#### POST /api/quarantine/{uuid}/restore

**响应**：
```json
{
  "success": true,
  "restored_to": "/data/downloads/infected.exe"
}
```

#### 7.1.1 POST /api/scan/start

**请求**：
```json
{
  "scan_type": "full",      // "full" 全盘扫描 | "custom" 自定义扫描
  "paths": ["/data"]        // 自定义扫描时的路径列表，全盘扫描可省略
}
```

**响应**：
```json
{
  "success": true,
  "scan_id": "scan_20250210_143022",
  "status": "scanning",
  "estimated_files": 2700
}
```

#### 7.1.2 GET /api/scan/status

**响应**：
```json
{
  "scan_id": "scan_20250210_143022",
  "status": "scanning",           // "idle" | "scanning" | "completed" | "stopped" | "error"
  "progress": {
    "percent": 45,
    "scanned": 1234,
    "estimated_total": 2700,
    "current_file": "/data/documents/file.pdf"
  },
  "threats": {
    "count": 3,
    "files": [
      {
        "path": "/tmp/eicar.com",
        "virus": "Eicar-Test-Signature",
        "action": "quarantined"
      }
    ]
  },
  "start_time": "2025-02-10T14:30:22Z",
  "elapsed_seconds": 180
}
```

### 7.2 更新相关

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /api/update/start | 手动触发病毒库更新 |
| GET | /api/update/status | 获取更新状态 |
| GET | /api/update/version | 获取当前病毒库版本 |
| GET | /api/update/history | 获取更新历史记录 |

#### 7.2.1 POST /api/update/start

**响应**：
```json
{
  "success": true,
  "status": "updating",
  "start_time": "2025-02-10T15:00:00Z"
}
```

#### 7.2.2 GET /api/update/status

**响应**：
```json
{
  "status": "idle",                    // "idle" | "updating" | "error"
  "current_version": {
    "daily": "27162",
    "main": "62",
    "bytecode": "335"
  },
  "last_update": "2025-02-10T03:30:00Z",
  "next_scheduled": "2025-02-11T03:30:00Z",
  "update_frequency": "daily"           // "daily" | "weekly" | "manual"
}
```

#### 7.2.3 GET /api/update/version

**响应**：
```json
{
  "version": {
    "daily": "27162",
    "main": "62",
    "bytecode": "335"
  },
  "age_days": 0.5                      // 病毒库年龄（天数）
}
```

#### 7.2.4 GET /api/update/history

**响应**：
```json
{
  "records": [
    {
      "time": "2025-02-10T03:30:00Z",
      "result": "success",
      "old_version": {"daily": "27160"},
      "new_version": {"daily": "27162"},
      "duration_seconds": 45
    }
  ],
  "total": 30,
  "page": 1,
  "page_size": 20
}
```

### 7.3 配置相关

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/config | 获取应用配置 |
| PUT | /api/config | 更新应用配置 |
| POST | /api/config/reset | 重置为默认配置 |

#### 7.3.1 GET /api/config

**响应**：
```json
{
  "scan": {
    "default_scan_type": "full",           // 默认扫描类型
    "exclude_paths": [                      // 排除路径
      "/proc",
      "/sys",
      "/dev"
    ],
    "max_file_size_mb": 100,               // 单文件最大扫描大小
    "scan_archives": true                   // 是否扫描压缩包
  },
  "threat": {
    "action": "quarantine",                 // "quarantine" | "delete" | "none"
    "auto_action": false                    // 是否自动处理威胁
  },
  "update": {
    "frequency": "daily",                   // "daily" | "weekly" | "manual"
    "schedule_time": "03:30",               // 定时更新时间
    "auto_check": true                      // 是否自动检查更新
  },
  "history": {
    "retention_days": 90,                   // 历史记录保留天数
    "max_records": 1000                     // 最大记录数
  }
}
```

#### 7.3.2 PUT /api/config

**请求**（支持部分更新）：
```json
{
  "scan": {
    "max_file_size_mb": 200
  },
  "threat": {
    "action": "delete",
    "auto_action": true
  }
}
```

**响应**：返回更新后的完整配置（同 GET /api/config）

---

## 7.4 错误响应格式

所有 API 错误统一格式：

```json
{
  "success": false,
  "error": {
    "code": "SCAN_ALREADY_RUNNING",
    "message": "扫描任务已在运行中",
    "details": {}
  }
}
```

### 常见错误码

| 错误码 | 说明 |
|--------|------|
| `SCAN_ALREADY_RUNNING` | 扫描任务已在运行 |
| `SCAN_NOT_FOUND` | 扫描任务不存在 |
| `SCAN_NOT_RUNNING` | 没有正在进行的扫描任务 |
| `SCAN_NOT_PAUSED` | 没有已暂停的扫描任务 |
| `SCAN_ENGINE_BUSY` | 扫描引擎忙碌，请求已排队 |
| `SCAN_ENGINE_ERROR` | 扫描引擎错误 |
| `ENGINE_INIT_FAILED` | 引擎初始化失败 |
| `ENGINE_CRASHED` | 引擎崩溃，服务正在恢复 |
| `INVALID_PATH` | 无效的扫描路径 |
| `UPDATE_IN_PROGRESS` | 更新任务正在进行 |
| `QUARANTINE_FAILED` | 隔离操作失败 |
| `INVALID_CONFIG` | 配置参数无效 |

---

## 8. 下一步工作

### 8.1 已完成设计
- [x] 完善 API 接口设计（扫描、更新、配置、威胁、隔离区）
- [x] 设计数据库表结构（扫描历史、威胁记录、更新历史、隔离记录）
- [x] 确定Rust服务与ClamAV的集成方案（采用FFI动态库调用）
- [x] 扫描控制方案设计（支持停止、暂停、恢复）
- [x] 扫描进度反馈方案设计（实时回调机制）
- [x] 扫描任务队列设计
- [x] 引擎异常恢复设计
- [x] 隔离区管理方案设计（目录结构、处理流程、API）
- [x] 定时更新任务实现方案（Rust 内置调度器）
- [x] 全盘扫描路径获取方案（读取 /proc/mounts）
- [x] ClamAV 编译与打包流程设计
- [x] 架构图更新（更新为FFI调用方式）
- [x] 引擎生命周期管理设计

### 8.2 待实现任务

#### FFI 相关（核心功能）
- [ ] ClamAV 动态库编译脚本
- [ ] Rust FFI 绑定层实现
- [ ] 引擎单例和生命周期管理
- [ ] 扫描任务队列实现
- [ ] 实时进度回调机制
- [ ] 暂停/恢复扫描功能
- [ ] 病毒库热重载
- [ ] 引擎异常恢复机制

#### 其他
- [ ] 制定详细实施计划

---

## 9. 设计完成摘要

### 9.1 已完成设计项

| 分类 | 设计项 | 状态 |
|------|--------|------|
| 需求 | 功能需求 | ✅ |
| 需求 | 非功能需求 | ✅ |
| 架构 | 整体架构 | ✅ |
| 架构 | 技术栈 | ✅ |
| 集成 | ClamAV 集成方案（FFI动态库调用） | 🔄 设计完成，待实现 |
| 集成 | 扫描控制方案 | 🔄 设计完成，待实现 |
| 集成 | 进度反馈方案 | 🔄 设计完成，待实现 |
| 集成 | 扫描任务队列 | 🔄 设计完成，待实现 |
| 集成 | 引擎异常恢复 | 🔄 设计完成，待实现 |
| 集成 | 引擎生命周期管理 | 🔄 设计完成，待实现 |
| 管理 | 隔离区管理方案 | ✅ |
| 管理 | 定时更新方案 | ✅ |
| 管理 | 全盘扫描方案 | ✅ |
| 打包 | ClamAV 编译打包 | 🔄 设计完成，待实现 |
| 接口 | REST API 设计 | ✅ |
| 数据 | 数据库表结构 | ✅ |
| 数据 | 存储规范 | ✅ |
| 生命周期 | 服务启停管理 | ✅ |

### 9.2 图例
- ✅ 已完成
- 🔄 设计完成，待实现

---

**变更记录**

| 日期 | 版本 | 变更内容 | 作者 |
|------|------|----------|------|
| 2026-02-13 | 1.0 | 补充 FFI 相关缺失设计：新增扫描任务队列设计、引擎异常恢复设计；更新 API 接口（添加 pause/resume）；更新错误码；修复重复架构图问题 | ecJon |
| 2026-02-13 | 0.9 | ClamAV 集成方案改为 FFI 动态库调用：更新方案选择、文件布局、扫描流程、扫描控制、进度反馈设计；更新架构图；新增引擎生命周期管理设计 | ecJon |
| 2026-02-10 | 0.8 | 再次核对飞牛规范：修正 manifest 中 arch→platform，添加 app/ui/config 应用入口配置示例 | ecJon |
| 2026-02-10 | 0.7 | 核对飞牛开发者文档规范，更新数据存储规范（使用 TRIM_* 环境变量）、更新应用打包结构、应用名称改为 fnnas.clamav | ecJon |
| 2026-02-10 | 0.6 | 完成所有设计方案（定时更新、全盘扫描、编译打包、架构图更新），添加设计完成摘要 | ecJon |
| 2026-02-10 | 0.5 | 完成隔离区管理方案设计（目录结构、处理流程、威胁/隔离区 API） | ecJon |
| 2026-02-10 | 0.4 | 完善 API 接口设计（更新、配置、错误码），完成数据库表结构设计 | ecJon |
| 2026-02-10 | 0.3 | 确定扫描控制方案（不支持暂停/继续），完成进度估算方案，完善扫描 API 设计 | ecJon |
| 2026-02-10 | 0.2 | 确定ClamAV集成方案（二进制调用方式） | ecJon |
| 2026-02-10 | 0.1 | 初始版本，完成需求讨论和架构设计 | ecJon |
