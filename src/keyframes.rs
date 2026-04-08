/// 关键帧序列模块 — 保存和加载镜头序列

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 单个关键帧
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    #[serde(rename = "Pos:")]
    pub pos: [String; 3],
    #[serde(rename = "Forward:")]
    pub forward: [String; 3],
    #[serde(rename = "Up:")]
    pub up: [String; 3],
    #[serde(rename = "Fov:")]
    pub fov: String,
    #[serde(rename = "Duration")]
    pub duration: String,
    #[serde(rename = "Lerp")]
    pub lerp: String,
}

/// 序列集合：name → (index → keyframe)
pub type SequenceMap = BTreeMap<String, BTreeMap<String, Keyframe>>;

/// 从 JSON 文件加载所有序列
pub fn load_all(path: &Path) -> SequenceMap {
    if let Ok(content) = std::fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        log::warn!("无法读取 keyframes 文件: {:?}", path);
        BTreeMap::new()
    }
}

/// 保存所有序列到 JSON 文件
pub fn save_all(path: &Path, data: &SequenceMap) {
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                log::error!("保存 keyframes 失败: {}", e);
            }
        }
        Err(e) => log::error!("序列化 keyframes 失败: {}", e),
    }
}

/// 将收到的 KEYFRAME 文本解析并保存
pub fn parse_and_save_keyframe(path: &Path, sequence_name: &str, text: &str) {
    let mut data = load_all(path);

    let array: Vec<&str> = text.split(' ').collect();
    if array.len() < 20 {
        log::error!("KEYFRAME 数据不完整: {}", text);
        return;
    }

    let keyframe = Keyframe {
        pos: [
            array[3].to_string(),
            array[4].to_string(),
            array[5].to_string(),
        ],
        forward: [
            array[7].to_string(),
            array[8].to_string(),
            array[9].to_string(),
        ],
        up: [
            array[11].to_string(),
            array[12].to_string(),
            array[13].to_string(),
        ],
        fov: array[15].to_string(),
        duration: array[17].to_string(),
        lerp: array[19].to_string(),
    };

    let entry = data.entry(sequence_name.to_string()).or_default();
    entry.insert(array[1].to_string(), keyframe);

    save_all(path, &data);
}

/// 获取序列名称列表
pub fn sequence_names(path: &Path) -> Vec<String> {
    load_all(path).keys().cloned().collect()
}
