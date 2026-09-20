use smoltcp::iface::{Config, Interface, SocketSet, SocketHandle};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer, State};
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::time::Instant;
use crate::net::smol_dev::{VirtioSmoltcpDevice, LoopbackDevice};
use crate::hal::device::DEVICE_MANAGER;
use spin::Mutex;
use alloc::vec::Vec;

lazy_static::lazy_static! {
    pub static ref LOOPBACK_IFACE: Mutex<Option<Interface>> = Mutex::new(None);
    pub static ref LOOPBACK_DEV: Mutex<LoopbackDevice> = Mutex::new(LoopbackDevice::new());
    pub static ref LOOPBACK_SOCKETS: Mutex<SocketSet<'static>> = Mutex::new(SocketSet::new(alloc::vec![]));
    pub static ref NET_IFACE: Mutex<Option<Interface>> = Mutex::new(None);
    pub static ref NET_SOCKETS: Mutex<SocketSet<'static>> = Mutex::new(SocketSet::new(alloc::vec![]));
    pub static ref NET_DEV: Mutex<Option<VirtioSmoltcpDevice>> = Mutex::new(None);
}

pub fn now() -> Instant {
    let mut count: u64 = 0;
    let mut freq: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) count);
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) freq);
    }
    let millis = if freq > 0 { (count * 1000) / freq } else { 0 };
    Instant::from_millis(millis as i64)
}

pub fn init() {
    // 1. Initialize Loopback Interface (127.0.0.1/8)
    let mut lo_config = Config::new(HardwareAddress::Ip);
    lo_config.random_seed = 0x87654321;
    let mut lo_iface = Interface::new(lo_config, &mut *LOOPBACK_DEV.lock(), now());
    lo_iface.update_ip_addrs(|ip_addrs| {
        let _ = ip_addrs.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::new(127, 0, 0, 1)), 8));
    });
    *LOOPBACK_IFACE.lock() = Some(lo_iface);
    crate::serial_println!("[Network] Loopback interface (lo) initialized: 127.0.0.1/8 (Active)");

    // 2. Initialize VirtIO Ethernet Interface if device available
    let mut dev_mgr = DEVICE_MANAGER.lock();
    if let Some(net_dev) = dev_mgr.net_devices.first() {
        let mac = net_dev.lock().mac_address();
        let hw_addr = HardwareAddress::Ethernet(EthernetAddress(mac));
        
        let mut config = Config::new(hw_addr);
        config.random_seed = 0x12345678;
        
        let mut iface = Interface::new(config, &mut VirtioSmoltcpDevice { inner: net_dev.clone() }, now());
        
        // QEMU user net default: guest is 10.0.2.15, host router is 10.0.2.2
        iface.update_ip_addrs(|ip_addrs| {
            let _ = ip_addrs.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::new(10, 0, 2, 15)), 24));
        });
        
        // Add default route to gateway 10.0.2.2 for outbound internet access!
        let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2));
        
        *NET_IFACE.lock() = Some(iface);
        *NET_DEV.lock() = Some(VirtioSmoltcpDevice { inner: net_dev.clone() });
        crate::serial_println!("[Network] Smoltcp ethernet interface (eth0) initialized: IP 10.0.2.15/24, Gateway 10.0.2.2");
    } else {
        crate::serial_println!("[Network] No NetDevice found for eth0, running in Loopback-only mode.");
    }
}

pub fn poll() {
    // 1. Poll loopback interface with loopback sockets
    if let Some(ref mut lo_iface) = *LOOPBACK_IFACE.lock() {
        let mut lo_dev = LOOPBACK_DEV.lock();
        let mut lo_sockets = LOOPBACK_SOCKETS.lock();
        let _ = lo_iface.poll(now(), &mut *lo_dev, &mut *lo_sockets);
    }

    // 2. Poll ethernet interface with ethernet sockets
    let mut iface_opt = NET_IFACE.lock();
    let mut dev_opt = NET_DEV.lock();
    if let (Some(iface), Some(dev)) = (iface_opt.as_mut(), dev_opt.as_mut()) {
        let mut net_sockets = NET_SOCKETS.lock();
        let _ = iface.poll(now(), dev, &mut *net_sockets);
    }
}

pub fn connect_lo(handle: SocketHandle, ip: [u8; 4], port: u16) -> Result<(), &'static str> {
    let remote_ip = Ipv4Address::new(ip[0], ip[1], ip[2], ip[3]);
    let remote_endpoint = (IpAddress::Ipv4(remote_ip), port);
    static mut NEXT_LO_PORT: u16 = 49152;
    let local_port = unsafe {
        let p = NEXT_LO_PORT;
        NEXT_LO_PORT = if NEXT_LO_PORT >= 65000 { 49152 } else { NEXT_LO_PORT + 1 };
        p
    };

    {
        let mut sockets = LOOPBACK_SOCKETS.lock();
        let socket = sockets.get_mut::<TcpSocket>(handle);
        let mut lo_opt = LOOPBACK_IFACE.lock();
        let iface = lo_opt.as_mut().ok_or("No loopback interface")?;
        socket.connect(iface.context(), remote_endpoint, local_port)
            .map_err(|_| "Loopback socket connect call failed")?;
    }

    let start = now();
    while (now() - start).total_millis() < 3000 {
        poll();
        {
            let mut sockets = LOOPBACK_SOCKETS.lock();
            let socket = sockets.get_mut::<TcpSocket>(handle);
            match socket.state() {
                State::Established => return Ok(()),
                State::Closed => return Err("Connection closed by peer"),
                _ => {}
            }
        }
        for _ in 0..5_000 { core::hint::spin_loop(); }
    }
    Err("Loopback connection timeout")
}

pub fn connect_eth(handle: SocketHandle, ip: [u8; 4], port: u16) -> Result<(), &'static str> {
    let remote_ip = Ipv4Address::new(ip[0], ip[1], ip[2], ip[3]);
    let remote_endpoint = (IpAddress::Ipv4(remote_ip), port);
    static mut NEXT_ETH_PORT: u16 = 50000;
    let local_port = unsafe {
        let p = NEXT_ETH_PORT;
        NEXT_ETH_PORT = if NEXT_ETH_PORT >= 65000 { 50000 } else { NEXT_ETH_PORT + 1 };
        p
    };

    {
        let mut sockets = NET_SOCKETS.lock();
        let socket = sockets.get_mut::<TcpSocket>(handle);
        let mut iface_opt = NET_IFACE.lock();
        let iface = iface_opt.as_mut().ok_or("No ethernet interface")?;
        socket.connect(iface.context(), remote_endpoint, local_port)
            .map_err(|_| "Ethernet socket connect call failed")?;
    }

    let start = now();
    while (now() - start).total_millis() < 3000 {
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
        for _ in 0..5_000 { core::hint::spin_loop(); }
    }
    Err("Ethernet connection timeout")
}

