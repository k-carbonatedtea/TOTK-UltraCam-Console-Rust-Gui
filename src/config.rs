/// 配置模块 — 读取 console.ini

use std::path::Path;

/// 应用配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub ip: String,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".to_string(),
            port: 5555,
        }
    }
}

impl AppConfig {
    /// 从 INI 文件加载配置，失败时使用默认值
    pub fn load(path: &Path) -> Self {
        let mut config = configparser::ini::Ini::new();
        if config.load(path.to_string_lossy().as_ref()).is_ok() {
            let ip = config
                .get("Console", "IP")
                .unwrap_or_else(|| "127.0.0.1".to_string())
                .trim()
                .to_string();
            let port = config
                .get("Console", "port")
                .and_then(|p| p.trim().parse::<u16>().ok())
                .unwrap_or(5555);
            Self { ip, port }
        } else {
            log::warn!("无法读取配置文件 {:?}，使用默认配置", path);
            Self::default()
        }
    }
}
