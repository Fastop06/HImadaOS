with open('src/net/socket.rs', 'r') as f:
    content = f.read()

connect_old = """    // Wait until established
    for _ in 0..500 {
        poll();
        {
            let mut sockets = NET_SOCKETS.lock();
            let socket = sockets.get_mut::<TcpSocket>(handle);
            match socket.state() {
                State::Established => return Ok(()),
                State::Closed => return Err("Connection closed by peer"),
                _ => {}
            }
        }
        for _ in 0..10_000 { core::hint::spin_loop(); }
    }
    Err("Connection timeout to gateway/remote")"""

connect_new = """    let start = now();
    // Wait up to 5000ms for ARP + SYN-ACK over QEMU network
    while (now() - start).total_millis() < 5000 {
        poll();
        {
            let mut sockets = NET_SOCKETS.lock();
            let socket = sockets.get_mut::<TcpSocket>(handle);
            match socket.state() {
                State::Established => return Ok(()),
                State::Closed => return Err("Connection closed by peer"),
                _ => {}
            }
        }
        for _ in 0..50_000 { core::hint::spin_loop(); }
    }
    Err("Connection timeout to gateway/remote")"""

content = content.replace(connect_old, connect_new)
with open('src/net/socket.rs', 'w') as f:
    f.write(content)
