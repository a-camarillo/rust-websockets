use std::{
    collections::HashMap,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use future::{ok, select};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::accept_async;

// transmission end of unbounded mpsc channel with type Message
type Tx = UnboundedSender<Message>;

// a map pairing a socket address and message that is able to be safely shared across threads
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

// asynchronous function for handling client connections
async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred"); // The catch when an error occurs

    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map
    
    // creates an unbounded mpsc channel returns (UnboundedSender, UnboundedReceiver)
    let (tx, rx) = unbounded();
    // acquire the mutex and block the current thread, then insert the address and sender into HashMap
    peer_map.lock().unwrap().insert(addr, tx);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
        let peers = peer_map.lock().unwrap();

        // We want to broadcast the message to everyone except ourselves.
        let broadcast_recipients = peers.iter().filter(|(peer_addr, _)| peer_addr != &&addr).map(|(_,|ws_sink)| ws_sink);

        // send message to recipients
        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }

        ok(())
    });

    // future is basically a promise, this keeps flushing things through the stream until it is finished
    let receive_from_others = rx.map(Ok).forward(outgoing);

    // pins a value on the stack? Like the data structure stack? not entirely sure
    pin_mut!(broadcast_incoming, receive_from_others);
    select(broadcast_incoming, receive_from_others).await;

    print!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    // hard code address LETS GO
    let addr = "127.0.0.1:3012".to_string();

    // create a new empty PeerMap
    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // spawn the handling of each connection in a separate task
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}