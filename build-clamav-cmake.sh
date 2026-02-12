#!/bin/bash
set -e

# ClamAV CMake 编译脚本
# 用法: ./build-clamav-cmake.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAMAV_SRC_DIR="${PROJECT_DIR}/clamAV"
CLAMAV_BUILD_DIR="${CLAMAV_SRC_DIR}/build"
CLAMAV_OUTPUT_DIR="${PROJECT_DIR}/app/bin"

echo "======================================"
echo "  Building ClamAV from local source"
echo "======================================"

# 检查源码目录
if [ ! -d "${CLAMAV_SRC_DIR}" ]; then
    echo "❌ ClamAV source not found at ${CLAMAV_SRC_DIR}"
    exit 1
fi

# 检查是否已有编译好的二进制
if [ -f "${CLAMAV_OUTPUT_DIR}/clamscan" ] && [ -f "${CLAMAV_OUTPUT_DIR}/freshclam" ]; then
    echo "✅ ClamAV binaries already exist in ${CLAMAV_OUTPUT_DIR}"
    echo "   To rebuild, remove them first:"
    echo "   rm ${CLAMAV_OUTPUT_DIR}/clamscan ${CLAMAV_OUTPUT_DIR}/freshclam"
    exit 0
fi

# 创建构建目录
mkdir -p "${CLAMAV_BUILD_DIR}"
mkdir -p "${CLAMAV_OUTPUT_DIR}"

# 检查 cmake
if ! command -v cmake &> /dev/null; then
    echo "❌ cmake not found. Please install:"
    echo "   sudo apt-get install -y cmake"
    exit 1
fi

cd "${CLAMAV_BUILD_DIR}"

# 配置（如果还没配置）
if [ ! -f "Makefile" ]; then
    echo "⚙️  Configuring ClamAV with CMake..."

    # 检查依赖并尝试安装
    if ! dpkg -l | grep -q libssl-dev; then
        echo "⚠️  libssl-dev not found. Attempting to install dependencies..."
        echo "   Please run: sudo apt-get install -y cmake libssl-dev libcurl4-openssl-dev libjson-c-dev zlib1g-dev"
    fi

    cmake .. \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DENABLE_JSON=OFF \
        -DENABLE_CURSORS=OFF \
        -DENABLE_MILTER=OFF \
        -DENABLE_CLAMSUBMIT=OFF \
        -DENABLE_CLAMONACC=OFF \
        -DENABLE_TESTS=OFF \
        -DBUILD_SHARED_LIBS=OFF \
        -DENABLE_STATIC_LIB=ON \
        || {
        echo ""
        echo "❌ CMake configuration failed!"
        echo ""
        echo "Missing dependencies? Install them:"
        echo "  sudo apt-get install -y \\"
        echo "    cmake \\"
        echo "    build-essential \\"
        echo "    libssl-dev \\"
        echo "    libcurl4-openssl-dev \\"
        echo "    libjson-c-dev \\"
        echo "    zlib1g-dev \\"
        echo "    libpcre2-dev \\"
        echo "    libcheck-dev"
        exit 1
    }
fi

# 编译
echo "🔨 Compiling ClamAV (this may take 10-30 minutes)..."
make -j$(nproc)

# 复制二进制文件
echo "📋 Copying binaries..."
if [ -f "${CLAMAV_BUILD_DIR}/clamscan/clamscan" ]; then
    cp "${CLAMAV_BUILD_DIR}/clamscan/clamscan" "${CLAMAV_OUTPUT_DIR}/"
elif [ -f "${CLAMAV_BUILD_DIR}/bin/clamscan" ]; then
    cp "${CLAMAV_BUILD_DIR}/bin/clamscan" "${CLAMAV_OUTPUT_DIR}/"
fi

if [ -f "${CLAMAV_BUILD_DIR}/freshclam/freshclam" ]; then
    cp "${CLAMAV_BUILD_DIR}/freshclam/freshclam" "${CLAMAV_OUTPUT_DIR}/"
elif [ -f "${CLAMAV_BUILD_DIR}/bin/freshclam" ]; then
    cp "${CLAMAV_BUILD_DIR}/bin/freshclam" "${CLAMAV_OUTPUT_DIR}/"
fi

chmod +x "${CLAMAV_OUTPUT_DIR}/clamscan"
chmod +x "${CLAMAV_OUTPUT_DIR}/freshclam"

# 验证
echo ""
echo "🔍 Verifying binaries..."
if [ -f "${CLAMAV_OUTPUT_DIR}/clamscan" ]; then
    echo "✅ clamscan: $(file "${CLAMAV_OUTPUT_DIR}/clamscan")"
    "${CLAMAV_OUTPUT_DIR}/clamscan" --version | head -1
else
    echo "❌ clamscan not found!"
fi

if [ -f "${CLAMAV_OUTPUT_DIR}/freshclam" ]; then
    echo "✅ freshclam: $(file "${CLAMAV_OUTPUT_DIR}/freshclam")"
    "${CLAMAV_OUTPUT_DIR}/freshclam" --version | head -1
else
    echo "❌ freshclam not found!"
fi

echo ""
echo "======================================"
echo "  ✅ ClamAV build complete!"
echo "======================================"
echo "📦 Binaries: ${CLAMAV_OUTPUT_DIR}/"
echo "   - clamscan"
echo "   - freshclam"
echo ""
