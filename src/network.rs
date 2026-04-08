/// 网络模块 — tokio 异步 TCP 服务器
///
/// 使用 mpsc 通道在 GUI 线程和网络线程之间通信。

use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::keyframes;
use crate::protocol;

/// GUI → 网络线程的消息
#[derive(Debug)]
pub enum NetCommand {
    /// 发送原始字节到客户端
    SendBytes(Vec<u8>),
    /// 加载序列（特殊处理，需要按帧发送）
    LoadSequence(String),
}

/// 网络线程 → GUI 的消息
#[derive(Debug, Clone)]
pub enum NetEvent {
    /// 服务器启动
    ServerStarted(String),
    /// 客户端已连接
    ClientConnected(String),
    /// 客户端断开
    ClientDisconnected,
    /// 收到日志消息
    LogMessage(String),
    /// 错误
    Error(String),
}

/// 启动 TCP 服务器（在 tokio 运行时中调用）
pub async fn run_server(
    ip: String,
    port: u16,
    cmd_rx: Arc<Mutex<mpsc::Receiver<NetCommand>>>,
    event_tx: mpsc::Sender<NetEvent>,
    keyframes_path: PathBuf,
    sequence_name: Arc<Mutex<String>>,
) {
    let addr = format!("{}:{}", ip, port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            let _ = event_tx
                .send(NetEvent::ServerStarted(format!("服务器启动: {}", addr)))
                .await;
            l
        }
        Err(e) => {
            let _ = event_tx
                .send(NetEvent::Error(format!("绑定失败 {}: {}", addr, e)))
                .await;
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let _ = event_tx
                    .send(NetEvent::ClientConnected(format!(
                        "客户端已连接: {}",
                        peer
                    )))
                    .await;

                let (reader, writer) = stream.into_split();
                let writer = Arc::new(Mutex::new(writer));
                let buf_reader = BufReader::new(reader);

                let event_tx_recv = event_tx.clone();
                let kf_path = keyframes_path.clone();
                let seq_name = sequence_name.clone();

                // 接收数据任务
                let recv_handle = {
                    let event_tx = event_tx_recv.clone();
                    tokio::spawn(async move {
                        let mut lines = buf_reader.lines();
                        loop {
                            match lines.next_line().await {
                                Ok(Some(line)) => {
                                    if line.starts_with("KEYFRAME") {
                                        let name = seq_name.lock().await.clone();
                                        keyframes::parse_and_save_keyframe(
                                            &kf_path, &name, &line,
                                        );
                                        let _ = event_tx
                                            .send(NetEvent::LogMessage(format!(
                                                "收到关键帧: {}",
                                                name
                                            )))
                                            .await;
                                    } else {
                                        let _ = event_tx
                                            .send(NetEvent::LogMessage(line))
                                            .await;
                                    }
                                }
                                Ok(None) => {
                                    let _ = event_tx
                                        .send(NetEvent::ClientDisconnected)
                                        .await;
                                    break;
                                }
                                Err(e) => {
                                    let _ = event_tx
                                        .send(NetEvent::Error(format!("读取错误: {}", e)))
                                        .await;
                                    break;
                                }
                            }
                        }
                    })
                };

                // 发送命令任务
                let send_handle = {
                    let writer = writer.clone();
                    let cmd_rx = cmd_rx.clone();
                    let event_tx = event_tx.clone();
                    let kf_path = keyframes_path.clone();
                    tokio::spawn(async move {
                        let mut rx = cmd_rx.lock().await;
                        while let Some(cmd) = rx.recv().await {
                            match cmd {
                                NetCommand::SendBytes(data) => {
                                    let mut w = writer.lock().await;
                                    if let Err(e) = w.write_all(&data).await {
                                        let _ = event_tx
                                            .send(NetEvent::Error(format!(
                                                "发送失败: {}",
                                                e
                                            )))
                                            .await;
                                        break;
                                    }
                                    if let Err(e) = w.flush().await {
                                        let _ = event_tx
                                            .send(NetEvent::Error(format!(
                                                "刷新失败: {}",
                                                e
                                            )))
                                            .await;
                                        break;
                                    }
                                }
                                NetCommand::LoadSequence(name) => {
                                    let sequences = keyframes::load_all(&kf_path);
                                    if let Some(seq) = sequences.get(&name) {
                                        let mut idx = 0u32;
                                        for (_key, kf) in seq.iter() {
                                            let pos = [
                                                kf.pos[0].parse::<f32>().unwrap_or(0.0),
                                                kf.pos[1].parse::<f32>().unwrap_or(0.0),
                                                kf.pos[2].parse::<f32>().unwrap_or(0.0),
                                            ];
                                            let fwd = [
                                                kf.forward[0].parse::<f32>().unwrap_or(0.0),
                                                kf.forward[1].parse::<f32>().unwrap_or(0.0),
                                                kf.forward[2].parse::<f32>().unwrap_or(0.0),
                                            ];
                                            let up = [
                                                kf.up[0].parse::<f32>().unwrap_or(0.0),
                                                kf.up[1].parse::<f32>().unwrap_or(0.0),
                                                kf.up[2].parse::<f32>().unwrap_or(0.0),
                                            ];
                                            let fov =
                                                kf.fov.parse::<f32>().unwrap_or(50.0);
                                            let dur = kf
                                                .duration
                                                .parse::<f32>()
                                                .unwrap_or(0.0);

                                            let data = protocol::pack_keyframe(
                                                idx, pos, fwd, up, fov, dur,
                                            );
                                            let mut w = writer.lock().await;
                                            if let Err(e) = w.write_all(&data).await {
                                                let _ = event_tx
                                                    .send(NetEvent::Error(format!(
                                                        "发送帧失败: {}",
                                                        e
                                                    )))
                                                    .await;
                                                return;
                                            }
                                            let _ = w.flush().await;
                                            idx += 1;
                                            tokio::time::sleep(
                                                std::time::Duration::from_millis(100),
                                            )
                                            .await;
                                        }
                                        let _ = event_tx
                                            .send(NetEvent::LogMessage(format!(
                                                "序列 \"{}\" 加载完成 ({} 帧)",
                                                name, idx
                                            )))
                                            .await;
                                    } else {
                                        let _ = event_tx
                                            .send(NetEvent::Error(format!(
                                                "序列 \"{}\" 不存在",
                                                name
                                            )))
                                            .await;
                                    }
                                }
                            }
                        }
                    })
                };

                // 任一任务结束则另一个也取消
                tokio::select! {
                    _ = recv_handle => {},
                    _ = send_handle => {},
                }

                let _ = event_tx.send(NetEvent::ClientDisconnected).await;
            }
            Err(e) => {
                let _ = event_tx
                    .send(NetEvent::Error(format!("接受连接失败: {}", e)))
                    .await;
            }
        }
    }
}
