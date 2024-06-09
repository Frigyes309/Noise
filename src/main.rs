mod dh_implementation;

use std::char::decode_utf16;
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ptr::null;
use jsonrpc_core::{BoxFuture, IoHandler, Params, Result, Value};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_core::futures::{future};
use jsonrpc_core::futures_util::stream::iter;
use snow::{Builder};
use snow::params::NoiseParams;


const IP_PORT: &str = "127.0.0.1:9999";

#[rpc]
pub trait Rpc {
    #[rpc(name = "protocolVersion")]
    fn protocol_version(&self) -> Result<String>;

    #[rpc(name = "add")]
    fn add(&self, a: u64, b: u64) -> Result<u64>;

    #[rpc(name = "callAsync")]
    fn call(&self, a: u64) -> BoxFuture<Result<String>>;
}

struct RpcImpl;

impl Rpc for RpcImpl {
    fn protocol_version(&self) -> Result<String> {
        Ok("version1".into())
    }

    fn add(&self, a: u64, b: u64) -> Result<u64> {
        Ok(a + b)
    }

    fn call(&self, _: u64) -> BoxFuture<Result<String>> {
        Box::pin(future::ready(Ok("OK".to_owned())))
    }
}

#[allow(unused)]
fn start_server() {
    let mut io = IoHandler::new();
    io.add_method("say_hello", |_params: Params| async {
        Ok(Value::String("hello".to_owned()))
    });
    io.extend_with(RpcImpl.to_delegate());

    let server = ServerBuilder::new(io)
        .threads(3)
        .start_http(&"127.0.0.1:9999".parse().unwrap())
        .unwrap();

    println!("Server is running on {}", IP_PORT);
    server.wait();
}

fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload: ");
    io::stdin().read_line(&mut payload).expect("Failed to read line");
    //insert the length of the payload in the beginning of the message then a char s for separation
    let mut payload = format!("{}s{}", payload.trim().len(), payload);
    payload.trim().to_string()
}

#[cfg(any(feature = "default-resolver", feature = "ring-accelerated"))]
fn run_server(params: NoiseParams, secret: &[u8; 32]) {
    let mut buf = vec![0u8; 65535];

    // Initialize our responder using a builder.
    let builder = Builder::new(params.clone());
    let static_key = builder.generate_keypair().unwrap().private;
    let mut noise = builder
        .local_private_key(&static_key)
        .unwrap()
        .psk(3, secret)
        .unwrap()
        .build_responder()
        .unwrap();

    // Wait on our client's arrival...
    println!("listening on 127.0.0.1:9999");
    let (mut stream, _) = TcpListener::bind("127.0.0.1:9999").unwrap().accept().unwrap();

    // <- e
    noise.read_message(&recv(&mut stream).unwrap(), &mut buf).unwrap();

    // -> e, ee, s, es
    let len = noise.write_message(&[], &mut buf).unwrap();
    send(&mut stream, &buf[..len]);

    // <- s, se
    noise.read_message(&recv(&mut stream).unwrap(), &mut buf).unwrap();

    // Transition the state machine into transport mode now that the handshake is complete.
    //let mut noise = noise.into_transport_mode().unwrap();
    let mut noise = noise.into_transport_mode().unwrap();

    let mut stop = false;
    while !stop {
        match recv(&mut stream) {
            Ok(msg) => {
                let len = noise.read_message(&msg, &mut buf).unwrap();
                let msg = String::from_utf8_lossy(&buf[..len]);
                let mut str_real_len = String::new();
                let mut chars_iter = msg.chars().peekable();
                let mut start_len = 0;
                while let Some(c) = chars_iter.next() {
                    if c == 's' {
                        break;
                    }
                    start_len += 1;
                    str_real_len.push(c);
                }
                start_len += 1;
                let real_len = str_real_len.parse::<usize>().unwrap() + start_len;
                let mut answer: Vec<u8> = Vec::new();
                if String::from_utf8_lossy(&buf[start_len..real_len]).eq("exit") {
                    for c in "exit".bytes() {
                        answer.push(c);
                    }
                    stop = true;
                } else {
                    for i in String::from_utf8_lossy(&buf[start_len..real_len]).chars() {
                        let mut c = i as char;
                        match c {
                            '1' => { c = '0'; }
                            '0' => { c = '1'; }
                            _ => {
                                if c.is_uppercase() {
                                    c = c.to_lowercase().next().unwrap();
                                } else {
                                    c = c.to_uppercase().next().unwrap();

                                }
                            }
                        }
                        answer.push(c as u8);
                    }
                }
                let len = noise.write_message(&(answer), &mut buf).unwrap();
                send(&mut stream, &buf[..len]);
            }
            Err(_) => {
                stop = true;
                println!("Error occurred while reading message.")
            }
        }
    }
    println!("Connection closed.");
}

