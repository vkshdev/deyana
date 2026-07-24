use crate::credential::BrowserCredential;
use crate::framing::{read_message, write_message};
use serde_json::Value;
use std::io;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Error as WebSocketError, Message};

enum NativeInput {
    Message(Value),
    WakeWord(Value),
    Closed,
    Failed(String),
}

pub fn run(credential: BrowserCredential, extension_origin: String) -> Result<(), String> {
    let mut request = credential
        .endpoint
        .into_client_request()
        .map_err(|error| format!("invalid browser bridge endpoint: {error}"))?;
    request.headers_mut().insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {}", credential.token))
            .map_err(|error| format!("invalid bridge authorization header: {error}"))?,
    );
    request.headers_mut().insert(
        "x-deyana-extension-origin",
        HeaderValue::from_str(&extension_origin)
            .map_err(|error| format!("invalid extension origin header: {error}"))?,
    );

    let (mut socket, _) = connect(request)
        .map_err(|error| format!("unable to connect to local Deyana browser bridge: {error}"))?;
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("unable to configure browser bridge socket: {error}"))?;
    }

    let (native_sender, native_receiver) = mpsc::channel();
    
    let ww_sender = native_sender.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            match read_message(&mut reader) {
                Ok(Some(message)) => {
                    if native_sender.send(NativeInput::Message(message)).is_err() {
                        return;
                    }
                }
                Ok(None) => {
                    let _ = native_sender.send(NativeInput::Closed);
                    return;
                }
                Err(error) => {
                    let _ = native_sender.send(NativeInput::Failed(error.to_string()));
                    return;
                }
            }
        }
    });
    thread::spawn(move || {
        // NOTE: Stub for Wake Word Engine (e.g. pvporcupine or rust-vosk).
        // A real implementation requires microphone stream (cpal) and wake word model.
        // For demonstration, we simply let the thread idle.
        loop {
            thread::sleep(Duration::from_secs(3600));
            /*
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let payload = serde_json::json!({
                "type": "browser.wake_word.detected",
                "timestamp": timestamp,
                "payload": {}
            });
            let _ = ww_sender.send(NativeInput::WakeWord(payload));
            */
        }
    });

    let stdout = io::stdout();
    let mut writer = stdout.lock();
    loop {
        match native_receiver.try_recv() {
            Ok(NativeInput::Message(value)) | Ok(NativeInput::WakeWord(value)) => {
                let serialized = serde_json::to_string(&value)
                    .map_err(|error| format!("unable to serialize extension message: {error}"))?;
                socket
                    .send(Message::Text(serialized.into()))
                    .map_err(|error| format!("unable to forward extension message: {error}"))?;
            }
            Ok(NativeInput::Closed) => {
                let _ = socket.close(None);
                return Ok(());
            }
            Ok(NativeInput::Failed(error)) => return Err(error),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return Ok(()),
        }

        match socket.read() {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text)
                    .map_err(|error| format!("core bridge returned invalid JSON: {error}"))?;
                write_message(&mut writer, &value)
                    .map_err(|error| format!("unable to write native response: {error}"))?;
            }
            Ok(Message::Binary(bytes)) => {
                let value: Value = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("core bridge returned invalid JSON bytes: {error}"))?;
                write_message(&mut writer, &value)
                    .map_err(|error| format!("unable to write native response: {error}"))?;
            }
            Ok(Message::Ping(payload)) => {
                socket
                    .send(Message::Pong(payload))
                    .map_err(|error| format!("unable to answer browser bridge ping: {error}"))?;
            }
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => {}
            Err(WebSocketError::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed) => return Ok(()),
            Err(error) => return Err(format!("browser bridge connection failed: {error}")),
        }
    }
}
