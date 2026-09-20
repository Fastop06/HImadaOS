use smoltcp::phy::{Device, DeviceCapabilities, RxToken, TxToken};
use smoltcp::time::Instant;
use alloc::vec::Vec;
use spin::Mutex;
use alloc::sync::Arc;
use crate::hal::device::NetDevice;

pub struct VirtioSmoltcpDevice {
    pub inner: Arc<Mutex<dyn NetDevice>>,
}

static RX_SCRATCH: Mutex<[u8; 2048]> = Mutex::new([0u8; 2048]);

impl Device for VirtioSmoltcpDevice {
    type RxToken<'a> = RxTokenImpl<'a>;
    type TxToken<'a> = TxTokenImpl<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut net = self.inner.lock();
        let mut scratch = RX_SCRATCH.lock();
        
        match net.receive(&mut *scratch) {
            Ok(len) if len > 0 => {
                let mut buf = alloc::vec![0u8; len];
                buf.copy_from_slice(&scratch[..len]);
                Some((
                    RxTokenImpl { buffer: buf, _phantom: core::marker::PhantomData },
                    TxTokenImpl { inner: self.inner.clone(), _phantom: core::marker::PhantomData }
                ))
            }
            _ => None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(TxTokenImpl { inner: self.inner.clone(), _phantom: core::marker::PhantomData })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1514;
        caps.medium = smoltcp::phy::Medium::Ethernet;
        caps
    }
}

pub struct RxTokenImpl<'a> {
    buffer: Vec<u8>,
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> RxToken for RxTokenImpl<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buf = self.buffer;
        f(&mut buf)
    }
}

pub struct TxTokenImpl<'a> {
    inner: Arc<Mutex<dyn NetDevice>>,
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> TxToken for TxTokenImpl<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buf = alloc::vec![0; len];
        let result = f(&mut buf);
        let mut net = self.inner.lock();
        let _ = net.transmit(&buf);
        result
    }
}

pub struct LoopbackDevice {
    pub queue: Mutex<alloc::collections::VecDeque<Vec<u8>>>,
}

impl LoopbackDevice {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(alloc::collections::VecDeque::new()),
        }
    }
}

impl Device for LoopbackDevice {
    type RxToken<'a> = LoopbackRxToken;
    type TxToken<'a> = LoopbackTxToken<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut queue = self.queue.lock();
        if let Some(buf) = queue.pop_front() {
            Some((
                LoopbackRxToken { buffer: buf },
                LoopbackTxToken { queue: &self.queue },
            ))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(LoopbackTxToken { queue: &self.queue })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 65535;
        caps.medium = smoltcp::phy::Medium::Ip;
        caps
    }
}

pub struct LoopbackRxToken {
    buffer: Vec<u8>,
}

impl RxToken for LoopbackRxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.buffer)
    }
}

pub struct LoopbackTxToken<'a> {
    queue: &'a Mutex<alloc::collections::VecDeque<Vec<u8>>>,
}

impl<'a> TxToken for LoopbackTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buf = alloc::vec![0u8; len];
        let res = f(&mut buf);
        self.queue.lock().push_back(buf);
        res
    }
}

