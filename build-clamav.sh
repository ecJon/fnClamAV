#!/bin/bash
set -e

# ClamAV 编译脚本
# 用法: ./build-clamav.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAMAV_VERSION="1.0.0"
CLAMAV_BUILD_DIR="${PROJECT_DIR}/clamAV/build"
CLAMAV_OUTPUT_DIR="${PROJECT_DIR}/app/bin"

echo "======================================"
echo "  Building ClamAV ${CLAMAV_VERSION}"
echo "======================================"

# 创建目录
mkdir -p "${CLAMAV_BUILD_DIR}"
mkdir -p "${CLAMAV_OUTPUT_DIR}"

# 检查是否已有编译好的二进制
if [ -f "${CLAMAV_OUTPUT_DIR}/clamscan" ] && [ -f "${CLAMAV_OUTPUT_DIR}/freshclam" ]; then
    echo "✅ ClamAV binaries already exist in ${CLAMAV_OUTPUT_DIR}"
    echo "   To rebuild, remove them first:"
    echo "   rm ${CLAMAV_OUTPUT_DIR}/clamscan ${CLAMAV_OUTPUT_DIR}/freshclam"
    exit 0
fi

# 下载 ClamAV 源码
CLAMAV tarball="${CLAMAV_BUILD_DIR}/clamav-${CLAMAV_VERSION}.tar.gz"
if [ ! -f "${CLAMAV_tarball}" ]; then
    echo "📥 Downloading ClamAV ${CLAMAV_VERSION} source..."
    wget -O "${CLAMAV_tarball}" "https://www.clamav.net/downloads/production/clamav-${CLAMAV_VERSION}.tar.gz"
fi

# 解压
CLAMAV_SRC_DIR="${CLAMAV_BUILD_DIR}/clamav-${CLAMAV_VERSION}"
if [ ! -d "${CLAMAV_SRC_DIR}" ]; then
    echo "📦 Extracting..."
    tar -xzf "${CLAMAV_tarball}" -C "${CLAMAV_BUILD_DIR}"
fi

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

# 检查 libcheck
if ! pkg-config --exists check 2>/dev/null; then
    echo "⚠️  libcheck not found. Installing..."
    sudo apt-get install -y libcheck-dev || true
fi

# 配置
echo "⚙️  Configuring ClamAV..."
if [ ! -f "Makefile" ]; then
    ./configure \
        --prefix=/usr/local \
        --disable-clamav \
        --disable-milter \
        --disable-zlib-vcheck \
        --enable-static \
        --disable-shared \
        --with-libjson=no \
        --with-libpcre2=no \
        --with-curl=no
fi

# 编译
echo "🔨 Compiling (this may take a while)..."
make -j$(nproc)

# 复制二进制文件
echo "📋 Copying binaries..."
cp "${CLAMAV_SRC_DIR}/clamscan/clamscan" "${CLAMAV_OUTPUT_DIR}/"
cp "${CLAMAV_SRC_DIR}/freshclam/freshclam" "${CLAMAV_OUTPUT_DIR}/"

# 验证
echo "🔍 Verifying binaries..."
if [ -f "${CLAMAV_OUTPUT_DIR}/clamscan" ]; then
    echo "✅ clamscan: $(file "${CLAMAV_OUTPUT_DIR}/clamscan")"
fi
if [ -f "${CLAMAV_OUTPUT_DIR}/freshclam" ]; then
    echo "✅ freshclam: $(file "${CLAMAV_OUTPUT_DIR}/freshclam}")"
fi

echo ""
echo "======================================"
echo "  ✅ ClamAV build complete!"
echo "======================================"
echo "📦 Binaries: ${CLAMAV_OUTPUT_DIR}/"
echo "   - clamscan"
echo "   - freshclam"
echo ""
