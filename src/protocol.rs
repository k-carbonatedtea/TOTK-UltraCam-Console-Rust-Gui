/// 协议层 — 7 种数据包类型的序列化
///
/// 所有数据包使用小端序，与 Python 版本完全兼容。

/// 将 u32 写入 Vec<u8>（小端序）
fn push_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// 将 i32 写入 Vec<u8>（小端序）
fn push_i32_le(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// 将 f32 写入 Vec<u8>（小端序）
fn push_f32_le(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

// ─── RequestResponse ────────────────────────────────────────
/// 仅发送 packet ID，无额外数据
pub fn pack_response(packet: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4);
    push_u32_le(&mut buf, packet);
    buf
}

// ─── RequestBool ────────────────────────────────────────────
/// packet ID + u32 值 (0/1/2)
pub fn pack_bool(packet: u32, value: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    push_u32_le(&mut buf, packet);
    push_u32_le(&mut buf, value);
    buf
}

/// 解析布尔字符串 ("true"/"on" → 1, "false"/"off" → 0, 空 → 2)
pub fn parse_bool_str(s: &str) -> u32 {
    match s.to_lowercase().as_str() {
        "true" | "on" => 1,
        "false" | "off" => 0,
        _ => 2,
    }
}

// ─── RequestFloat ───────────────────────────────────────────
/// packet ID + f32 值
pub fn pack_float(packet: u32, value: f32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    push_u32_le(&mut buf, packet);
    push_f32_le(&mut buf, value);
    buf
}

// ─── RequestInt ─────────────────────────────────────────────
/// packet ID + u32 值
pub fn pack_int(packet: u32, value: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    push_u32_le(&mut buf, packet);
    push_u32_le(&mut buf, value);
    buf
}

// ─── RequestVector3F ────────────────────────────────────────
/// packet ID (i32) + 0 (i32) + 3 × f32
pub fn pack_vector3f(packet: i32, x: f32, y: f32, z: f32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(20);
    push_i32_le(&mut buf, packet);
    push_i32_le(&mut buf, 0);
    push_f32_le(&mut buf, x);
    push_f32_le(&mut buf, y);
    push_f32_le(&mut buf, z);
    buf
}

// ─── RequestSetTime ─────────────────────────────────────────
/// packet=20 + 0 + hour + minutes （全部 u32 小端序）
pub fn pack_set_time(hour: u32, minutes: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16);
    push_u32_le(&mut buf, 20); // fixed packet id
    push_u32_le(&mut buf, 0);
    push_u32_le(&mut buf, hour);
    push_u32_le(&mut buf, minutes);
    buf
}

/// 解析时间字符串: "day"/"dawn"/"night"/"dusk" 或 "HH:MM"
pub fn parse_time_str(s: &str) -> (u32, u32) {
    match s.to_lowercase().as_str() {
        "day" => (12, 0),
        "dawn" => (8, 0),
        "night" => (24, 0),
        "dusk" => (20, 0),
        _ => {
            let parts: Vec<&str> = s.split(':').collect();
            let hour = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
            let min = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            (hour, min)
        }
    }
}

// ─── RequestActor ───────────────────────────────────────────
/// packet ID + amount(u32) + actor name (big-endian bytes) + 0(u32)
pub fn pack_actor(packet: u32, amount: u32, actor: &str) -> Vec<u8> {
    let name_bytes = actor.as_bytes();
    let mut buf = Vec::with_capacity(12 + name_bytes.len());
    push_u32_le(&mut buf, packet);
    push_u32_le(&mut buf, amount);
    buf.extend_from_slice(name_bytes); // big-endian string (raw bytes)
    push_u32_le(&mut buf, 0);
    buf
}

// ─── Keyframe ───────────────────────────────────────────────
/// 序列帧数据包 (packet=61)
pub fn pack_keyframe(
    index: u32,
    pos: [f32; 3],
    forward: [f32; 3],
    up: [f32; 3],
    fov: f32,
    duration: f32,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(48);
    push_u32_le(&mut buf, 61);
    push_u32_le(&mut buf, index);
    for v in pos {
        push_f32_le(&mut buf, v);
    }
    for v in forward {
        push_f32_le(&mut buf, v);
    }
    for v in up {
        push_f32_le(&mut buf, v);
    }
    push_f32_le(&mut buf, fov);
    push_f32_le(&mut buf, duration);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_response() {
        let data = pack_response(50);
        assert_eq!(data, 50u32.to_le_bytes().to_vec());
    }

    #[test]
    fn test_pack_bool() {
        let data = pack_bool(32, 1);
        assert_eq!(data.len(), 8);
        assert_eq!(u32::from_le_bytes(data[0..4].try_into().unwrap()), 32);
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), 1);
    }

    #[test]
    fn test_pack_float() {
        let data = pack_float(40, 60.0);
        assert_eq!(data.len(), 8);
        assert_eq!(u32::from_le_bytes(data[0..4].try_into().unwrap()), 40);
        assert_eq!(f32::from_le_bytes(data[4..8].try_into().unwrap()), 60.0);
    }

    #[test]
    fn test_pack_set_time() {
        let data = pack_set_time(12, 30);
        assert_eq!(data.len(), 16);
        assert_eq!(u32::from_le_bytes(data[0..4].try_into().unwrap()), 20);
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(data[8..12].try_into().unwrap()), 12);
        assert_eq!(u32::from_le_bytes(data[12..16].try_into().unwrap()), 30);
    }

    #[test]
    fn test_parse_time_str() {
        assert_eq!(parse_time_str("day"), (12, 0));
        assert_eq!(parse_time_str("night"), (24, 0));
        assert_eq!(parse_time_str("14:30"), (14, 30));
        assert_eq!(parse_time_str("8"), (8, 0));
    }

    #[test]
    fn test_parse_bool_str() {
        assert_eq!(parse_bool_str("true"), 1);
        assert_eq!(parse_bool_str("on"), 1);
        assert_eq!(parse_bool_str("false"), 0);
        assert_eq!(parse_bool_str("off"), 0);
        assert_eq!(parse_bool_str(""), 2);
    }

    #[test]
    fn test_pack_actor() {
        let data = pack_actor(1, 5, "Enemy_Bokoblin_Junior");
        // packet(4) + amount(4) + name(21) + null(4) = 33
        assert_eq!(data.len(), 33);
        assert_eq!(u32::from_le_bytes(data[0..4].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), 5);
        assert_eq!(&data[8..29], b"Enemy_Bokoblin_Junior");
    }
}
