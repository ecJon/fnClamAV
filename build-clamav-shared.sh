#!/bin/bash
set -e

# ClamAV 动态库编译脚本 (FFI 方式)
# 用法: ./build-clamav-shared.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAMAV_VERSION="1.0.0"
CLAMAV_BUILD_DIR="${PROJECT_DIR}/clamAV"
CLAMAV_OUTPUT_DIR="${PROJECT_DIR}/app/lib"           # 动态库输出目录
CLAMAV_BIN_DIR="${PROJECT_DIR}/app/bin"              # freshclam 输出目录
CLAMAV_LIB_DIR="${PROJECT_DIR}/app/lib/clamav"    # 预置病毒库目录

echo "======================================"
echo " Building ClamAV ${CLAMAV_VERSION} (Shared Library for FFI)"
echo "======================================"

# 创建目录
mkdir -p "${CLAMAV_BUILD_DIR}"
mkdir -p "${CLAMAV_OUTPUT_DIR}"
mkdir -p "${CLAMAV_BIN_DIR}"
mkdir -p "${CLAMAV_LIB_DIR}"

# 检查是否已有编译好的二进制
if [ -f "${CLAMAV_BIN_DIR}/freshclam" ] && [ -f "${CLAMAV_OUTPUT_DIR}/libclamav.so" ]; then
    echo "✅ ClamAV binaries already exist"
    echo "   To rebuild, remove them first:"
    echo "   rm ${CLAMAV_BIN_DIR}/freshclam ${CLAMAV_OUTPUT_DIR}/libclamav.so*"
    exit 0
fi

# 下载 ClamAV 源码
CLAMAV_TARBALL="${CLAMAV_BUILD_DIR}/clamav-${CLAMAV_VERSION}.tar.gz"
if [ ! -f "${CLAMAV_TARBALL}" ]; then
    echo "📥 Downloading ClamAV ${CLAMAV_VERSION} source..."
    wget -O "${CLAMAV_TARBALL}" "https://www.clamav.net/downloads/production/clamav-${CLAMAV_VERSION}.tar.gz"
fi

# 解压
CLAMAV_SRC_DIR="${CLAMAV_BUILD_DIR}/clamav-${CLAMAV_VERSION}"
if [ ! -d "${CLAMAV_SRC_DIR}" ]; then
    echo "📦 Extracting..."
    tar -xzf "${CLAMAV_TARBALL}" -C "${CLAMAV_BUILD_DIR}"
fi

# 进入源码目录
cd "${CLAMAV_SRC_DIR}"

# 检查依赖
echo "🔍 Checking dependencies..."
missing_deps=()

for cmd in gcc g++ make autoconf automake libtool pkg-config; do
    if ! command -v $cmd &> /dev/null; then
        missing_deps+=($cmd)
    fi
done

if [ ${#missing_deps[@]} -gt 0 ]; then
    echo "❌ Missing dependencies: ${missing_deps[*]}"
    echo "   Please install them first:"
    echo "   sudo apt-get install build-essential autoconf automake libtool pkg-config"
    exit 1
fi

if ! pkg-config --exists openssl 2>/dev/null; then
    echo "⚠️  libssl not found. Installing..."
    sudo apt-get install -y libssl-dev || true
fi

# 配置编译选项（生成动态库）
echo "⚙️  Configuring ClamAV for shared library..."
if [ ! -f "Makefile" ]; then
    ./configure \
        --prefix=/usr/local \
        --disable-clamscan \
        --disable-clamdscan \
        --disable-milter \
        --disable-freshclam \
        --disable-clamsubmit \
        --disable-sigtool \
        --disable-clambc \
        --disable-clamscan \
        --enable-shared \
        --disable-static \
        --with-libjson=no \
        --without-libpcre2 \
        --without-libpcre2 \
        --disable-zlib-vcheck \
        --disable-llvm \
        --disable-experimental \
        || {
        echo ""
        echo "❌ Configure failed!"
        echo ""
        echo "Missing dependencies? Install them:"
        echo "   sudo apt-get install \\"
        echo "    build-essential \\"
        echo "    autoconf \\"
        echo "    automake \\"
        echo "    libtool \\"
        echo "    pkg-config \\"
        echo "    libssl-dev \\"
        echo "    libcurl4-openssl-dev \\"
        echo "    libjson-c-dev \\"
        echo "    zlib1g-dev"
        exit 1
    }
fi

# 编译
echo "🔨 Compiling (this may take a while)..."
make -j$(nproc)

# 提取动态库到输出目录
echo "📋 Copying shared libraries..."
find . -name "libclamav.so*" -type f -exec cp {} "${CLAMAV_OUTPUT_DIR}/" \;
find . -name "libclammspack.so*" -type f -exec cp {} "${CLAMAV_OUTPUT_DIR}/" \; 2>/dev/null || true
find . -name "libclamunrar_iface.so*" -type f -exec cp {} "${CLAMAV_OUTPUT_DIR}/" \; 2>/dev/null || true
find . -name "libclamunrar.so*" -type f -exec cp {} "${CLAMAV_OUTPUT_DIR}/" \; 2>/dev/null || true

# 设置软链接（so 版本）
cd "${CLAMAV_OUTPUT_DIR}"
for lib in libclamav.so libclammspack.so libclamunrar_iface.so libclamunrar.so; do
    if [ -f "${lib}".* ]; then
        real_lib=$(ls ${lib}.* | head -1)
        ln -sf "$(basename "${real_lib}")" "${lib}"
    fi
done

# 编译 freshclam（需要保留用于病毒库更新）
echo ""
echo "🔨 Compiling freshclam..."
cd "${CLAMAV_SRC_DIR}/freshclam"
if [ ! -f "Makefile" ]; then
    ./configure --prefix=/usr/local || exit 1
fi
make -j$(nproc)

echo "📋 Copying freshclam binary..."
cp "${CLAMAV_SRC_DIR}/freshclam/freshclam" "${CLAMAV_BIN_DIR}/"
chmod +x "${CLAMAV_BIN_DIR}/freshclam"

# 验证
echo ""
echo "🔍 Verifying..."
if [ -f "${CLAMAV_OUTPUT_DIR}/libclamav.so" ]; then
    echo "✅ libclamav.so: $(file "${CLAMAV_OUTPUT_DIR}/libclamav.so")"
    ls -lh "${CLAMAV_OUTPUT_DIR}"/libclamav.so*
else
    echo "❌ libclamav.so not found!"
fi

if [ -f "${CLAMAV_BIN_DIR}/freshclam" ]; then
    echo "✅ freshclam: $(file "${CLAMAV_BIN_DIR}/freshclam")"
else
    echo "❌ freshclam not found!"
fi

echo ""
echo "======================================"
echo " ✅ ClamAV shared library build complete!"
echo "======================================"
echo ""
echo "📦 Output directories:"
echo "   Libraries: ${CLAMAV_OUTPUT_DIR}/"
echo "   Binaries: ${CLAMAV_BIN_DIR}/"
echo ""
echo "   - libclamav.so (main engine)"
echo "   - libclammspack.so (optional, for archive support)"
echo "   - libclamunrar_iface.so (optional, for RAR support)"
echo "   - freshclam (for signature updates)"
