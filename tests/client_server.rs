use serbytes::prelude::SerBytes;
use simple_tcp_wrapper::prelude::{Incoming, TcpWrapperClient, TcpWrapperServer};

#[derive(SerBytes, Debug, Eq, PartialEq)]
enum S2CPacket {
    Pong,
    ServerData(String),
}

#[derive(SerBytes, Debug, Eq, PartialEq)]
enum C2SPacket {
    Ping,
}

#[test]
fn client_server() {
    // println!("start test");
    let port = 1756;

    let server = TcpWrapperServer::<S2CPacket, C2SPacket>::new(port).expect("Create server");

    let mut client =
        TcpWrapperClient::<C2SPacket, S2CPacket>::new(("127.0.0.1", port)).expect("Create client");

    client
        .send(&C2SPacket::Ping)
        .expect("Send ping packet to server");

    println!("before rcv blocking");
    let Incoming { data, client_id } = server.rcv_blocking().expect("Receive packet from client");

    assert_eq!(data, C2SPacket::Ping);

    // println!("about to call send");
    server
        .send(S2CPacket::Pong, client_id)
        .expect("Send pong packet to client");

    // thread::sleep(Duration::from_millis(500));

    let pong = client
        .rcv_blocking()
        .expect("Receive from client")
        .expect("Packet is None");

    assert_eq!(pong, S2CPacket::Pong);
}
