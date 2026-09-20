use alloc::vec::Vec;
use alloc::string::String;
use smoltcp::socket::udp::{Socket as UdpSocket, PacketBuffer as UdpPacketBuffer, PacketMetadata as UdpPacketMetadata};
use smoltcp::wire::{IpAddress, Ipv4Address, IpEndpoint};

pub fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 { return None; }
    let mut ip = [0u8; 4];
    for i in 0..4 {
        if let Ok(b) = parts[i].parse::<u8>() {
            ip[i] = b;
        } else {
            return None;
        }
    }
    Some(ip)
}

pub fn parse_dns_response(data: &[u8], expected_id: u16) -> Option<[u8; 4]> {
    if data.len() < 12 { return None; }
    let id = u16::from_be_bytes([data[0], data[1]]);
    if id != expected_id { return None; }
    let flags = u16::from_be_bytes([data[2], data[3]]);
    let is_response = (flags & 0x8000) != 0;
    let rcode = flags & 0x000F;
    if !is_response || rcode != 0 { return None; }
    
    let qdcount = u16::from_be_bytes([data[4], data[5]]) as usize;
    let ancount = u16::from_be_bytes([data[6], data[7]]) as usize;
    if ancount == 0 { return None; }

    let mut pos = 12;
    // Skip Question section
    for _ in 0..qdcount {
        while pos < data.len() {
            let len = data[pos] as usize;
            pos += 1;
            if len == 0 { break; }
            pos += len;
        }
        pos += 4; // QTYPE + QCLASS
    }

    // Parse Answers
    for _ in 0..ancount {
        if pos >= data.len() { break; }
        // Name: pointer (0xC0..) or labels
        if (data[pos] & 0xC0) == 0xC0 {
            pos += 2;
        } else {
            while pos < data.len() {
                let len = data[pos] as usize;
                pos += 1;
                if len == 0 { break; }
                pos += len;
            }
        }
        if pos + 10 > data.len() { break; }
        let a_type = u16::from_be_bytes([data[pos], data[pos+1]]);
        let a_class = u16::from_be_bytes([data[pos+2], data[pos+3]]);
        let rdlength = u16::from_be_bytes([data[pos+8], data[pos+9]]) as usize;
        pos += 10;
        if a_type == 1 && a_class == 1 && rdlength == 4 && pos + 4 <= data.len() {
            return Some([data[pos], data[pos+1], data[pos+2], data[pos+3]]);
        }
        pos += rdlength;
    }
    None
}

pub fn resolve_hostname(raw_name: &str) -> Option<[u8; 4]> {
    let mut name = raw_name.trim();
    if name.starts_with("http://") {
        name = &name["http://".len()..];
    } else if name.starts_with("https://") {
        name = &name["https://".len()..];
    }
    if let Some(slash_idx) = name.find('/') {
        name = &name[..slash_idx];
    }
    if let Some(colon_idx) = name.find(':') {
        name = &name[..colon_idx];
    }
    let clean_name = name.trim();

    if let Some(ip) = parse_ipv4(clean_name) {
        return Some(ip);
    }

    match clean_name {
        "localhost" | "localhost.localdomain" => return Some([127, 0, 0, 1]),
        "himada.org" | "docs.himada.org" | "support.himada.org" => return Some([10, 0, 2, 2]),
        "gateway" | "qemu.gateway" => return Some([10, 0, 2, 2]),
        "dns" | "qemu.dns" => return Some([10, 0, 2, 3]),
        _ => {}
    }

    // Attempt live UDP DNS query over VirtIO-Net
    let query_id: u16 = 0x5a5a;
    let mut query = Vec::new();
    query.extend_from_slice(&query_id.to_be_bytes());
    query.extend_from_slice(&0x0100u16.to_be_bytes()); // RD=1
    query.extend_from_slice(&0x0001u16.to_be_bytes()); // QDCOUNT=1
    query.extend_from_slice(&0x0000u16.to_be_bytes()); // ANCOUNT=0
    query.extend_from_slice(&0x0000u16.to_be_bytes()); // NSCOUNT=0
    query.extend_from_slice(&0x0000u16.to_be_bytes()); // ARCOUNT=0

    for part in clean_name.split('.') {
        let b = part.as_bytes();
        if b.is_empty() || b.len() > 63 { break; }
        query.push(b.len() as u8);
        query.extend_from_slice(b);
    }
    query.push(0);
    query.extend_from_slice(&0x0001u16.to_be_bytes()); // QTYPE: A
    query.extend_from_slice(&0x0001u16.to_be_bytes()); // QCLASS: IN

    let rx_buffer = UdpPacketBuffer::new(alloc::vec![UdpPacketMetadata::EMPTY; 4], alloc::vec![0; 1024]);
    let tx_buffer = UdpPacketBuffer::new(alloc::vec![UdpPacketMetadata::EMPTY; 4], alloc::vec![0; 1024]);
    let mut udp_socket = UdpSocket::new(rx_buffer, tx_buffer);
    let _ = udp_socket.bind(54321);

    let handle = {
        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
        sockets.add(udp_socket)
    };

    let dns_server = IpEndpoint::new(
        IpAddress::Ipv4(Ipv4Address::new(10, 0, 2, 3)),
        53
    );

    {
        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
        let socket = sockets.get_mut::<UdpSocket>(handle);
        let _ = socket.send_slice(&query, dns_server);
    }

    let start = crate::net::socket::now();
    let mut resolved_ip = None;
    while (crate::net::socket::now() - start).total_millis() < 800 {
        crate::net::socket::poll();
        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
        let socket = sockets.get_mut::<UdpSocket>(handle);
        if socket.can_recv() {
            let mut resp = [0u8; 1024];
            if let Ok((len, _)) = socket.recv_slice(&mut resp) {
                if let Some(ip) = parse_dns_response(&resp[..len], query_id) {
                    resolved_ip = Some(ip);
                    break;
                }
            }
        }
        drop(sockets);
        for _ in 0..5_000 { core::hint::spin_loop(); }
    }

    {
        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
        sockets.remove(handle);
    }

    if let Some(ip) = resolved_ip {
        return Some(ip);
    }

    // Static fallback for standard server endpoints when network is offline / isolated
    match clean_name {
        "google.com" | "www.google.com" => Some([142, 250, 180, 206]),
        "kernel.org" | "www.kernel.org" => Some([139, 178, 84, 217]),
        "ubuntu.com" | "archive.ubuntu.com" => Some([185, 125, 190, 36]),
        "github.com" | "api.github.com" => Some([140, 82, 121, 4]),
        "cloudflare.com" => Some([104, 16, 132, 229]),
        "debian.org" => Some([128, 31, 0, 62]),
        _ => None,
    }
}
