#!/bin/bash
# ClamAV freshclam 包装脚本 - 自动处理路径
# 用法: ./freshclam-wrapper.sh [选项]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 获取 ClamAV 二进制目录
CLAMAV_BIN_DIR="${SCRIPT_DIR}"
CLAMAV_BIN="${CLAMAV_BIN_DIR}/freshclam"

# 获取病毒库目录
if [ -n "$TRIM_DATA_SHARE_PATHS" ]; then
    DATA_DIR="${TRIM_DATA_SHARE_PATHS%%:*}"
    DB_DIR="${DATA_DIR}/clamav"
else
    # 本地测试回退
    DB_DIR="${SCRIPT_DIR}/../share/clamav"
fi

# 确保病毒库目录存在
mkdir -p "${DB_DIR}"

# 调用 freshclam，自动添加 --datadir 参数
exec "${CLAMAV_BIN}" --datadir="${DB_DIR}" --stdout "$@"
