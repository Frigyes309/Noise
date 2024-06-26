## How to use after clonging the repository

#### 1. Open the terminal and navigate to the directory where the repository is cloned.

> [!IMPORTANT]
> Before the following points make sure, that 127.0.0.1:9999 is not in use.

#### 2. Run the following command to start a server:

```plaintext
cargo run -- -s
```

#### 3. Run the following command to start a client:

```plaintext
cargo run -- -c
```

#### 4. As the client, you can send messages to the server by typing them in the terminal.

#### Example:

```plaintext
{ "jsonrpc": "2.0", "method": "add", "params": [42, 23], "id": 1 }
```

#### 5. You can exit by sending the following message as the client:

```plaintext
{ "jsonrpc": "2.0", "method": "exit", "params": null, "id": 1 }
```

### Github link for jsonrpsee integration

https://github.com/Frigyes309/jsonrpsee-integration.git

### My current working branch: "rpsee"
