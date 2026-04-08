/// TOTK UltraCam 控制台 — Rust GUI 版本
///
/// 主入口：初始化 tokio 运行时 → 启动 eframe GUI

mod actors;
mod app;
mod commands;
mod config;
mod keyframes;
mod network;
mod protocol;

use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Seconds))
        .init();

    log::info!("TOTK UltraCam 控制台启动中...");

    // 加载配置
    let config_path = PathBuf::from("../src/console.ini");
    let config = config::AppConfig::load(&config_path);
    log::info!("配置: {}:{}", config.ip, config.port);

    // 创建 tokio 运行时
    let runtime = Arc::new(
        tokio::runtime::Runtime::new().expect("无法创建 tokio 运行时"),
    );

    // eframe 选项
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("⚡ UltraCam 控制台")
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([700.0, 500.0]),
        ..Default::default()
    };

    // 启动 GUI
    let rt = runtime.clone();
    let cfg = config.clone();
    eframe::run_native(
        "UltraCam 控制台",
        options,
        Box::new(move |cc| {
            // 进入 tokio 运行时上下文
            let _guard = rt.enter();
            Ok(Box::new(app::UltraCamApp::new(cc, rt.clone(), cfg)))
        }),
    )
    .expect("eframe 启动失败");
}