#[cfg(any(feature = "default-resolver", feature = "ring-accelerated"))]
fn run_client(params: NoiseParams, secret: &[u8; 32]) {
    let mut buf = vec![0u8; 65535];

    // Initialize our initiator using a builder.
    let builder = Builder::new(params.clone());
    let static_key = builder.generate_keypair().unwrap().private;
    let mut noise = builder
        .local_private_key(&static_key)
        .unwrap()
        .psk(3, secret)
        .unwrap()
        .build_initiator()
        .unwrap();

    // Connect to our server, which is hopefully listening.
    let mut stream = TcpStream::connect("127.0.0.1:9999").unwrap();
    println!("Connected...");

    // -> e
    let len = noise.write_message(&[], &mut buf).unwrap();
    send(&mut stream, &buf[..len]);

    // <- e, ee, s, es
    noise.read_message(&recv(&mut stream).unwrap(), &mut buf).unwrap();

    // -> s, se
    let len = noise.write_message(&[], &mut buf).unwrap();
    send(&mut stream, &buf[..len]);

    let mut noise = noise.into_transport_mode().unwrap();
    println!("Session established...");

    let msg = payload_generator();
    let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
    send(&mut stream, &buf[..len]);
    println!("Message sent.");

    loop {
        let msg = recv(&mut stream);
        match msg {
            Ok(msg) => {
                let len = noise.read_message(&msg, &mut buf).unwrap();
                if String::from_utf8_lossy(&buf[..len]).eq("exit") {
                    break;
                }
                println!("Server said: {}", String::from_utf8_lossy(&buf[..len]));
                let msg = payload_generator();
                let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
                send(&mut stream, &buf[..len]);
                println!("Message sent.");
            }
            Err(_) => {
                println!("An error occurred while reading message.");
                break;
            }
        }
    }
    /*while let Ok(msg) = recv(&mut stream) {
        /*match recv(&mut stream) {
            Ok(msg) => {
                let len = noise.read_message(&msg, &mut buf).unwrap();
                println!("Server said: {}", String::from_utf8_lossy(&buf[..len]));
                let msg = payload_generator();
                let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
                send(&mut stream, &buf[..len]);
                println!("Message sent.");
            }
            Err(_) => { stop = true; }
        }*/
        let len = noise.read_message(&msg, &mut buf).unwrap();
        println!("Server said: {}", String::from_utf8_lossy(&buf[..len]));
        let msg = payload_generator();
        let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
        send(&mut stream, &buf[..len]);
        println!("Message sent.");
    }*/
    println!("Connection closed.");
}
fn recv(stream: &mut TcpStream) -> io::Result<Vec<u8>> {

    let mut msg_len_buf = [0u8; 2];
    stream.read_exact(&mut msg_len_buf)?;
    let msg_len = usize::from(u16::from_be_bytes(msg_len_buf));
    let mut msg = vec![0u8; msg_len];
    stream.read_exact(&mut msg[..])?;
    Ok(msg)
}

fn send(stream: &mut TcpStream, buf: &[u8]) {
    let len = u16::try_from(buf.len()).expect("message too large");
    stream.write_all(&len.to_be_bytes()).unwrap();
    stream.write_all(buf).unwrap();
}

fn main() {
    /*let server_mode =
        std::env::args().next_back().map_or(true, |arg | arg == "-s" || arg == "--server");*/
    //Connection will be KX, beacuse we know the public key of the server
    println!("Mode? [s = server]");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    let server_mode = 's' == input.trim().chars().next().unwrap();
    //Will have to change to KX instead of XX
    let params: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    const SECRET: &[u8; 32] = b"Random 32 characters long secret";
    if server_mode {
        println!("Server mode");
        run_server(params.clone(), SECRET);
    } else {
        println!("Client mode");
        run_client(params.clone(), SECRET);
    }
}
