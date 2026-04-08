/// 命令定义 — 所有 UltraCam 支持的命令
///
/// 每个命令包含 packet ID、中文描述、分类和参数类型。

/// 命令分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Graphics,  // 图形
    Utility,   // 工具
    Cheats,    // 作弊
    Gameplay,  // 游戏性
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Graphics => "🎨 图形",
            Category::Utility => "🔧 工具",
            Category::Cheats => "⚔ 作弊",
            Category::Gameplay => "🎮 游戏性",
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::Graphics,
            Category::Utility,
            Category::Cheats,
            Category::Gameplay,
        ]
    }
}

/// 参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    Response,    // 无参数，仅发送 packet ID
    Bool,        // true/false/on/off
    Float,       // 浮点数值
    Int,         // 整数值
    Vector3F,    // 三个浮点数 (x, y, z)
    SetTime,     // 时间设置 (特殊处理)
    Actor,       // Actor 生成 (名称 + 数量)
}

/// 命令定义
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub name: &'static str,
    pub packet_id: i32,
    pub description: &'static str,
    pub category: Category,
    pub param_type: ParamType,
}

/// 获取所有命令定义
pub fn all_commands() -> Vec<CommandDef> {
    vec![
        // ─── 图形类 ─────────────────────────────
        CommandDef {
            name: "fps",
            packet_id: 40,
            description: "调整 FPS 上限",
            category: Category::Graphics,
            param_type: ParamType::Float,
        },
        CommandDef {
            name: "fov",
            packet_id: 41,
            description: "调整视野 (FOV)",
            category: Category::Graphics,
            param_type: ParamType::Float,
        },
        CommandDef {
            name: "shadow",
            packet_id: 43,
            description: "调整阴影质量",
            category: Category::Graphics,
            param_type: ParamType::Int,
        },
        CommandDef {
            name: "resolution",
            packet_id: 44,
            description: "调整分辨率",
            category: Category::Graphics,
            param_type: ParamType::Vector3F,
        },
        CommandDef {
            name: "settime",
            packet_id: -1,
            description: "设置当前时间 (day/dawn/night/dusk 或 HH:MM)",
            category: Category::Graphics,
            param_type: ParamType::SetTime,
        },
        CommandDef {
            name: "timespeed",
            packet_id: 21,
            description: "设置时间流速",
            category: Category::Graphics,
            param_type: ParamType::Float,
        },
        CommandDef {
            name: "changeweather",
            packet_id: 25,
            description: "更改天气（随机）",
            category: Category::Graphics,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "hideui",
            packet_id: 51,
            description: "切换 UI 显示/隐藏",
            category: Category::Graphics,
            param_type: ParamType::Response,
        },
        CommandDef {
            name: "firstperson",
            packet_id: 52,
            description: "切换第一人称视角",
            category: Category::Graphics,
            param_type: ParamType::Response,
        },
        CommandDef {
            name: "benchmark",
            packet_id: 63,
            description: "运行基准测试",
            category: Category::Graphics,
            param_type: ParamType::Response,
        },

        // ─── 工具类 ─────────────────────────────
        CommandDef {
            name: "spawn",
            packet_id: 1,
            description: "生成 Actor（输入 \"random\" 随机生成敌人）",
            category: Category::Utility,
            param_type: ParamType::Actor,
        },
        CommandDef {
            name: "tp",
            packet_id: 10,
            description: "传送玩家到指定坐标 (X Y Z)",
            category: Category::Utility,
            param_type: ParamType::Vector3F,
        },
        CommandDef {
            name: "cords",
            packet_id: 11,
            description: "获取玩家当前坐标",
            category: Category::Utility,
            param_type: ParamType::Response,
        },
        CommandDef {
            name: "gettime",
            packet_id: 24,
            description: "获取当前游戏时间",
            category: Category::Utility,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "pause",
            packet_id: 23,
            description: "暂停游戏（自由镜头仍可移动）",
            category: Category::Utility,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "freecam",
            packet_id: 50,
            description: "切换自由镜头",
            category: Category::Utility,
            param_type: ParamType::Response,
        },
        CommandDef {
            name: "idleanimation",
            packet_id: 53,
            description: "切换自拍相机的待机动画",
            category: Category::Utility,
            param_type: ParamType::Response,
        },
        CommandDef {
            name: "savesequence",
            packet_id: 60,
            description: "保存当前镜头序列",
            category: Category::Utility,
            param_type: ParamType::Response,
        },
        CommandDef {
            name: "loadsequence",
            packet_id: 61,
            description: "加载镜头序列",
            category: Category::Utility,
            param_type: ParamType::Response, // handled specially
        },
        CommandDef {
            name: "deletesequence",
            packet_id: 62,
            description: "删除当前游戏内的序列",
            category: Category::Utility,
            param_type: ParamType::Response,
        },

        // ─── 作弊类 ─────────────────────────────
        CommandDef {
            name: "godmode",
            packet_id: 32,
            description: "切换无敌模式",
            category: Category::Cheats,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "kill",
            packet_id: 31,
            description: "杀死玩家",
            category: Category::Cheats,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "killall",
            packet_id: 110,
            description: "杀死所有敌人",
            category: Category::Cheats,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "heal",
            packet_id: 30,
            description: "治疗玩家",
            category: Category::Cheats,
            param_type: ParamType::Int,
        },
        CommandDef {
            name: "healall",
            packet_id: 111,
            description: "治疗所有敌人",
            category: Category::Cheats,
            param_type: ParamType::Bool,
        },
        CommandDef {
            name: "gamespeed",
            packet_id: 42,
            description: "调整游戏速度",
            category: Category::Cheats,
            param_type: ParamType::Float,
        },

        // ─── 游戏性 ─────────────────────────────
        CommandDef {
            name: "healthregen",
            packet_id: 33,
            description: "脱战后每秒自动恢复生命",
            category: Category::Gameplay,
            param_type: ParamType::Response,
        },
    ]
}
