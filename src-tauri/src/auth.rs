use git_toys_engine::{runner::Auth, types::GitError};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tauri::Emitter;

#[derive(Serialize, Deserialize)]
struct Request {
    token: String,
    prompt: String,
}
#[derive(Serialize, Clone)]
pub struct Prompt {
    pub id: String,
    pub prompt: String,
}
pub type Replies = Arc<Mutex<HashMap<String, mpsc::Sender<Option<String>>>>>;
pub fn start(app: tauri::AppHandle, replies: Replies) -> Result<Auth, GitError> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let token = uuid::Uuid::new_v4().to_string();
    let auth = Auth {
        executable: std::env::current_exe()?.to_string_lossy().into(),
        address: listener.local_addr()?.to_string(),
        token: token.clone(),
        progress: None,
    };
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let token = token.clone();
            let replies = Arc::clone(&replies);
            let app = app.clone();
            std::thread::spawn(move || {
                let mut stream = stream;
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let mut raw = String::new();
                if BufReader::new(&stream).read_line(&mut raw).is_err() || raw.len() > 16_384 {
                    return;
                }
                let Ok(request) = serde_json::from_str::<Request>(&raw) else {
                    return;
                };
                if request.token != token {
                    return;
                }
                let id = uuid::Uuid::new_v4().to_string();
                let (tx, rx) = mpsc::channel();
                if let Ok(mut map) = replies.lock() {
                    map.insert(id.clone(), tx);
                } else {
                    return;
                }
                let _ = app.emit(
                    "auth-prompt",
                    Prompt {
                        id: id.clone(),
                        prompt: request.prompt,
                    },
                );
                let response = rx.recv_timeout(Duration::from_secs(120)).ok().flatten();
                if let Ok(mut map) = replies.lock() {
                    map.remove(&id);
                }
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = writeln!(stream, "{json}");
                }
            });
        }
    });
    Ok(auth)
}
pub fn askpass() {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(std::env::var("GIT_TOYS_AUTH_ADDRESS")?)?;
        stream.set_read_timeout(Some(Duration::from_secs(125)))?;
        let request = Request {
            token: std::env::var("GIT_TOYS_AUTH_TOKEN")?,
            prompt: std::env::args()
                .nth(1)
                .unwrap_or_else(|| "Git authentication".into()),
        };
        writeln!(stream, "{}", serde_json::to_string(&request)?)?;
        let mut reply = String::new();
        BufReader::new(stream).read_line(&mut reply)?;
        let secret: Option<String> = serde_json::from_str(&reply)?;
        match secret {
            Some(value) => {
                println!("{value}");
                Ok(())
            }
            None => Err("Authentication cancelled".into()),
        }
    })();
    if result.is_err() {
        std::process::exit(1)
    }
}
