//! 可配置的台词：从 `~/.nixie/quotes.json` 读取，按 mood 随机展示。
//! 文件需为 UTF-8 编码；key 与 mood 的 CSS class 一致（idle / thinking / writing / …）。
//!
//! Phase 3：当 `subagent_depth > 0` 时优先使用 **`{mood}_subagent`** 键（如 `thinking_subagent`），
//! 缺失则回退到普通 mood 键，与副标题「子任务进行中」同屏配合。

use std::collections::HashMap;
use std::path::PathBuf;

fn quotes_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".nixie").join("quotes.json"))
}

fn random_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mixed = n ^ (n >> 40) ^ (n >> 80);
    (mixed as usize) % len
}

/// 选台词时的 Hook 侧上下文（Phase 3）。
#[derive(Clone, Copy, Default)]
pub struct QuoteContext {
    /// `HookState.subagent_depth`；>0 时尝试 `{mood}_subagent` 列表。
    pub subagent_depth: u32,
}

/// 从 `~/.nixie/quotes.json` 读取配置（UTF-8），缺失的 key 用默认台词补全。
pub fn load_quotes() -> HashMap<String, Vec<String>> {
    let mut fallback = default_quotes();
    let path = match quotes_path() {
        Some(p) => p,
        None => return fallback,
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return fallback,
    };
    let user: HashMap<String, Vec<String>> = match serde_json::from_str(&contents) {
        Ok(u) => u,
        Err(_) => return fallback,
    };
    for (k, v) in user {
        if !v.is_empty() {
            fallback.insert(k, v);
        }
    }
    fallback
}

/// Phase 3：按 mood + 子 Agent 上下文选一句；无 `_subagent` 配置时与普通 mood 相同。
pub fn pick_quote(
    quotes: &HashMap<String, Vec<String>>,
    mood_class: &str,
    label: &str,
    ctx: &QuoteContext,
) -> String {
    if ctx.subagent_depth > 0 {
        let key = format!("{mood_class}_subagent");
        if let Some(list) = quotes.get(&key).filter(|v| !v.is_empty()) {
            let idx = random_index(list.len());
            return list[idx].clone();
        }
    }
    get_random_quote(quotes, mood_class, label)
}

/// 从配置中按 mood 随机取一句；无配置或空列表时返回 label 作为回退。
pub fn get_random_quote(quotes: &HashMap<String, Vec<String>>, mood_class: &str, label: &str) -> String {
    let list = match quotes.get(mood_class) {
        Some(v) if !v.is_empty() => v,
        _ => return label.to_string(),
    };
    let idx = random_index(list.len());
    list[idx].clone()
}

fn subagent_lines() -> Vec<String> {
    vec![
        "分身干活中".to_string(),
        "子任务跑着呢".to_string(),
        "那边也在忙".to_string(),
        "小号加班中".to_string(),
        "分身别摸鱼".to_string(),
        "并行冲冲冲".to_string(),
        "子猪努力中".to_string(),
        "后台在跑".to_string(),
    ]
}

