use crate::fs::vfs::{FileOps, NodeKind, VfsNode};
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, RwLock};

pub const MAX_PTYS: usize = 8;
pub const PTY_BUF_SIZE: usize = 4096;

pub struct PtyBuffer {
    buf: [u8; PTY_BUF_SIZE],
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl PtyBuffer {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; PTY_BUF_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        let space = PTY_BUF_SIZE - self.count;
        let to_write = data.len().min(space);
        for i in 0..to_write {
            self.buf[self.write_pos] = data[i];
            self.write_pos = (self.write_pos + 1) % PTY_BUF_SIZE;
        }
        self.count += to_write;
        to_write
    }

    pub fn read(&mut self, out: &mut [u8]) -> usize {
        let to_read = self.count.min(out.len());
        for i in 0..to_read {
            out[i] = self.buf[self.read_pos];
            self.read_pos = (self.read_pos + 1) % PTY_BUF_SIZE;
        }
        self.count -= to_read;
        to_read
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub __c_ispeed: u32,
    pub __c_ospeed: u32,
}

impl Default for Termios {
    fn default() -> Self {
        let mut cc = [0u8; 32];
        cc[0] = 3;   // VINTR = ^C
        cc[1] = 28;  // VQUIT = ^\
        cc[2] = 127; // VERASE = DEL
        cc[3] = 21;  // VKILL = ^U
        cc[4] = 4;   // VEOF = ^D
        cc[5] = 0;   // VTIME
        cc[6] = 1;   // VMIN
        Self {
            c_iflag: 0x0500, // ICRNL | IXON
            c_oflag: 0x0005, // OPOST | ONLCR
            c_cflag: 0x00bf, // CS8 | CREAD | B38400
            c_lflag: 0x8a3b, // ISIG | ICANON | ECHO | ECHOE | ECHOK | IEXTEN
            c_line: 0,
            c_cc: cc,
            __c_ispeed: 38400,
            __c_ospeed: 38400,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

impl Default for WinSize {
    fn default() -> Self {
        Self {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

pub struct PtySession {
    pub id: usize,
    pub master_to_slave: Mutex<PtyBuffer>,
    pub slave_to_master: Mutex<PtyBuffer>,
    pub termios: RwLock<Termios>,
    pub winsize: RwLock<WinSize>,
    pub unlocked: RwLock<bool>,
    pub slave_node: RwLock<Option<Arc<VfsNode>>>,
}

impl PtySession {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            master_to_slave: Mutex::new(PtyBuffer::new()),
            slave_to_master: Mutex::new(PtyBuffer::new()),
            termios: RwLock::new(Termios::default()),
            winsize: RwLock::new(WinSize::default()),
            unlocked: RwLock::new(false),
            slave_node: RwLock::new(None),
        }
    }
}

pub static PTY_TABLE: Mutex<[Option<Arc<PtySession>>; MAX_PTYS]> = Mutex::new([
    None, None, None, None, None, None, None, None,
]);

pub struct PtyMasterOps(pub Arc<PtySession>);
impl FileOps for PtyMasterOps {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> usize {
        self.0.slave_to_master.lock().read(buf)
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> usize {
        self.0.master_to_slave.lock().write(buf)
    }
}

pub struct PtySlaveOps(pub Arc<PtySession>);
impl FileOps for PtySlaveOps {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> usize {
        self.0.master_to_slave.lock().read(buf)
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> usize {
        self.0.slave_to_master.lock().write(buf)
    }
}

pub fn open_ptmx() -> Option<(Arc<PtySession>, Arc<VfsNode>)> {
    let mut table = PTY_TABLE.lock();
    for i in 0..MAX_PTYS {
        if table[i].is_none() {
            let session = Arc::new(PtySession::new(i));
            table[i] = Some(session.clone());

            let slave_name = format!("pts/{}", i);
            let slave_node = Arc::new(VfsNode {
                name: format!("{}", i),
                kind: NodeKind::CharDevice,
                size: 0,
                children: RwLock::new(Vec::new()),
                file_ops: Some(Arc::new(PtySlaveOps(session.clone()))),
                data: RwLock::new(Vec::new()),
                static_data: None,
                uid: RwLock::new(0),
                gid: RwLock::new(5),
                mode: RwLock::new(0o620),
            });
            *session.slave_node.write() = Some(slave_node.clone());
            crate::fs::vfs::add_node(&format!("dev/{}", slave_name), slave_node);

            let master_node = Arc::new(VfsNode {
                name: String::from("ptmx"),
                kind: NodeKind::CharDevice,
                size: 0,
                children: RwLock::new(Vec::new()),
                file_ops: Some(Arc::new(PtyMasterOps(session.clone()))),
                data: RwLock::new(Vec::new()),
                static_data: None,
                uid: RwLock::new(0),
                gid: RwLock::new(5),
                mode: RwLock::new(0o666),
            });
            return Some((session, master_node));
        }
    }
    None
}

pub fn get_pty_session(id: usize) -> Option<Arc<PtySession>> {
    let table = PTY_TABLE.lock();
    if id < MAX_PTYS {
        table[id].clone()
    } else {
        None
    }
}
