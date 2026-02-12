#!/bin/bash
# ClamAV 包装脚本 - 自动处理路径
# 用法: ./clamscan-wrapper.sh [选项] [路径...]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 获取 ClamAV 二进制目录
CLAMAV_BIN_DIR="${SCRIPT_DIR}"
CLAMAV_BIN="${CLAMAV_BIN_DIR}/clamscan"

# 获取病毒库目录
# 优先使用 TRIM_DATA_SHARE_PATHS，然后回退到本地目录
if [ -n "$TRIM_DATA_SHARE_PATHS" ]; then
    DATA_DIR="${TRIM_DATA_SHARE_PATHS%%:*}"
    DB_DIR="${DATA_DIR}/clamav"
else
    # 本地测试回退
    DB_DIR="${SCRIPT_DIR}/../share/clamav"
fi

# 确保病毒库目录存在
mkdir -p "${DB_DIR}"

# 如果病毒库为空，显示提示
if [ ! -f "${DB_DIR}"/daily.cld ] && [ ! -f "${DB_DIR}"/daily.cvd ] && \
   [ ! -f "${DB_DIR}"/main.cld ] && [ ! -f "${DB_DIR}"/main.cvd ]; then
    echo "⚠️  Warning: ClamAV virus database not found in ${DB_DIR}"
    echo ""
    echo "To update the virus database, run:"
    echo "  ${CLAMAV_BIN_DIR}/freshclam-wrapper.sh --datadir=${DB_DIR}"
    echo ""
    echo "Or copy existing database:"
    echo "  cp -r /var/lib/clamav/* ${DB_DIR}/"
    echo ""
fi

# 调用 clamscan，自动添加 --database 参数
exec "${CLAMAV_BIN}" --database="${DB_DIR}" "$@"
