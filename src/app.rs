/// GUI 应用模块 — egui 主界面
///
/// 使用 eframe/egui 构建全中文 GUI 界面。

use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::actors;
use crate::commands::{self, Category, CommandDef, ParamType};
use crate::config::AppConfig;
use crate::network::{self, NetCommand, NetEvent};
use crate::protocol;

/// 每个命令的当前输入状态
struct CommandInputState {
    /// 浮点/整数输入框
    value_str: String,
    /// Vector3 输入框 (x, y, z)
    vec_x: String,
    vec_y: String,
    vec_z: String,
    /// Bool 当前值
    bool_val: bool,
    /// Actor 名称输入
    actor_name: String,
    /// Actor 数量
    actor_amount: String,
    /// 时间输入
    time_str: String,
}

impl Default for CommandInputState {
    fn default() -> Self {
        Self {
            value_str: String::new(),
            vec_x: String::new(),
            vec_y: String::new(),
            vec_z: String::new(),
            bool_val: true,
            actor_name: String::new(),
            actor_amount: "1".to_string(),
            time_str: "day".to_string(),
        }
    }
}

/// 主应用状态
pub struct UltraCamApp {
    /// 当前选中分类
    selected_category: Category,
    /// 所有命令定义
    commands: Vec<CommandDef>,
    /// 每个命令的输入状态 (按 name 索引)
    input_states: std::collections::HashMap<String, CommandInputState>,
    /// 发送命令到网络线程
    cmd_tx: mpsc::Sender<NetCommand>,
    /// 接收网络事件
    event_rx: Arc<StdMutex<mpsc::Receiver<NetEvent>>>,
    /// 日志消息列表
    log_messages: VecDeque<String>,
    /// 连接状态文本
    connection_status: String,
    /// 是否已连接
    is_connected: bool,
    /// 序列名称输入
    sequence_name: String,
    /// keyframes 文件路径
    keyframes_path: PathBuf,
    /// 共享的序列名 (网络线程也需要)
    shared_sequence_name: Arc<Mutex<String>>,
    /// tokio 运行时句柄 (保持运行时存活)
    _runtime: Arc<Runtime>,
    /// 配置
    config: AppConfig,
}

impl UltraCamApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        runtime: Arc<Runtime>,
        config: AppConfig,
    ) -> Self {
        // 设置字体
        setup_fonts(&cc.egui_ctx);

        // 设置深色主题
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // 创建通道
        let (cmd_tx, cmd_rx) = mpsc::channel::<NetCommand>(256);
        let (event_tx, event_rx) = mpsc::channel::<NetEvent>(256);

        let cmd_rx = Arc::new(Mutex::new(cmd_rx));
        let event_rx = Arc::new(StdMutex::new(event_rx));

        // 确定 keyframes.json 路径 (相对于 console.ini)
        let keyframes_path = PathBuf::from("../src/keyframes.json");
        let shared_sequence_name = Arc::new(Mutex::new("Default".to_string()));

        // 启动网络服务器
        let ip = config.ip.clone();
        let port = config.port;
        let kf_path = keyframes_path.clone();
        let seq_name = shared_sequence_name.clone();
        runtime.spawn(async move {
            network::run_server(ip, port, cmd_rx, event_tx, kf_path, seq_name).await;
        });

        // 初始化命令
        let all_cmds = commands::all_commands();
        let mut input_states = std::collections::HashMap::new();
        for cmd in &all_cmds {
            input_states.insert(cmd.name.to_string(), CommandInputState::default());
        }

        Self {
            selected_category: Category::Graphics,
            commands: all_cmds,
            input_states,
            cmd_tx,
            event_rx,
            log_messages: VecDeque::with_capacity(200),
            connection_status: "等待连接...".to_string(),
            is_connected: false,
            sequence_name: "Default".to_string(),
            keyframes_path,
            shared_sequence_name,
            _runtime: runtime,
            config,
        }
    }

    /// 处理收到的网络事件
    fn poll_events(&mut self) {
        // 先收集所有事件到临时 Vec，释放锁后再处理
        let events: Vec<NetEvent> = {
            if let Ok(mut rx_guard) = self.event_rx.lock() {
                let mut evts = Vec::new();
                while let Ok(event) = rx_guard.try_recv() {
                    evts.push(event);
                }
                evts
            } else {
                Vec::new()
            }
        };

        for event in events {
            match event {
                NetEvent::ServerStarted(msg) => {
                    self.connection_status = "等待连接...".to_string();
                    self.add_log(&msg);
                }
                NetEvent::ClientConnected(msg) => {
                    self.connection_status = "已连接 ✓".to_string();
                    self.is_connected = true;
                    self.add_log(&msg);
                }
                NetEvent::ClientDisconnected => {
                    self.connection_status = "已断开".to_string();
                    self.is_connected = false;
                    self.add_log("客户端已断开连接");
                }
                NetEvent::LogMessage(msg) => {
                    self.add_log(&msg);
                }
                NetEvent::Error(msg) => {
                    self.add_log(&format!("❌ {}", msg));
                }
            }
        }
    }

    fn add_log(&mut self, msg: &str) {
        let now = chrono_now();
        self.log_messages
            .push_back(format!("[{}] {}", now, msg));
        while self.log_messages.len() > 200 {
            self.log_messages.pop_front();
        }
    }

    fn send_command(&self, data: Vec<u8>) {
        let tx = self.cmd_tx.clone();
        tokio::task::block_in_place(|| {
            let _ = tx.blocking_send(NetCommand::SendBytes(data));
        });
    }

    fn send_load_sequence(&self, name: String) {
        let tx = self.cmd_tx.clone();
        tokio::task::block_in_place(|| {
            let _ = tx.blocking_send(NetCommand::LoadSequence(name));
        });
    }
}

