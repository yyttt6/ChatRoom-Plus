use chrono::Utc;

use sophia_core::command::Command;
use sophia_core::errors::Result;
use sophia_core::model::{Message, Request, User, UserInfo};

use crate::controller::Server;
use crate::service::push;

use super::user;

async fn get_whisper(s: &Server, msg: &str, chat_id: i64) -> Option<UserInfo> {
    if msg.starts_with("@") {
        let start = msg.find('@')? + 1;
        let remaining = &msg[start..];
        let end = remaining.find(' ').unwrap_or(remaining.len());
        let name = &remaining[..end];
        let result = user::find_user_by_name(s, name, chat_id).await;
        return result.ok()?
    }
    None
}

pub async fn send(s: &Server, user: &UserInfo, msg: &str) -> Result<()> {
    let u = User::from_user_info(&user);
    let now = Utc::now().timestamp();
    let whisper: Option<UserInfo> = get_whisper(s, msg, u.chat_id).await;
    match whisper {
        Some(whisper_to) => {
            let signal = "  (wispering)";
            let message = Message {user: u, time: now, content: msg.to_string()+signal, whisper: Some(whisper_to.clone())};
            let _ = s.repo.message.save(message.clone()).await?;
            let req = Request::new(Command::NewMessage(message));
            let temp_vec = vec![whisper_to, user.clone()];
            push::push_to_user(req, s, "", &temp_vec).await;
        },
        None => {
            let message = Message {user: u, time: now, content: msg.to_string(), whisper: None};
            let _ = s.repo.message.save(message.clone()).await?;
            let req = Request::new(Command::NewMessage(message));
            push::push_to_chat_user(req, s, "", user.chat_id).await?;
        }
    };
    
    Ok(())
}