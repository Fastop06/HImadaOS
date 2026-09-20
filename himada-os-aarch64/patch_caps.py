with open('src/net/smol_dev.rs', 'r') as f:
    content = f.read()

old_caps = """    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps
    }"""

new_caps = """    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1514;
        caps.medium = smoltcp::phy::Medium::Ethernet;
        caps
    }"""

content = content.replace(old_caps, new_caps)
with open('src/net/smol_dev.rs', 'w') as f:
    f.write(content)
