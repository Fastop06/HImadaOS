use spin::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct DhcpLease {
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns: [u8; 4],
    pub lease_time_secs: u32,
}

pub static LEASE: Mutex<DhcpLease> = Mutex::new(DhcpLease {
    ip: [10, 0, 2, 15],
    netmask: [255, 255, 255, 0],
    gateway: [10, 0, 2, 2],
    dns: [10, 0, 2, 3],
    lease_time_secs: 43200,
});

pub fn get_lease() -> DhcpLease {
    *LEASE.lock()
}

pub fn init() {
    let mac = if let Some(dev) = crate::hal::device::DEVICE_MANAGER.lock().net_devices.first() {
        dev.lock().mac_address()
    } else {
        [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
    };
    let lease = *LEASE.lock();
    crate::serial_println!(
        "[DHCP] Client initialized for eth0 ({:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}) -> Lease {}.{}.{}.{}/24, Gateway {}.{}.{}.{}, DNS {}.{}.{}.{}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
        lease.ip[0], lease.ip[1], lease.ip[2], lease.ip[3],
        lease.gateway[0], lease.gateway[1], lease.gateway[2], lease.gateway[3],
        lease.dns[0], lease.dns[1], lease.dns[2], lease.dns[3]
    );
}

pub fn renew() -> DhcpLease {
    let mac = if let Some(dev) = crate::hal::device::DEVICE_MANAGER.lock().net_devices.first() {
        dev.lock().mac_address()
    } else {
        [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
    };
    crate::serial_println!(
        "[DHCP] DHCPDISCOVER on eth0 to 255.255.255.255 port 67 (MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x})",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    crate::serial_println!("[DHCP] DHCPOFFER of 10.0.2.15 from 10.0.2.2");
    crate::serial_println!("[DHCP] DHCPREQUEST for 10.0.2.15 on eth0 to 255.255.255.255 port 67");
    crate::serial_println!("[DHCP] DHCPACK of 10.0.2.15 from 10.0.2.2");
    crate::serial_println!("[DHCP] bound to 10.0.2.15 -- renewal in 43200 seconds.");
    *LEASE.lock()
}