/// 简单的时间格式化
fn chrono_now() -> String {
    let now = std::time::SystemTime::now();
    let since = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = since.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    // 加上 UTC+8
    let hours = (hours + 8) % 24;
    format!("{:02}:{:02}:{:02}", hours, mins, s)
}

/// 设置字体 — 使用 system-fonts 加载微软雅黑
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 尝试使用 system-fonts 加载
    let font_data = load_system_font();

    if let Some(data) = font_data {
        fonts.font_data.insert(
            "msyh".to_owned(),
            Arc::new(egui::FontData::from_owned(data)),
        );
        // 设为首选字体
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "msyh".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "msyh".to_owned());

        log::info!("微软雅黑字体加载成功");
    } else {
        log::warn!("无法加载微软雅黑字体，使用默认字体");
    }

    ctx.set_fonts(fonts);
}

/// 使用 system-fonts 加载中文字体
fn load_system_font() -> Option<Vec<u8>> {
    // 尝试 system-fonts crate
    use system_fonts::{find_for_locale, FontStyle, FoundFontSource};

    let (_region, found_fonts) = find_for_locale("zh-CN", FontStyle::Sans);

    for font in &found_fonts {
        match &font.source {
            FoundFontSource::Path(path) => {
                log::info!("尝试加载字体: {:?} ({})", path, font.family);
                if let Ok(data) = std::fs::read(path) {
                    return Some(data);
                }
            }
            FoundFontSource::Bytes(bytes) => {
                return Some(bytes.to_vec());
            }
        }
    }

    // 回退方案：直接读取 Windows 字体路径
    let fallback_paths = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
    ];

    for path in &fallback_paths {
        if let Ok(data) = std::fs::read(path) {
            log::info!("使用回退路径加载字体: {}", path);
            return Some(data);
        }
    }

    None
}