/// 内置默认台词（呆萌风格），作为文件缺失或解析失败时的回退。
fn default_quotes() -> HashMap<String, Vec<String>> {
    let mut m = HashMap::new();
    let sa = subagent_lines();
    m.insert(
        "idle".to_string(),
        vec![
            "在呢在呢".to_string(),
            "等指令中".to_string(),
            "休息一下".to_string(),
            "发会儿呆~".to_string(),
            "放空一下".to_string(),
            "歇歇".to_string(),
            "待机中".to_string(),
            "啥也不干".to_string(),
            "摸鱼预备".to_string(),
            "安静如猪".to_string(),
        ],
    );
    m.insert(
        "coding".to_string(),
        vec![
            "噼里啪啦".to_string(),
            "写写写".to_string(),
            "敲键盘中".to_string(),
            "码码码".to_string(),
            "键盘冒火星".to_string(),
            "本猪在写".to_string(),
            "打字中".to_string(),
            "敲敲敲".to_string(),
            "代码飞起来".to_string(),
            "手速拉满".to_string(),
        ],
    );
    m.insert(
        "thinking".to_string(),
        vec![
            "让本猪想想".to_string(),
            "思考中...".to_string(),
            "脑袋转呀转".to_string(),
            "收到！".to_string(),
            "好的好的".to_string(),
            "本猪来了".to_string(),
            "嗯...".to_string(),
            "在想了在想了".to_string(),
            "脑子转圈圈".to_string(),
            "稍等本猪".to_string(),
            "推理中".to_string(),
            "动脑筋".to_string(),
            "想破小脑瓜".to_string(),
        ],
    );
    m.insert(
        "writing".to_string(),
        vec![
            "噼里啪啦".to_string(),
            "在写呢".to_string(),
            "码字中".to_string(),
            "改文件啦".to_string(),
            "写写写".to_string(),
            "本猪在改".to_string(),
            "刷刷刷".to_string(),
            "编辑中".to_string(),
            "键盘噼里啪啦".to_string(),
            "写好了再叫你".to_string(),
        ],
    );
    m.insert(
        "running".to_string(),
        vec![
            "跑起来~".to_string(),
            "执行中".to_string(),
            "冲冲冲".to_string(),
            "跑跑跑".to_string(),
            "命令跑起来".to_string(),
            "等结果中".to_string(),
            "跑着呢".to_string(),
            "执行跑起来".to_string(),
            "跑完叫你".to_string(),
            "马上好".to_string(),
        ],
    );
    m.insert(
        "searching".to_string(),
        vec![
            "让本猪看看代码".to_string(),
            "翻翻文件".to_string(),
            "搜一搜".to_string(),
            "查查去".to_string(),
            "找找看".to_string(),
            "读读读".to_string(),
            "看看有啥".to_string(),
            "本猪去查".to_string(),
            "找找".to_string(),
            "搜搜".to_string(),
        ],
    );
    m.insert(
        "web-search".to_string(),
        vec![
            "开始网上冲浪".to_string(),
            "冲浪中".to_string(),
            "上网查查".to_string(),
            "让本猪看看".to_string(),
            "网上搜搜".to_string(),
            "冲浪~".to_string(),
            "上网去".to_string(),
            "查查网上".to_string(),
            "冲浪去".to_string(),
            "上网冲浪".to_string(),
        ],
    );
    m.insert(
        "error".to_string(),
        vec![
            "哎呀".to_string(),
            "翻车了".to_string(),
            "没事没事".to_string(),
            "嗐".to_string(),
            "翻车现场".to_string(),
            "不慌不慌".to_string(),
            "再来一次".to_string(),
            "出错了诶".to_string(),
            "本猪也懵了".to_string(),
            "摸摸头".to_string(),
        ],
    );
    m.insert(
        "success".to_string(),
        vec![
            "搞定~".to_string(),
            "棒棒的".to_string(),
            "嘿嘿".to_string(),
            "完成~".to_string(),
            "好耶".to_string(),
            "稳了".to_string(),
            "本猪厉害吧".to_string(),
            "搞定搞定".to_string(),
            "收工".to_string(),
            "完美".to_string(),
        ],
    );
    m.insert(
        "sleeping".to_string(),
        vec![
            "zzZ".to_string(),
            "呼...".to_string(),
            "睡一会儿".to_string(),
            "困了".to_string(),
            "打盹中".to_string(),
            "zzz".to_string(),
            "歇会儿".to_string(),
            "眯一下".to_string(),
            "呼噜呼噜".to_string(),
            "睡香香".to_string(),
        ],
    );

    // ── Phase 3：子 Agent 专用（键名 = mood_class + "_subagent"）────────────────
    m.insert("thinking_subagent".to_string(), sa.clone());
    m.insert("writing_subagent".to_string(), sa.clone());
    m.insert("running_subagent".to_string(), sa.clone());
    m.insert("searching_subagent".to_string(), sa.clone());
    m.insert("web-search_subagent".to_string(), sa);
    m.insert(
        "error_subagent".to_string(),
        vec![
            "分身也绊了".to_string(),
            "那边翻车了".to_string(),
            "子任务报错".to_string(),
            "小号翻车".to_string(),
            "并行里有个坑".to_string(),
        ],
    );
    m.insert(
        "success_subagent".to_string(),
        vec![
            "分身收工".to_string(),
            "那边搞定了".to_string(),
            "子任务过线".to_string(),
            "小号立功".to_string(),
        ],
    );

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_quote_prefers_subagent_key() {
        let mut m = HashMap::new();
        m.insert("thinking".to_string(), vec!["plain".to_string()]);
        m.insert(
            "thinking_subagent".to_string(),
            vec!["from_sub".to_string()],
        );
        let ctx = QuoteContext { subagent_depth: 1 };
        assert_eq!(pick_quote(&m, "thinking", "fb", &ctx), "from_sub");
        let ctx0 = QuoteContext { subagent_depth: 0 };
        assert_eq!(pick_quote(&m, "thinking", "fb", &ctx0), "plain");
    }
}
