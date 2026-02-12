# ClamAV 二进制文件安装指南

## 快速方法：从系统安装复制（推荐）

### 1. 安装 ClamAV
```bash
sudo apt-get update
sudo apt-get install -y clamav
```

### 2. 复制二进制到项目
```bash
cd /home/test/code/example
./copy-clamav.sh
```

### 3. 验证
```bash
ls -lh app/bin/
# 应该看到 clamscan 和 freshclam
```

### 4. 重新打包
```bash
./build.sh
```

---

## 完整方法：从源码编译

如果系统没有合适的 ClamAV 包，可以从源码编译：

### 1. 安装编译依赖
```bash
sudo apt-get update
sudo apt-get install -y build-essential autoconf automake libtool pkg-config \
    libssl-dev libcurl4-openssl-dev libjson-c-dev libpcre2-dev \
    libcheck-dev zlib1g-dev
```

### 2. 编译 ClamAV
```bash
cd /home/test/code/example
./build-clamav.sh
```

这个过程可能需要 10-30 分钟。

### 3. 验证和打包
同上。

---

## 跳过 ClamAV（仅测试 API）

如果只想测试 API 接口而不需要实际扫描功能，可以跳过此步骤。

扫描 API 会返回错误："ClamAV 扫描程序不存在"，但其他功能正常。

---

## 验证安装

安装后测试：

```bash
# 测试 clamscan
app/bin/clamscan --version

# 测试 freshclam
app/bin/freshclam --version
```

预期输出类似：
```
ClamAV 0.103.0
...
```
