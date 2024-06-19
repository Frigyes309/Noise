#![allow(unused_imports)]
#![allow(dead_code)]
use futures_util::{SinkExt, StreamExt};

use jsonrpsee::{
    core::{client::ClientT, RpcResult},
    RpcModule,
    ws_client::WsClientBuilder,
    types::{ErrorCode, Params},
    server::ServerBuilder,
    proc_macros::rpc
};
use lazy_static::lazy_static;
use snow::{params::NoiseParams, Builder};
use std::sync::Arc;
use jsonrpsee::server::Server;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use jsonrpsee::core::server::rpc_module::Method;

pub type Error =  Box<dyn std::error::Error>;

const IP_PORT: &str = "127.0.0.1:9999";
lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
}

#[rpc(server)]
pub trait Rpc {
    #[method(name = "add")]
    fn add(&self, a: u64, b: u64) -> RpcResult<u64>;

    #[method(name = "exit")]
    fn exit(&self) -> RpcResult<String>;

    #[method(name = "say_hello")]
    fn say_hello(&self) -> RpcResult<String>;

    #[method(name = "add_i32")]
    fn add_i32(&self, a: i32, b: i32) -> RpcResult<i32>;
}

struct RpcImpl;

impl RpcServer for RpcImpl {
    fn add(&self, a: u64, b: u64) -> RpcResult<u64> {
        Ok(a + b)
    }

    fn exit(&self) -> RpcResult<String> {
        Ok(String::from("exit"))
    }

    fn say_hello(&self) -> RpcResult<String> {
        Ok(String::from("Hello, World!"))
    }

    fn add_i32(&self, a: i32, b: i32) -> RpcResult<i32> {
        Ok(a + b)
    }
}

async fn handle_request(stream: TcpStream) -> Into<RpcImpl> {
    let ws_stream = match accept_async(stream).await {
        Ok(ws_stream) => ws_stream,
        Err(e) => {
            eprintln!("Failed to accept WebSocket connection: {:?}", e);
            return Err(e.into());
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

    //return RpcImpl.into_rpc();

    /*while !stop {
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
    }*/
    println!("Connection closed.");
    Ok(())
}

/*async fn start_websocket_client() {
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
        if let Some(value) =
            serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&buf[..len]))
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
*/
fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload: ");
    std::io::stdin()
        .read_line(&mut payload)
        .expect("Failed to read line");
    payload.trim().to_string()
}

/*async fn handle_request(mut stream: TcpStream, noise: HandshakeState, io_handler: Arc<RpcModule<()>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut noise = noise;

    // Handshake folyamat
    let mut buf = [0u8; 65535];
    let len = noise.write_message(&[], &mut buf)?;
    stream.write_all(&buf[..len]).await?;

    let len = stream.read(&mut buf).await?;
    noise.read_message(&buf[..len], &mut [])?;

    let len = noise.write_message(&[], &mut buf)?;
    stream.write_all(&buf[..len]).await?;

    let len = stream.read(&mut buf).await?;
    let mut payload = vec![0u8; len];
    noise.read_message(&buf[..len], &mut payload)?;

    let mut transport = noise.into_transport_mode()?;

    // Titkosított kommunikáció kezelése
    loop {
        let len = stream.read(&mut buf).await?;
        let mut decrypted_msg = vec![0u8; len];
        transport.read_message(&buf[..len], &mut decrypted_msg)?;

        // Itt feldolgozhatod az üzenetet
        let request_str = std::str::from_utf8(&decrypted_msg)?;
        let response = io_handler.call(request_str, ()).await.unwrap_or_else(|_| "".into());

        let mut encrypted_response = vec![0u8; response.len() + 16];
        transport.write_message(response.as_bytes(), &mut encrypted_response)?;
        stream.write_all(&encrypted_response).await?;
    }
}*/

async fn run_server() -> Result<(), Error> {
    /*let listener = TcpListener::bind(IP_PORT).await.expect("Failed to bind");
    println!("WebSocket server running on {}", IP_PORT);

    let io_handler = Arc::new(Mutex::new({
        let mut io = IoHandler::new();
        io.extend_with(RpcImpl.to_delegate());
        io
    }));

    while let Ok((stream, _)) = listener.accept().await {
        let io_handler = io_handler.clone();
        tokio::spawn(handle_connection(stream, io_handler));
    }*/
    //------------
    //Create server
    let server = ServerBuilder::default()
        .ws_only()
        .build(IP_PORT)
        .await?;

    let tcp_listener = TcpListener::bind(IP_PORT).await?;

    let (stream, _) = tcp_listener.accept().await?;
    //let handle = server.start(RpcImpl.into_rpc());

    let handle = server.start(handle_request(stream));

    /*while let Ok((stream, _)) = handle.accept().await {
        let io_handler = handle.clone();
        tokio::spawn(handle_connection(stream));
    }*/

    handle.stopped().await;

    Ok(())
}

async fn run_client() -> Result<(), Error> {
    let url: String = String::from("ws://127.0.0.1:3030");

    let client = WsClientBuilder::new().build(url).await?;
    let _module = RpcModule::new(());
    println!(
        "Connection {}",
        if client.is_connected() {
            "was successful"
        } else {
            "failed"
        }
    );
    let response: serde_json::Value = client
        .request("say_hello", jsonrpsee::rpc_params![])
        .await?;
    println!("say_hello response (for no params): {}", response);

    let response: serde_json::Value = client
        .request("add_i32", jsonrpsee::rpc_params![9, 1])
        .await?;
    println!("add response (for 9+1): {}", response);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let server_mode;
    if std::env::args().len() > 1 {
        server_mode = std::env::args()
            .next_back()
            .map_or(true, |arg| arg == "-s" || arg == "--server")
    } else {
        println!("Mode? [s = server]");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        server_mode = 's' == input.trim().chars().next().unwrap();
    }
    if server_mode {
        println!("Server mode");
        run_server().await?;
    } else {
        println!("Client mode");
        run_client().await?;
    }

    Ok(())
}