impl eframe::App for UltraCamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 轮询网络事件
        self.poll_events();

        // 请求持续重绘（以便实时更新日志）
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // ─── 顶部面板 ─────────────────────────────────
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⚡ UltraCam 控制台");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status_color = if self.is_connected {
                        egui::Color32::from_rgb(100, 220, 100)
                    } else {
                        egui::Color32::from_rgb(220, 150, 50)
                    };
                    ui.colored_label(status_color, &self.connection_status);
                    ui.separator();
                    ui.label(format!(
                        "{}:{}",
                        self.config.ip, self.config.port
                    ));
                });
            });
            ui.add_space(2.0);
        });

        // ─── 左侧分类导航面板 ──────────────────────────
        egui::SidePanel::left("category_panel")
            .resizable(false)
            .default_width(120.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("命令分类")
                        .strong()
                        .size(14.0),
                );
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                for cat in Category::all() {
                    let selected = self.selected_category == *cat;
                    let btn = egui::Button::new(
                        egui::RichText::new(cat.label()).size(13.0),
                    )
                    .selected(selected)
                    .min_size(egui::vec2(110.0, 32.0));

                    if ui.add(btn).clicked() {
                        self.selected_category = *cat;
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                // 序列编辑器部分
                ui.label(
                    egui::RichText::new("📷 序列编辑")
                        .strong()
                        .size(14.0),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("名称:");
                    ui.text_edit_singleline(&mut self.sequence_name);
                });

                ui.add_space(4.0);

                if ui
                    .add(egui::Button::new("保存序列").min_size(egui::vec2(110.0, 28.0)))
                    .clicked()
                {
                    // 更新共享序列名
                    let name = self.sequence_name.clone();
                    let shared = self.shared_sequence_name.clone();
                    tokio::task::block_in_place(|| {
                        let rt = tokio::runtime::Handle::current();
                        rt.block_on(async {
                            *shared.lock().await = name.clone();
                        });
                    });
                    let data = protocol::pack_response(60);
                    self.send_command(data);
                    self.add_log(&format!("保存序列: {}", self.sequence_name));
                }

                if ui
                    .add(egui::Button::new("加载序列").min_size(egui::vec2(110.0, 28.0)))
                    .clicked()
                {
                    let name = self.sequence_name.clone();
                    self.send_load_sequence(name.clone());
                    self.add_log(&format!("加载序列: {}", name));
                }

                if ui
                    .add(egui::Button::new("删除序列").min_size(egui::vec2(110.0, 28.0)))
                    .clicked()
                {
                    let data = protocol::pack_response(62);
                    self.send_command(data);
                    self.add_log("已发送删除序列命令");
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // 可用序列列表
                ui.label(egui::RichText::new("已保存序列:").size(12.0));
                let names = crate::keyframes::sequence_names(&self.keyframes_path);
                for name in names {
                    if ui.small_button(&name).clicked() {
                        self.sequence_name = name;
                    }
                }
            });

        // ─── 底部日志面板 ──────────────────────────────
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(160.0)
            .min_height(80.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("📋 日志输出")
                            .strong()
                            .size(13.0),
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.small_button("清空").clicked() {
                                self.log_messages.clear();
                            }
                        },
                    );
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for msg in &self.log_messages {
                            ui.label(
                                egui::RichText::new(msg)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(180, 200, 220)),
                            );
                        }
                    });
            });

        // ─── 中央命令面板 ──────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let cmds: Vec<CommandDef> = self
                .commands
                .iter()
                .filter(|c| c.category == self.selected_category)
                .cloned()
                .collect();

            ui.heading(
                egui::RichText::new(self.selected_category.label())
                    .size(18.0),
            );
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for cmd in &cmds {
                        self.render_command(ui, cmd);
                        ui.add_space(4.0);
                    }
                });
        });
    }
}

