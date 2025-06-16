use chrono::Utc;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use sophia_core::command::Command;
use sophia_core::errors::Result;
use sophia_core::model::{Message, Request, User, UserInfo};
use crate::controller::Server;
use crate::service::push;
use tokio::sync::Mutex;

static GUESS_COUNTER: Lazy<Mutex<HashMap<i64, u8>>> = Lazy::new(|| Mutex::new(HashMap::new()));

async fn save_and_push_message(s: &Server, message: &Message, chat_id: i64) -> Result<()> {
    s.repo.message.save(message.clone()).await?;
    let req = Request::new(Command::NewMessage(message.clone()));
    push::push_to_chat_user(req, s, "", chat_id).await?;
    Ok(())
}

pub async fn check_answer(s: &Server, user: &UserInfo, msg: &str) -> Result<()> {
    let wordle_rules = r#"
📖 Wordle 规则（输入 @W + 你的猜测）：
1. 猜一个5字母的单词（例如：@W apple）
2. 结果标记：
    ✅ - 字母和位置都正确
    🟨 - 字母正确但位置不对
    ❌ - 字母不存在
3. 你有6次机会！
"#.to_string();

    let welcome = "欢迎玩 Wordle 小游戏，输入 @Help 获取游戏规则".to_string();

    let sys_user_info = UserInfo {
        name: "Wordle Bot🤖".to_string(),
        session_id: user.session_id.clone(),
        address: user.address.clone(),
        chat_id: user.chat_id.clone(),
        login_time: user.login_time.clone(),
    };
    let sys_user = User::from_user_info(&sys_user_info);

    if msg.starts_with("@Help") || msg.starts_with("@H") {
        return save_and_push_message(s, &Message {
            user: sys_user,
            time: Utc::now().timestamp(),
            content: wordle_rules,
            whisper: None,
        }, user.chat_id).await;
    }

    let trimmed = if msg.starts_with("@Wordle") {
        msg.trim_start_matches("@Wordle").trim_start()
    } else if msg.starts_with("@W") {
        msg.trim_start_matches("@W").trim_start()
    } else {
        return Ok(());
    };

    if trimmed.len() < 5 {
        return save_and_push_message(s, &Message {
            user: sys_user,
            time: Utc::now().timestamp(),
            content: welcome,
            whisper: None,
        }, user.chat_id).await;
    }

    let guess = trimmed[..5].to_lowercase();
    let cache = s.repo.wordle.get().await?.to_lowercase();

    let mut result = ['❌'; 5];
    let mut cache_chars: Vec<char> = cache.chars().collect();

    for (i, (g, c)) in guess.chars().zip(cache_chars.iter_mut()).enumerate() {
        if g == *c {
            result[i] = '✅';
            *c = '\0';
        }
    }

    for (i, g) in guess.chars().enumerate() {
        if result[i] == '❌' {
            if let Some(pos) = cache_chars.iter().position(|&c| c == g && c != '\0') {
                result[i] = '🟨';
                cache_chars[pos] = '\0';
            }
        }
    }

    let result_str = result.iter().collect::<String>();
    let is_correct = guess == cache;

    let (response, should_remove) = {
        let mut counter = GUESS_COUNTER.lock().await;
        let count = counter.entry(user.chat_id).or_insert(0);

        if is_correct {
            let resp = format!("{} 猜对了😎 用了 {} 次尝试", user.name, *count);
            (resp, true)
        } else if *count >= 5 {
            let resp = format!("机会用尽😈 正确答案是: {}", cache);
            (resp, true)
        } else {
            *count += 1;
            let attempts_left = 6 - *count;
            let resp = format!("{} {} (剩余机会⏱️ {})", guess, result_str, attempts_left);
            (resp, false)
        }
    };

    if should_remove {
        GUESS_COUNTER.lock().await.remove(&user.chat_id);
    }

    save_and_push_message(s, &Message {
        user: sys_user,
        time: Utc::now().timestamp(),
        content: response,
        whisper: None,
    }, user.chat_id).await
}