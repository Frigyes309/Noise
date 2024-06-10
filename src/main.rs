use futures_util::{SinkExt, StreamExt};
use jsonrpc_core::{IoHandler, Result};
use jsonrpc_derive::rpc;
use lazy_static::lazy_static;
use snow::{Builder, params::NoiseParams};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{accept_async, connect_async};
use std::sync::Arc;
use tokio::sync::Mutex;

const IP_PORT: &str = "127.0.0.1:9999";
lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
}

#[rpc]
pub trait Rpc {
    #[rpc(name = "add")]
    fn add(&self, a: u64, b: u64) -> Result<u64>;

    #[rpc(name = "exit")]
    fn exit(&self) -> Result<String>;
}

struct RpcImpl;

impl Rpc for RpcImpl {
    fn add(&self, a: u64, b: u64) -> Result<u64> {
        Ok(a + b)
    }

    fn exit(&self) -> Result<String> {
        Ok(String::from("exit"))
    }
}

async fn start_websocket_server() {
    let listener = TcpListener::bind(IP_PORT).await.expect("Failed to bind");
    println!("WebSocket server running on {}", IP_PORT);

    let io_handler = Arc::new(Mutex::new({
        let mut io = IoHandler::new();
        io.extend_with(RpcImpl.to_delegate());
        io
    }));

    while let Ok((stream, _)) = listener.accept().await {
        let io_handler = io_handler.clone();
        tokio::spawn(handle_connection(stream, io_handler));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, io_handler: Arc<Mutex<IoHandler>>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws_stream) => ws_stream,
        Err(e) => {
            eprintln!("Failed to accept WebSocket connection: {:?}", e);
            return;
        }
    };
    let (mut write, mut read) = ws_stream.split();

    let builder = Builder::new(NOISE_PARAMS.clone());
    let static_key = builder.generate_keypair().unwrap().private;
    let mut noise = builder
        .local_private_key(&static_key)
        .unwrap()
        .psk(3, &SECRET.clone())
        .unwrap()
        .build_responder()
        .unwrap();
    let mut buf = vec![0u8; 65535];

    // <- e
    let msg = read.next().await.unwrap().unwrap();
    noise.read_message(&msg.into_data(), &mut buf).unwrap();

    // -> e, ee, s, es
    let len = noise.write_message(&[], &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    // <- s, se
    let msg = read.next().await.unwrap().unwrap();
    noise.read_message(&msg.into_data(), &mut buf).unwrap();

    let mut noise = noise.into_transport_mode().unwrap();
    let mut stop = false;

    while !stop {
        if let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            let len = noise.read_message(&msg.into_data(), &mut buf).unwrap();
            let msg = String::from_utf8_lossy(&buf[..len]);

            let response_message = {
                let io_handler = io_handler.lock().await;
                io_handler.handle_request(&msg).await
            };

            if let Some(response) = response_message {
                println!("Response message: {}", response);
                if let Some(value) = serde_json::from_str::<serde_json::Value>(&response)
                    .unwrap()
                    .get("result")
                    .and_then(|v| v.as_str())
                {
                    if value == "exit" {
                        stop = true;
                    }
                }
                let len = noise.write_message(response.as_bytes(), &mut buf).unwrap();
                write.send(Message::binary(&buf[..len])).await.unwrap();
            }
        }
    }
    println!("Connection closed.");
}

async fn start_websocket_client() {
    let url = format!("ws://{}", IP_PORT);
    let (mut write, mut read) = match connect_async(&url).await {
        Ok((ws_stream, _)) => ws_stream.split(),
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
            return;
        }
    };

    let builder = Builder::new(NOISE_PARAMS.clone());
    let static_key = builder.generate_keypair().unwrap().private;
    let mut noise = builder
        .local_private_key(&static_key)
        .unwrap()
        .psk(3, &SECRET.clone())
        .unwrap()
        .build_initiator()
        .unwrap();
    let mut buf = vec![0u8; 65535];

    // -> e
    let len = noise.write_message(&[], &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    // <- e, ee, s, es
    let msg = read.next().await.unwrap().unwrap();
    noise.read_message(&msg.into_data(), &mut buf).unwrap();

    // -> s, se
    let len = noise.write_message(&[], &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    let mut noise = noise.into_transport_mode().unwrap();
    println!("Session established...");

    let msg = payload_generator();
    let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();
    println!("Message sent.");

    loop {
        let msg = read.next().await.unwrap().unwrap();
        let len = noise.read_message(&msg.into_data(), &mut buf).unwrap();
        if String::from_utf8_lossy(&buf[..len]).eq("exit") {
            break;
        }
        println!("Server said: {}", String::from_utf8_lossy(&buf[..len]));
        if let Some(value) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&buf[..len]))
            .unwrap()
            .get("result")
            .and_then(|v| v.as_str())
        {
            if value == "exit" {
                break;
            }
        }
        let msg = payload_generator();
        let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
        write.send(Message::binary(&buf[..len])).await.unwrap();
        println!("Message sent.");
    }
    println!("Connection closed.");
}

fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload: ");
    std::io::stdin().read_line(&mut payload).expect("Failed to read line");
    payload.trim().to_string()
}

#[tokio::main]
async fn main() {
    let mut server_mode = false;
    if std::env::args().len() > 1 {
        server_mode = std::env::args().next_back().map_or(true, |arg| arg == "-s" || arg == "--server")
    } else {
        println!("Mode? [s = server]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        server_mode = 's' == input.trim().chars().next().unwrap();
    }
    if server_mode {
        println!("Server mode");
        start_websocket_server().await;
    } else {
        println!("Client mode");
        start_websocket_client().await;
    }
}
