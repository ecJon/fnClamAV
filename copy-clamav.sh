#!/bin/bash
set -e

# ClamAV 快速复制脚本 (从系统安装的 ClamAV 复制二进制)
# 用法: ./copy-clamav.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${PROJECT_DIR}/app/bin"

echo "======================================"
echo "  Copying ClamAV binaries from system"
echo "======================================"

# 创建目录
mkdir -p "${OUTPUT_DIR}"

# 检查系统 ClamAV
if ! command -v clamscan &> /dev/null; then
    echo "❌ ClamAV not found on system"
    echo ""
    echo "To install ClamAV on Debian/Ubuntu:"
    echo "  sudo apt-get update"
    echo "  sudo apt-get install -y clamav"
    echo ""
    echo "Or build from source:"
    echo "  ./build-clamav.sh"
    exit 1
fi

# 复制二进制
echo "📋 Copying binaries..."
CLAMSCAN_PATH=$(command -v clamscan)
FRESHCLAM_PATH=$(command -v freshclam)

cp "$CLAMSCAN_PATH" "${OUTPUT_DIR}/clamscan"
cp "$FRESHCLAM_PATH" "${OUTPUT_DIR}/freshclam"

chmod +x "${OUTPUT_DIR}/clamscan"
chmod +x "${OUTPUT_DIR}/freshclam"

echo "✅ Copied from:"
echo "   $CLAMSCAN_PATH → ${OUTPUT_DIR}/clamscan"
echo "   $FRESHCLAM_PATH → ${OUTPUT_DIR}/freshclam"
echo ""
echo "📦 Output: ${OUTPUT_DIR}/"
echo "   - clamscan"
echo "   - freshclam"
echo ""
echo "======================================"
echo "  ✅ Done!"
echo "======================================"