impl UltraCamApp {
    fn render_command(&mut self, ui: &mut egui::Ui, cmd: &CommandDef) {
        let name = cmd.name.to_string();

        egui::Frame::group(ui.style())
            .inner_margin(8)
            .corner_radius(6)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&name)
                            .strong()
                            .size(14.0)
                            .color(egui::Color32::from_rgb(130, 190, 255)),
                    );
                    ui.label(
                        egui::RichText::new(cmd.description)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(160, 160, 170)),
                    );
                });

                ui.add_space(4.0);

                // 确保有输入状态
                if !self.input_states.contains_key(&name) {
                    self.input_states
                        .insert(name.clone(), CommandInputState::default());
                }

                let state = self.input_states.get_mut(&name).unwrap();

                match cmd.param_type {
                    ParamType::Response => {
                        if ui
                            .add(
                                egui::Button::new("⚡ 发送")
                                    .min_size(egui::vec2(80.0, 28.0)),
                            )
                            .clicked()
                        {
                            let data = protocol::pack_response(cmd.packet_id as u32);
                            self.cmd_tx
                                .try_send(NetCommand::SendBytes(data))
                                .ok();
                        }
                    }
                    ParamType::Bool => {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut state.bool_val, "启用");
                            if ui
                                .add(
                                    egui::Button::new("⚡ 发送")
                                        .min_size(egui::vec2(80.0, 28.0)),
                                )
                                .clicked()
                            {
                                let val = if state.bool_val { 1u32 } else { 0u32 };
                                let data =
                                    protocol::pack_bool(cmd.packet_id as u32, val);
                                self.cmd_tx
                                    .try_send(NetCommand::SendBytes(data))
                                    .ok();
                            }
                        });
                    }
                    ParamType::Float => {
                        ui.horizontal(|ui| {
                            ui.label("值:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.value_str)
                                    .desired_width(100.0)
                                    .hint_text("输入数值"),
                            );
                            if ui
                                .add(
                                    egui::Button::new("⚡ 发送")
                                        .min_size(egui::vec2(80.0, 28.0)),
                                )
                                .clicked()
                            {
                                if let Ok(v) = state.value_str.parse::<f32>() {
                                    let data =
                                        protocol::pack_float(cmd.packet_id as u32, v);
                                    self.cmd_tx
                                        .try_send(NetCommand::SendBytes(data))
                                        .ok();
                                }
                            }
                        });
                    }
                    ParamType::Int => {
                        ui.horizontal(|ui| {
                            ui.label("值:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.value_str)
                                    .desired_width(100.0)
                                    .hint_text("输入整数"),
                            );
                            if ui
                                .add(
                                    egui::Button::new("⚡ 发送")
                                        .min_size(egui::vec2(80.0, 28.0)),
                                )
                                .clicked()
                            {
                                if let Ok(v) = state.value_str.parse::<u32>() {
                                    let data =
                                        protocol::pack_int(cmd.packet_id as u32, v);
                                    self.cmd_tx
                                        .try_send(NetCommand::SendBytes(data))
                                        .ok();
                                }
                            }
                        });
                    }
                    ParamType::Vector3F => {
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.vec_x)
                                    .desired_width(70.0),
                            );
                            ui.label("Y:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.vec_y)
                                    .desired_width(70.0),
                            );
                            ui.label("Z:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.vec_z)
                                    .desired_width(70.0),
                            );
                            if ui
                                .add(
                                    egui::Button::new("⚡ 发送")
                                        .min_size(egui::vec2(80.0, 28.0)),
                                )
                                .clicked()
                            {
                                let x = state.vec_x.parse::<f32>().unwrap_or(0.0);
                                let y = state.vec_y.parse::<f32>().unwrap_or(0.0);
                                let z = state.vec_z.parse::<f32>().unwrap_or(0.0);
                                let data = protocol::pack_vector3f(
                                    cmd.packet_id,
                                    x,
                                    y,
                                    z,
                                );
                                self.cmd_tx
                                    .try_send(NetCommand::SendBytes(data))
                                    .ok();
                            }
                        });
                    }
                    ParamType::SetTime => {
                        ui.horizontal(|ui| {
                            ui.label("时间:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.time_str)
                                    .desired_width(120.0)
                                    .hint_text("day/dawn/night/dusk 或 HH:MM"),
                            );
                            if ui
                                .add(
                                    egui::Button::new("⚡ 发送")
                                        .min_size(egui::vec2(80.0, 28.0)),
                                )
                                .clicked()
                            {
                                let (h, m) =
                                    protocol::parse_time_str(&state.time_str);
                                let data = protocol::pack_set_time(h, m);
                                self.cmd_tx
                                    .try_send(NetCommand::SendBytes(data))
                                    .ok();
                            }
                        });
                    }
                    ParamType::Actor => {
                        ui.horizontal(|ui| {
                            ui.label("Actor:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.actor_name)
                                    .desired_width(200.0)
                                    .hint_text("名称 或 random"),
                            );
                            ui.label("数量:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.actor_amount)
                                    .desired_width(40.0),
                            );
                            if ui
                                .add(
                                    egui::Button::new("⚡ 生成")
                                        .min_size(egui::vec2(80.0, 28.0)),
                                )
                                .clicked()
                            {
                                let amount =
                                    state.actor_amount.parse::<u32>().unwrap_or(1);
                                let actor = if state.actor_name.to_lowercase() == "random"
                                    || state.actor_name.to_lowercase() == "r"
                                {
                                    actors::random_actor().to_string()
                                } else {
                                    state.actor_name.clone()
                                };
                                let data = protocol::pack_actor(
                                    cmd.packet_id as u32,
                                    amount,
                                    &actor,
                                );
                                self.cmd_tx
                                    .try_send(NetCommand::SendBytes(data))
                                    .ok();
                            }
                        });
                    }
                }
            });
    }
}
