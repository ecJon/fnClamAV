// 环境变量配置
use std::env;

/// 飞牛环境变量
#[derive(Debug, Clone)]
pub struct FnosEnv {
    /// 应用可执行文件目录 (TRIM_APPDEST)
    pub app_dest: String,
    /// 数据共享路径列表 (TRIM_DATA_SHARE_PATHS)
    pub data_share_paths: String,
    /// 配置文件目录 (TRIM_PKGETC)
    pub pkg_etc: String,
    /// 动态数据目录 (TRIM_PKGVAR)
    pub pkg_var: String,
    /// 临时文件目录 (TRIM_PKGTMP)
    pub pkg_tmp: String,
}

impl FnosEnv {
    pub fn from_env() -> Result<Self, String> {
        let app_dest = env::var("TRIM_APPDEST")
            .map_err(|_| "TRIM_APPDEST not set".to_string())?;

        // 获取数据共享路径，如果未设置则使用默认值
        // 注意：飞牛系统会根据manifest中的配置设置这个变量
        let data_share_paths = env::var("TRIM_DATA_SHARE_PATHS")
            .unwrap_or_else(|_| "/tmp/clamav_data".to_string());

        let pkg_etc = env::var("TRIM_PKGETC")
            .unwrap_or_else(|_| format!("{}/config", app_dest));

        let pkg_var = env::var("TRIM_PKGVAR")
            .unwrap_or_else(|_| format!("{}/var", app_dest));

        let pkg_tmp = env::var("TRIM_PKGTMP")
            .unwrap_or_else(|_| "/tmp".to_string());

        Ok(Self {
            app_dest,
            data_share_paths,
            pkg_etc,
            pkg_var,
            pkg_tmp,
        })
    }

    /// 获取第一个数据共享目录路径
    pub fn data_dir(&self) -> String {
        self.data_share_paths
            .split(':')
            .next()
            .unwrap_or("/tmp/clamav_data")
            .to_string()
    }

    /// ClamAV 包装脚本路径
    pub fn clamscan_bin(&self) -> String {
        format!("{}/bin/clamscan", self.app_dest)
    }

    pub fn freshclam_bin(&self) -> String {
        format!("{}/bin/freshclam", self.app_dest)
    }

    /// 病毒库目录
    pub fn clamav_db_dir(&self) -> String {
        format!("{}/clamav", self.data_dir())
    }

    /// 隔离区目录
    pub fn quarantine_dir(&self) -> String {
        format!("{}/quarantine", self.data_dir())
    }

    /// 历史记录数据库
    pub fn history_db(&self) -> String {
        format!("{}/history.db", self.data_dir())
    }

    /// 配置文件路径
    pub fn settings_file(&self) -> String {
        format!("{}/settings.json", self.pkg_etc)
    }

    /// 扫描状态文件
    pub fn scan_state_file(&self) -> String {
        format!("{}/scan_state.json", self.pkg_var)
    }

    /// 日志文件
    pub fn log_file(&self) -> String {
        format!("{}/daemon.log", self.pkg_var)
    }

    /// PID 文件
    pub fn pid_file(&self) -> String {
        format!("{}/daemon.pid", self.pkg_var)
    }
}
