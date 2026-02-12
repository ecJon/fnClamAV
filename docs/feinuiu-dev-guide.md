# 飞牛 fnOS 开发指南

> 文档来源：https://developer.fnnas.com/docs/category/开发指南
> 更新日期：2026-02-12

---

## 目录

1. [快速开始](#快速开始)
2. [架构概述](#架构概述)
3. [Manifest](#manifest)
4. [应用权限](#应用权限)
5. [应用资源](#应用资源)
6. [Native 应用构建](#native-应用构建)

---

## 快速开始

### 准备工作

**系统要求：**
- 系统版本：飞牛 fnOS 0.9.27 及以上版本
- 存储空间：至少创建一个存储空间，可用于安装应用
- 管理员权限：拥有该设备的管理权限

**系统架构：**
- Linux 内核版本：6.12.18
- 系统架构支持：仅支持 x86_64 (AMD64) 设备

**应用技术栈：**
- 服务开发语言：Node.js、Python、Java、Go 以及 Linux 运行时支持的其他语言
- 前端开发语言：Html/Javascript/CSS

**CLI 工具：**
- `fnpack` - 应用打包工具
- `appcenter-cli` - 应用中心命令行工具（已在飞牛 fnOS 环境中预装）

### 创建应用流程

开发流程图：
```
制作 web 页面 → fnpack 创建项目 → 制作 icon 图标 → 编写 manifest
→ 迁移 web 应用 → 配置权限 → 配置应用入口 → 编写生命周期脚本
→ 编译打包
```

#### 1. fnpack 工具创建应用项目

```bash
# 创建独立项目
fnpack create App.Native.HelloFnosAppCenter
```

创建后的目录结构：

```
App.Native.HelloFnosAppCenter
├── app/               # 应用文件目录
│   ├── server/        # 后台服务程序目录
│   ├── ui/            # 应用入口及视图
│   │   ├── config/    # 应用入口配置文件
│   │   └── images/    # 应用图标资源
│   └── www/           # 应用 web 资源目录
├── cmd/               # 应用生命周期管理脚本
│   ├── config_callback
│   ├── config_init
│   ├── install_callback
│   ├── install_init
│   ├── main
│   ├── uninstall_callback
│   ├── uninstall_init
│   ├── upgrade_callback
│   └── upgrade_init
├── config/            # 应用配置目录
│   ├── privilege      # 应用权限配置
│   └── resource       # 应用资源配置
├── ICON_256.PNG       # 256*256 图标文件
├── ICON.PNG           # 64*64 图标文件
├── manifest           # 应用基本信息描述文件
└── wizard/            # 应用向导目录
```

#### 2. 制作包文件 icon 图标

- 制作 256x256 和 64x64 的图标文件
- 命名为 `ICON_256.PNG` 和 `ICON.PNG`，放置在项目根目录
- 在 `app/ui/images` 目录下放置小写命名的图标：`icon_256.png` 和 `icon_64.png`

#### 3. 编写 manifest 文件

```
appname               = App.Native.HelloFnosAppCenter
version               = 1.0.0
display_name          = 教学案例
desc                  = 方便开发者快速了解应用开发及应用封包流程
platform              = x86
source                = thirdparty
maintainer            = MR_XIAOBO
distributor           = MR_XIAOBO
desktop_uidir         = ui
desktop_applaunchname = App.Native.HelloFnosAppCenter.Application
```

#### 4. 配置应用权限

`config/privilege` 文件：

```json
{
    "defaults": {
        "run-as": "package"
    }
}
```

> 注意：省略 `username` 和 `groupname`，系统会自动使用 `appname` 创建用户和用户组

#### 5. 配置应用入口

`app/ui/config` 文件：

```json
{
    ".url": {
        "App.Native.HelloFnosAppCenter.Application": {
            "title": "应用中心案例",
            "icon": "images/icon-{0}.png",
            "type": "iframe",
            "protocol": "http",
            "url": "/cgi/ThirdParty/App.Native.HelloFnosAppCenter/index.cgi/",
            "allUsers": true
        }
    }
}
```

**字段说明：**
- `title` - 入口的显示标题
- `icon` - 图标文件路径，`{0}` 会被替换为图标尺寸（64 或 256）
- `type` - 入口类型：`url`（浏览器新标签）或 `iframe`（桌面窗口）
- `protocol` - 访问协议：`http` 或 `https`
- `url` - 访问路径
- `allUsers` - `true` 表示所有用户可访问，`false` 表示仅管理员可访问

#### 6. 编写应用生命周期脚本

`cmd/main` 文件：

```bash
#!/bin/bash
case $1 in
    start)
        # 启动应用的命令，成功返回 0，失败返回 1
        exit 0
        ;;
    stop)
        # 停止应用的命令，成功返回 0，失败返回 1
        exit 0
        ;;
    status)
        # 检查应用运行状态，运行中返回 0，未运行返回 3
        exit 0
        ;;
    *)
        exit 1
        ;;
esac
```

**状态返回值：**
- `exit 0` - 成功或应用运行中
- `exit 1` - 失败
- `exit 3` - 应用未运行

#### 7. 编译应用包文件

```bash
# 前往项目根目录
cd App.Native.HelloFnosAppCenter

# 编译打包
fnpack build
```

如果环境变量未配置，可直接执行：

```bash
# Linux 环境
./fnpack-1.0.4-linux-amd64 build

# MacOS 环境
./fnpack-1.0.4-darwin-amd64 build

# Windows 环境
.\fnpack-1.0.4-windows-amd64 build
```

---

---

## 架构概述

### 应用目录结构

当应用安装到飞牛 fnOS 系统后，会在系统中创建如下目录结构：

```
/var/apps/[appname]
├── cmd
│   ├── install_callback
│   ├── install_init
│   ├── main
│   ├── uninstall_callback
│   ├── uninstall_init
│   ├── upgrade_init
│   ├── upgrade_callback
│   ├── config_init
│   └── config_callback
├── config
│   ├── privilege
│   └── resource
├── ICON_256.PNG
├── ICON.PNG
├── LICENSE
├── manifest
├── etc -> /vol[volume_number]/@appconf/[appname]
├── home -> /vol[volume_number]/@apphome/[appname]
├── target -> /vol[volume_number]/@appcenter/[appname]
├── tmp -> /vol[volume_number]/@apptemp/[appname]
├── var -> /vol[volume_number]/@appdata/[appname]
├── shares
│   ├── datashare1 -> /vol[volume_number]/@appshare/datashare1
│   └── datashare2 -> /vol[volume_number]/@appshare/datashare2
└── wizard
    ├── install
    ├── uninstall
    ├── upgrade
    └── config
```

### 核心文件说明

**应用标识文件：**
- `manifest` - 应用的"身份证"，定义了应用的基本信息和运行属性
- `config/privilege` - 应用的"权限清单"，声明应用需要哪些系统权限
- `config/resource` - 应用的"能力声明"，定义应用可以使用的扩展功能

**界面资源：**
- `ICON.PNG` - 应用中心显示的小图标（64x64 像素）
- `ICON_256.PNG` - 应用详情页显示的大图标（256x256 像素）
- `LICENSE` - 用户安装前需要同意的隐私协议（可选）

### 目录功能说明

**开发者定义目录：**
- `cmd` - 存放应用生命周期管理的脚本文件
- `wizard` - 存放用户交互向导的配置文件

**系统自动创建目录：**
- `target` - 应用可执行文件的存放位置
- `etc` - 静态配置文件存放位置
- `var` - 运行时动态数据存放位置
- `tmp` - 临时文件存放位置
- `home` - 用户数据文件存放位置
- `shares` - 数据共享目录（根据 resource 配置自动创建）

### 应用生命周期管理

#### 应用安装流程

安装过程分为三个阶段：安装前准备、文件解压、安装后处理。

```
install_init -> 解压安装 target/ 目录 -> install_callback
```

#### 应用卸载流程

卸载应用时，系统会删除以下目录：`target`、`tmp`、`home`、`etc`，但会保留 `var` 和 `shares` 目录（保护用户数据）。

```
uninstall_init -> 删除应用目录文件/软链 -> uninstall_callback
```

如果卸载时应用仍在运行：
```
main stop -> uninstall_init -> 删除应用目录文件/软链 -> uninstall_callback
```

#### 应用更新流程

```
upgrade_init -> uninstall_init + uninstall_callback -> install_init + install_callback -> upgrade_callback
```

如果更新时应用正在运行：
```
main stop -> upgrade_init -> ... -> upgrade_callback -> main start
```

#### 应用配置流程

```
config_init -> 更新环境变量 -> config_callback
```

### 应用运行状态管理

典型的 `cmd/main` 脚本结构：

```bash
#!/bin/bash
case $1 in
    start)
        # 启动应用的命令，成功返回 0，失败返回 1
        exit 0
        ;;
    stop)
        # 停止应用的命令，成功返回 0，失败返回 1
        exit 0
        ;;
    status)
        # 检查应用运行状态，运行中返回 0，未运行返回 3
        exit 0
        ;;
    *)
        exit 1
        ;;
esac
```

**状态返回值：**
- `exit 0` - 应用正在运行
- `exit 3` - 应用未运行

### 应用错误异常展示处理 (V1.1.8+)

通过向 `TRIM_TEMP_LOGFILE` 环境变量写入错误信息并返回错误码 `1`，系统会自动将错误日志在前端展示。

```bash
echo "Error: Something went wrong" > $TRIM_TEMP_LOGFILE
exit 1
```

**注意事项：**
- 请不要使用 `echo` 直接输出错误，而是写入 `TRIM_TEMP_LOGFILE`
- 如果不写入 `TRIM_TEMP_LOGFILE` 直接 `exit 1`，系统将显示"执行XX脚本出错且原因未知"

---

## Manifest

manifest 文件必须放在应用包的根目录下，文件名为 `manifest`（没有扩展名）。

### 基本信息字段

| 字段 | 说明 | 示例 |
|------|------|------|
| `appname` | 应用的唯一标识符 | `fnnas.clamav` |
| `version` | 应用版本号 | `1.0.0` |
| `display_name` | 显示名称 | `ClamAV 杀毒软件` |
| `desc` | 应用介绍 | `基于 ClamAV 的病毒扫描和防护软件` |

### 系统要求字段

| 字段 | 说明 | 值 |
|------|------|-----|
| `platform` | 架构类型 | `x86`（默认）、`arm`、`loongarch`、`risc-v`、`all` |
| `source` | 应用来源 | 固定值：`thirdparty` |

> **注意：** `arch` 字段已废弃，应使用 `platform`

### 开发者信息字段

| 字段 | 说明 |
|------|------|
| `maintainer` | 应用开发者或开发团队名称 |
| `maintainer_url` | 开发者网站或联系方式 |
| `distributor` | 应用发布者 |
| `distributor_url` | 发布者网站 |

### 安装和运行控制字段

| 字段 | 说明 |
|------|------|
| `os_min_version` | 支持的最低系统版本 |
| `os_max_version` | 支持的最高系统版本 |
| `ctl_stop` | 是否显示启动/停止功能，默认 `true` |
| `install_type` | 安装类型，`root` 表示安装到系统分区 |
| `install_dep_apps` | 依赖应用列表，格式：`app1>2.2.2:app2:app3` |

### 用户界面字段

| 字段 | 说明 |
|------|------|
| `desktop_uidir` | UI 组件目录路径，默认 `ui` |
| `desktop_applaunchname` | 应用中心启动入口 |

### 端口管理字段

| 字段 | 说明 |
|------|------|
| `service_port` | 应用监听的端口号 |
| `checkport` | 是否启用端口检查，默认 `true` |

### 权限控制字段

| 字段 | 说明 |
|------|------|
| `disable_authorization_path` | 是否禁用授权目录功能，默认 `false` |

### 更新字段

| 字段 | 说明 |
|------|------|
| `changelog` | 应用更新日志 |

### 示例 manifest

```
appname               = fnnas.clamav
version               = 1.0.0
display_name          = ClamAV 杀毒软件
desc                  = 基于 ClamAV 的病毒扫描和防护软件
platform              = x86
source                = thirdparty
maintainer            = fnOS
distributor           = ClamAV Team
desktop_uidir         = ui
desktop_applaunchname = fnnas.clamav.Application
category              = security
```

---

## 应用权限

在 `config/privilege` 文件中定义应用运行时的权限级别和用户身份。

### 默认权限模式

大多数应用使用默认权限模式（应用用户运行），这是最安全的运行方式。

#### 用户配置示例

```json
{
    "defaults": {
        "run-as": "package"
    },
    "username": "myapp_user",
    "groupname": "myapp_group"
}
```

**字段说明：**
- `username` - 应用专用用户名，默认为 manifest 中的 `appname`
- `groupname` - 应用专用用户组名，默认为 manifest 中的 `appname`
- `run-as` - 运行身份，默认为 `package`（应用用户）

> **重要：** 如果未指定用户名和组名，系统会自动使用应用名称（对应 manifest 中的 `appname` 字段）创建用户和用户组。

### Root 权限模式

> Root 权限模式仅适用于飞牛官方合作的企业开发者。

#### 配置方式

```json
{
    "defaults": {
        "run-as": "root"
    },
    "username": "myapp_user",
    "groupname": "myapp_group"
}
```

启用 root 权限后：
- 应用脚本以 root 身份执行
- 应用进程可以以 root 身份或指定的应用用户身份运行
- 应用文件的所有者变为 root 用户

### 外部文件访问权限

应用默认无法访问用户的个人文件，用户需要在应用设置中明确授权。

**授权方式：**
1. 用户在应用设置页面中选择要授权的目录或文件
2. 通过 `config/resource` 的 `data-share` 设置默认的共享目录

### 权限检查示例

```bash
#!/bin/bash
echo "当前运行用户: $TRIM_RUN_USERNAME"
echo "应用专用用户: $TRIM_USERNAME"

if [ "$TRIM_RUN_USERNAME" = "root" ]; then
    echo "应用以 root 权限运行"
else
    echo "应用以应用用户权限运行"
fi
```

---

## 应用资源

在 `config/resource` 文件中声明应用需要的扩展能力。

### 数据共享 (data-share)

数据共享功能允许应用与用户共享特定的数据目录。

#### 配置示例

```json
{
    "data-share": {
        "shares": [
            {
                "name": "documents",
                "permission": {
                    "rw": ["myapp_user"]
                }
            },
            {
                "name": "documents/backups",
                "permission": {
                    "ro": ["myapp_user"]
                }
            }
        ]
    }
}
```

**权限类型：**
- `rw` - 读写权限：应用可以读取和修改文件
- `ro` - 只读权限：应用只能读取文件

### 系统集成 (usr-local-linker)

系统集成功能允许应用在启动时创建软链接到系统目录。

#### 配置示例

```json
{
    "usr-local-linker": {
        "bin": [
            "bin/myapp-cli",
            "bin/myapp-server"
        ],
        "lib": [
            "lib/mylib.so",
            "lib/mylib.a"
        ],
        "etc": [
            "etc/myapp.conf",
            "etc/myapp.d/default.conf"
        ]
    }
}
```

**链接说明：**
- `bin` - 可执行文件链接到 `/usr/local/bin/`
- `lib` - 库文件链接到 `/usr/local/lib/`
- `etc` - 配置文件链接到 `/usr/local/etc/`

### Docker 项目支持 (docker-project)

Docker 项目支持让应用可以基于 Docker Compose 运行。

#### 配置示例

```json
{
    "docker-project": {
        "projects": [
            {
                "name": "myapp-stack",
                "path": "docker"
            }
        ]
    }
}
```

---

## Native 应用构建

### 应用打包目录结构

```
myapp/
├── app/
│   └── server/          # 可执行文件和依赖
├── manifest             # 应用清单
├── cmd/
│   └── main             # 启停脚本
├── config/
│   ├── privilege        # 权限配置
│   └── resource         # 资源配置
├── ui/
│   ├── config           # UI 配置
│   └── images/          # 图标资源
├── wizard/
│   ├── install          # 安装向导
│   ├── uninstall        # 卸载向导
│   ├── upgrade          # 更新向导
│   └── config           # 配置向导
├── LICENSE              # 许可协议
├── ICON.PNG             # 64x64 图标
└── ICON_256.PNG         # 256x256 图标
```

### 权限配置示例

Native 应用的典型权限配置：

```json
{
    "defaults": {
        "run-as": "package"
    },
    "username": "fnnas.myapp",
    "groupname": "fnnas.myapp"
}
```

### 启停脚本示例

```bash
#!/bin/bash
LOG_FILE="${TRIM_PKGVAR}/info.log"
PID_FILE="${TRIM_PKGVAR}/app.pid"

# 启动命令
CMD="${TRIM_APPDEST}/server/myapp --port 8080"

start_process() {
    if status; then
        return 0
    fi
    bash -c "${CMD}" >> ${LOG_FILE} 2>&1 &
    printf "%s" "$!" > ${PID_FILE}
    return 0
}

stop_process() {
    if [ -r "${PID_FILE}" ]; then
        pid=$(head -n 1 "${PID_FILE}" | tr -d '[:space:]')
        kill -TERM ${pid}
        sleep 1
        rm -f "${PID_FILE}"
    fi
    return 0
}

status() {
    if [ -f "${PID_FILE}" ]; then
        pid=$(head -n 1 "${PID_FILE}" | tr -d '[:space:]')
        if kill -0 "${pid}" 2>/dev/null; then
            return 0
        else
            rm -f "${PID_FILE}"
        fi
    fi
    return 1
}

case $1 in
    start)
        start_process
        exit $?
        ;;
    stop)
        stop_process
        exit $?
        ;;
    status)
        if status; then
            exit 0
        else
            exit 3
        fi
        ;;
    *)
        exit 1
        ;;
esac
```

### UI 配置示例

`ui/config` 文件：

```json
{
    ".url": {
        "fnnas.myapp.Application": {
            "title": "My Application",
            "icon": "images/icon_{0}.png",
            "type": "url",
            "protocol": "http",
            "port": "8080"
        }
    }
}
```

### 打包命令

```bash
# 创建应用打包目录
fnpack create fnnas.myapp

# 打包成 fpk
cd fnnas.myapp
fnpack build
```

---

## 重要注意事项

### 1. manifest 配置

- ❌ 已废弃：`arch=x86_64`
- ✅ 正确：`platform=x86`（默认值）

### 2. privilege 配置

- 用户名和组名应与 `appname` 保持一致
- 避免使用包含特殊字符（如点号）的用户名
- 如果省略 `username` 和 `groupname`，系统会自动使用 `appname`

### 3. 目录权限

- `target` 目录：应用可执行文件存放位置
- `var` 目录：运行时数据，卸载时保留
- `shares` 目录：数据共享，根据 resource 配置创建

### 4. 错误处理

- 使用 `TRIM_TEMP_LOGFILE` 记录错误信息
- 返回适当的退出码（0 成功，1 失败，3 未运行）

---

## 参考资源

- 官方文档：https://developer.fnnas.com/docs/guide
- 开发指南：https://developer.fnnas.com/docs/category/开发指南
