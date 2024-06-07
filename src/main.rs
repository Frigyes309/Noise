//mod noise_protocolv2local;

mod dh_implementation;

use jsonrpc_core::{BoxFuture, Error, IoHandler, Params, Result, Value};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_core_client::transports::local;
use jsonrpc_core::futures::{self, future, TryFutureExt};
#[allow(unused_imports)]
use std::thread;
use hyper::header::Keys;
//mod adder;
//use noise_protocolv2local::noise_params::NoiseProtocol;
use noise_protocol::{HandshakeState, HandshakeStateBuilder};
use noise_protocol::patterns::{HandshakePattern, Token};
use noise_protocol::{DH, U8Array};
use snow::resolvers::Dh25519;


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
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .unwrap();

    println!("Server is running on 127.0.0.1:3030");
    server.wait();
}

#[allow(unused)]
async fn run_client() {
    // Client code for adding
    let mut io = IoHandler::new();
    io.extend_with(RpcImpl.to_delegate());

    let (client, server) = local::connect::<gen_client::Client, _, _>(io.clone());
    let fut_add = client.add(5, 6).map_ok(|res| println!("5 + 6 = {}", res));

    // Client code for protocol_version
    /*let mut io = IoHandler::new();
    io.extend_with(RpcImpl.to_delegate());*/

    let (client, server2) = local::connect::<gen_client::Client, _, _>(io.clone());
    let fut_protocol_version = client.protocol_version().map_ok(|res| println!("Protocol version: {}", res));

    // Client code for call
    /*let mut io = IoHandler::new();
    io.extend_with(RpcImpl.to_delegate());*/

    let (client, server3) = local::connect::<gen_client::Client, _, _>(io.clone());
    let fut_call = client.call(41).map_ok(|res| println!("Call result: {}", res));

    /*let (client, server4) = local::connect::<gen_client::Client, _, _>(io.clone());
    let called_method = handler.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "callAsync",
        "params": [41],
        "id": 1
    }));
    let fut_random = match  called_method {
        _ => client.call(41).map_ok(|res| println!("Call result: {}", res));

    };*/

    // Await each future sequentially
    let _ = futures::join!(fut_protocol_version, fut_add, fut_call, server, server2, server3/*, server4, fut_random*/ );
}

fn main() {
    // Start the server in a new thread
    /*thread::spawn(|| {
        start_server();
    });

    // Give the server a moment to start
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Run the client
    futures::executor::block_on(run_client());

    adder::check();*/


    //Since I know the public key of the server
    //I will use KX pattern
    //let h = HandshakePattern::new();

    //let HSS = HandshakeState::new("Noise_KX_123456_Random_Text", true, "RandomPrologue");

    //-------------------
    let server_mode =
        std::env::args().next_back().map_or(true, |arg| arg == "-s" || arg == "--server");

    /*if server_mode {
        run_server();
        println!("Server done.");
    } else {
        run_client();
        println!("Client done.");
    }*/
    //-------------- Dead end from here


    let dh:DH<Key = U8Array, Pubkey = U8Array, > = DH::new();
    dh.genkey();
    print!("Type: {}", dh.name());
    //DH::Curve25519
    /*let mut HSB: HandshakeStateBuilder<> = HandshakeStateBuilder::new();

    let pre_i = &[Token::S];
    let pre_r = &[Token::E];
    let msg_pattern: &[&[Token]] = &[&[Token::E], &[Token::E, Token::EE, Token::SE, Token::S, Token::ES]];
    let pattern = HandshakePattern::new(pre_i, pre_r, msg_pattern, "RandomName".clone());
    HSB.set_pattern(pattern);
    HSB.set_prologue("RandomPrologue".as_bytes());
    HSB.set_is_initiator(true);
    let mut handshake = HSB.build_handshake_state();
    let mut out = [0u8; 1024];
    handshake.write_message("111000110010010011000111".as_bytes(), &mut out).unwrap();
    let mut msg2 = "000111001101101100111000".as_bytes();
    let mut out2 = [0u8; 1024];
    handshake.read_message(&mut msg2, &mut out2).unwrap();
    println!("Handshake done.");
    println!("out: {}\nout2: {}", String::from_utf8_lossy(&out), String::from_utf8_lossy(&out2));
    */
    //------------------- End of dead end


    //-------------------
    /*
    let mut io = IoHandler::default();

    io.add_method("say_hello", |_params| async {
        Ok(Value::String("Hello".to_string()))
    });

    io.add_method("add", |params: Params| async {
        let tuple = params.parse::<(u32, u32)>();
        match tuple {
            Ok((a, b)) => {
                Ok(Value::Number(serde_json::Number::from(a + b)))
            },
            Err(_) => Err(Error::invalid_params("expected two integers of two numbers")),
        }
    });

    let server = ServerBuilder::new(io)
        .threads(3)
        .start_http(&"127.0.0.1:3030".parse().unwrap()).unwrap();

    println!("Server is running on 127.0.0.1:3030");
    server.wait();*/
}
