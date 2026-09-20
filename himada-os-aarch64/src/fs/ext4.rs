use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use crate::hal::device::BlockDevice;

pub const EXT4_SUPERBLOCK_OFFSET: u64 = 1024;
pub const EXT4_MAGIC: u16 = 0xEF53;
pub const EXT4_EXTENTS_FL: u32 = 0x0008_0000;
pub const EXT4_EXT_MAGIC: u16 = 0xF30A;

pub const S_IFDIR: u16 = 0x4000;
pub const S_IFREG: u16 = 0x8000;
pub const S_IFLNK: u16 = 0xA000;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Ext4Superblock {
    pub s_inodes_count: u32,
    pub s_blocks_count_lo: u32,
    pub s_r_blocks_count_lo: u32,
    pub s_free_blocks_count_lo: u32,
    pub s_free_inodes_count: u32,
    pub s_first_data_block: u32,
    pub s_log_block_size: u32,
    pub s_log_cluster_size: u32,
    pub s_blocks_per_group: u32,
    pub s_clusters_per_group: u32,
    pub s_inodes_per_group: u32,
    pub s_mtime: u32,
    pub s_wtime: u32,
    pub s_mnt_count: u16,
    pub s_max_mnt_count: u16,
    pub s_magic: u16,
    pub s_state: u16,
    pub s_errors: u16,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: u32,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    // Dynamic revision fields (s_rev_level >= 1)
    pub s_first_ino: u32,
    pub s_inode_size: u16,
    pub s_block_group_nr: u16,
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
    pub s_uuid: [u8; 16],
    pub s_volume_name: [u8; 16],
    pub s_last_mounted: [u8; 64],
    pub s_algorithm_usage_bitmap: u32,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: u16,
    pub s_journal_uuid: [u8; 16],
    pub s_journal_inum: u32,
    pub s_journal_dev: u32,
    pub s_last_orphan: u32,
    pub s_hash_seed: [u32; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_desc_size: u16,
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32,
    pub s_mkfs_time: u32,
    pub s_jnl_blocks: [u32; 17],
    pub s_blocks_count_hi: u32,
    pub s_r_blocks_count_hi: u32,
    pub s_free_blocks_count_hi: u32,
    pub s_min_extra_isize: u16,
    pub s_want_extra_isize: u16,
    pub s_flags: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Ext4GroupDesc {
    pub bg_block_bitmap_lo: u32,
    pub bg_inode_bitmap_lo: u32,
    pub bg_inode_table_lo: u32,
    pub bg_free_blocks_count_lo: u16,
    pub bg_free_inodes_count_lo: u16,
    pub bg_used_dirs_count_lo: u16,
    pub bg_flags: u16,
    pub bg_exclude_bitmap_lo: u32,
    pub bg_block_bitmap_csum_lo: u16,
    pub bg_inode_bitmap_csum_lo: u16,
    pub bg_itable_unused_lo: u16,
    pub bg_checksum: u16,
    // 64-bit extension fields
    pub bg_block_bitmap_hi: u32,
    pub bg_inode_bitmap_hi: u32,
    pub bg_inode_table_hi: u32,
    pub bg_free_blocks_count_hi: u16,
    pub bg_free_inodes_count_hi: u16,
    pub bg_used_dirs_count_hi: u16,
    pub bg_itable_unused_hi: u16,
    pub bg_exclude_bitmap_hi: u32,
    pub bg_block_bitmap_csum_hi: u16,
    pub bg_inode_bitmap_csum_hi: u16,
    pub bg_reserved: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Ext4Inode {
    pub i_mode: u16,
    pub i_uid: u16,
    pub i_size_lo: u32,
    pub i_atime: u32,
    pub i_ctime: u32,
    pub i_mtime: u32,
    pub i_dtime: u32,
    pub i_gid: u16,
    pub i_links_count: u16,
    pub i_blocks_lo: u32,
    pub i_flags: u32,
    pub i_osd1: u32,
    pub i_block: [u8; 60],
    pub i_generation: u32,
    pub i_file_acl_lo: u32,
    pub i_size_high: u32,
    pub i_obso_faddr: u32,
    pub i_osd2: [u8; 12],
    pub i_extra_isize: u16,
    pub i_checksum_hi: u16,
    pub i_ctime_extra: u32,
    pub i_mtime_extra: u32,
    pub i_atime_extra: u32,
    pub i_crtime: u32,
    pub i_crtime_extra: u32,
    pub i_version_hi: u32,
    pub i_projid: u32,
}

impl Ext4Inode {
    pub fn size(&self) -> u64 {
        if (self.i_mode & 0xF000) == S_IFREG {
            ((self.i_size_high as u64) << 32) | (self.i_size_lo as u64)
        } else {
            self.i_size_lo as u64
        }
    }

    pub fn is_dir(&self) -> bool {
        (self.i_mode & 0xF000) == S_IFDIR
    }

    pub fn is_reg(&self) -> bool {
        (self.i_mode & 0xF000) == S_IFREG
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Ext4ExtentHeader {
    pub eh_magic: u16,
    pub eh_entries: u16,
    pub eh_max: u16,
    pub eh_depth: u16,
    pub eh_generation: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Ext4Extent {
    pub ee_block: u32,
    pub ee_len: u16,
    pub ee_start_hi: u16,
    pub ee_start_lo: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Ext4ExtentIdx {
    pub ei_block: u32,
    pub ei_leaf_lo: u32,
    pub ei_leaf_hi: u16,
    pub ei_unused: u16,
}

#[derive(Clone, Debug)]
pub struct Ext4DirEntry {
    pub inode: u32,
    pub file_type: u8,
    pub name: String,
}

pub struct Ext4Filesystem {
    pub dev: Arc<Mutex<dyn BlockDevice>>,
    pub sb: Ext4Superblock,
    pub block_size: usize,
    pub group_descs: Vec<Ext4GroupDesc>,
}

impl Ext4Filesystem {
    pub fn new(dev: Arc<Mutex<dyn BlockDevice>>) -> Result<Self, &'static str> {
        let mut sb_buf = [0u8; 1024];
        // Superblock is at offset 1024 bytes (LBA = 2, 2 sectors of 512 bytes)
        {
            let d = dev.lock();
            d.read_sectors(2, 2, &mut sb_buf)?;
        }

        let sb = unsafe { core::ptr::read_unaligned(sb_buf.as_ptr() as *const Ext4Superblock) };
        let magic = sb.s_magic;
        if magic != EXT4_MAGIC {
            crate::serial_println!("[Ext4] Invalid magic: 0x{:04X} (expected 0x{:04X})", magic, EXT4_MAGIC);
            return Err("Not an ext4 filesystem");
        }

        let block_size = (1024usize) << sb.s_log_block_size;
        let blocks_count = if (sb.s_feature_incompat & 0x80) != 0 {
            ((sb.s_blocks_count_hi as u64) << 32) | (sb.s_blocks_count_lo as u64)
        } else {
            sb.s_blocks_count_lo as u64
        };

        let blocks_per_group = sb.s_blocks_per_group as u64;
        let group_count = ((blocks_count + blocks_per_group - 1) / blocks_per_group) as usize;
        let desc_size = if sb.s_desc_size >= 32 { sb.s_desc_size as usize } else { 32 };

        let inodes_count = sb.s_inodes_count;
        let inode_size = sb.s_inode_size;
        crate::serial_println!(
            "[Ext4] Mounted successfully: BlockSize={} B, Groups={}, Inodes={}, InodeSize={}",
            block_size, group_count, inodes_count, inode_size
        );

        // Group descriptors follow the superblock block
        // For block_size == 1024: superblock is in block 1, descriptors in block 2
        // For block_size >= 2048: superblock is in block 0, descriptors in block 1
        let gd_block = if block_size == 1024 { 2u64 } else { 1u64 };
        let gdt_bytes = group_count * desc_size;
        let gdt_blocks = (gdt_bytes + block_size - 1) / block_size;

        let mut gdt_buf = alloc::vec![0u8; gdt_blocks * block_size];
        for b in 0..gdt_blocks {
            let curr_block = gd_block + (b as u64);
            let offset = b * block_size;
            Self::read_block_raw(&dev, curr_block, block_size, &mut gdt_buf[offset..offset + block_size])?;
        }

        let mut group_descs = Vec::with_capacity(group_count);
        for g in 0..group_count {
            let offset = g * desc_size;
            let desc = unsafe {
                core::ptr::read_unaligned(gdt_buf.as_ptr().add(offset) as *const Ext4GroupDesc)
            };
            group_descs.push(desc);
        }

        Ok(Self {
            dev,
            sb,
            block_size,
            group_descs,
        })
    }

    fn read_block_raw(dev: &Arc<Mutex<dyn BlockDevice>>, block_nr: u64, block_size: usize, buf: &mut [u8]) -> Result<(), &'static str> {
        let sectors_per_block = (block_size / 512) as u16;
        let lba = block_nr * (sectors_per_block as u64);
        let d = dev.lock();
        d.read_sectors(lba, sectors_per_block, buf)?;
        Ok(())
    }

    fn write_block_raw(dev: &Arc<Mutex<dyn BlockDevice>>, block_nr: u64, block_size: usize, buf: &[u8]) -> Result<(), &'static str> {
        let sectors_per_block = (block_size / 512) as u16;
        let lba = block_nr * (sectors_per_block as u64);
        let d = dev.lock();
        d.write_sectors(lba, sectors_per_block, buf)?;
        Ok(())
    }

    pub fn read_block(&self, block_nr: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        Self::read_block_raw(&self.dev, block_nr, self.block_size, buf)
    }

    pub fn write_block(&self, block_nr: u64, buf: &[u8]) -> Result<(), &'static str> {
        Self::write_block_raw(&self.dev, block_nr, self.block_size, buf)
    }

    pub fn read_inode(&self, ino: u32) -> Result<Ext4Inode, &'static str> {
        if ino == 0 || ino > self.sb.s_inodes_count {
            return Err("Invalid inode number");
        }

        let group = ((ino - 1) / self.sb.s_inodes_per_group) as usize;
        let index = ((ino - 1) % self.sb.s_inodes_per_group) as usize;

        if group >= self.group_descs.len() {
            return Err("Group index out of bounds");
        }

        let gd = &self.group_descs[group];
        let table_block = ((gd.bg_inode_table_hi as u64) << 32) | (gd.bg_inode_table_lo as u64);
        let inode_size = self.sb.s_inode_size as usize;
        let byte_offset = index * inode_size;
        let block_offset = byte_offset / self.block_size;
        let offset_in_block = byte_offset % self.block_size;

        let mut buf = alloc::vec![0u8; self.block_size];
        self.read_block(table_block + block_offset as u64, &mut buf)?;

        let inode = unsafe {
            core::ptr::read_unaligned(buf.as_ptr().add(offset_in_block) as *const Ext4Inode)
        };

        Ok(inode)
    }

    pub fn write_inode(&self, ino: u32, inode: &Ext4Inode) -> Result<(), &'static str> {
        if ino == 0 || ino > self.sb.s_inodes_count {
            return Err("Invalid inode number");
        }

        let group = ((ino - 1) / self.sb.s_inodes_per_group) as usize;
        let index = ((ino - 1) % self.sb.s_inodes_per_group) as usize;

        if group >= self.group_descs.len() {
            return Err("Group index out of bounds");
        }

        let gd = &self.group_descs[group];
        let table_block = ((gd.bg_inode_table_hi as u64) << 32) | (gd.bg_inode_table_lo as u64);
        let inode_size = self.sb.s_inode_size as usize;
        let byte_offset = index * inode_size;
        let block_offset = byte_offset / self.block_size;
        let offset_in_block = byte_offset % self.block_size;

        let mut buf = alloc::vec![0u8; self.block_size];
        self.read_block(table_block + block_offset as u64, &mut buf)?;

        let target_ptr = unsafe { buf.as_mut_ptr().add(offset_in_block) as *mut Ext4Inode };
        unsafe {
            core::ptr::write_unaligned(target_ptr, *inode);
        }

        self.write_block(table_block + block_offset as u64, &buf)?;
        Ok(())
    }

    pub fn get_inode_blocks(&self, inode: &Ext4Inode) -> Result<Vec<u64>, &'static str> {
        let mut blocks = Vec::new();

        if (inode.i_flags & EXT4_EXTENTS_FL) != 0 {
            let header = unsafe {
                core::ptr::read_unaligned(inode.i_block.as_ptr() as *const Ext4ExtentHeader)
            };
            if header.eh_magic != EXT4_EXT_MAGIC {
                return Err("Corrupted extent header magic");
            }
            self.collect_extent_blocks(&header, &inode.i_block[12..], &mut blocks)?;
        } else {
            // Direct block addressing (ext2/ext3 fallback)
            for i in 0..12 {
                let blk = u32::from_le_bytes(inode.i_block[i * 4..i * 4 + 4].try_into().unwrap());
                if blk != 0 {
                    blocks.push(blk as u64);
                }
            }
        }

        Ok(blocks)
    }

    fn collect_extent_blocks(&self, header: &Ext4ExtentHeader, slice: &[u8], out: &mut Vec<u64>) -> Result<(), &'static str> {
        if header.eh_depth == 0 {
            // Leaf extent nodes
            for i in 0..(header.eh_entries as usize) {
                let offset = i * core::mem::size_of::<Ext4Extent>();
                if offset + core::mem::size_of::<Ext4Extent>() > slice.len() {
                    break;
                }
                let ext = unsafe {
                    core::ptr::read_unaligned(slice.as_ptr().add(offset) as *const Ext4Extent)
                };
                let pblock = ((ext.ee_start_hi as u64) << 32) | (ext.ee_start_lo as u64);
                let count = ext.ee_len.min(32768) as u64;
                for b in 0..count {
                    out.push(pblock + b);
                }
            }
        } else {
            // Internal index nodes
            for i in 0..(header.eh_entries as usize) {
                let offset = i * core::mem::size_of::<Ext4ExtentIdx>();
                if offset + core::mem::size_of::<Ext4ExtentIdx>() > slice.len() {
                    break;
                }
                let idx = unsafe {
                    core::ptr::read_unaligned(slice.as_ptr().add(offset) as *const Ext4ExtentIdx)
                };
                let child_pblock = ((idx.ei_leaf_hi as u64) << 32) | (idx.ei_leaf_lo as u64);

                let mut node_buf = alloc::vec![0u8; self.block_size];
                self.read_block(child_pblock, &mut node_buf)?;

                let child_header = unsafe {
                    core::ptr::read_unaligned(node_buf.as_ptr() as *const Ext4ExtentHeader)
                };
                if child_header.eh_magic == EXT4_EXT_MAGIC {
                    self.collect_extent_blocks(&child_header, &node_buf[12..], out)?;
                }
            }
        }
        Ok(())
    }

    pub fn read_file_data(&self, inode: &Ext4Inode) -> Result<Vec<u8>, &'static str> {
        let size = inode.size() as usize;
        let blocks = self.get_inode_blocks(inode)?;
        let mut data = Vec::with_capacity(size);

        let mut block_buf = alloc::vec![0u8; self.block_size];
        let mut bytes_left = size;

        for blk in blocks {
            if bytes_left == 0 {
                break;
            }
            self.read_block(blk, &mut block_buf)?;
            let to_copy = bytes_left.min(self.block_size);
            data.extend_from_slice(&block_buf[..to_copy]);
            bytes_left -= to_copy;
        }

        Ok(data)
    }

    pub fn read_dir_entries(&self, inode: &Ext4Inode) -> Result<Vec<Ext4DirEntry>, &'static str> {
        if !inode.is_dir() {
            return Err("Inode is not a directory");
        }

        let blocks = self.get_inode_blocks(inode)?;
        let mut entries = Vec::new();
        let mut block_buf = alloc::vec![0u8; self.block_size];

        for blk in blocks {
            self.read_block(blk, &mut block_buf)?;
            let mut offset = 0;
            while offset + 8 <= self.block_size {
                let ino = u32::from_le_bytes(block_buf[offset..offset + 4].try_into().unwrap());
                let rec_len = u16::from_le_bytes(block_buf[offset + 4..offset + 6].try_into().unwrap()) as usize;
                let name_len = block_buf[offset + 6] as usize;
                let file_type = block_buf[offset + 7];

                if rec_len == 0 {
                    break;
                }

                if ino != 0 && name_len > 0 && offset + 8 + name_len <= self.block_size {
                    if let Ok(name) = core::str::from_utf8(&block_buf[offset + 8..offset + 8 + name_len]) {
                        entries.push(Ext4DirEntry {
                            inode: ino,
                            file_type,
                            name: name.to_string(),
                        });
                    }
                }

                offset += rec_len;
            }
        }

        Ok(entries)
    }

    pub fn lookup_path(&self, path: &str) -> Result<u32, &'static str> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
        let mut current_ino = 2u32; // Root inode in ext4 is 2

        for part in parts {
            let inode = self.read_inode(current_ino)?;
            if !inode.is_dir() {
                return Err("Component is not a directory");
            }

            let entries = self.read_dir_entries(&inode)?;
            let mut found = None;
            for entry in entries {
                if entry.name == part {
                    found = Some(entry.inode);
                    break;
                }
            }

            match found {
                Some(next_ino) => current_ino = next_ino,
                None => return Err("File not found in ext4 directory"),
            }
        }

        Ok(current_ino)
    }

    pub fn alloc_block(&mut self) -> Result<u64, &'static str> {
        let num_groups = self.group_descs.len();
        for g in 0..num_groups {
            if self.group_descs[g].bg_free_blocks_count_lo > 0 {
                let b_bitmap = ((self.group_descs[g].bg_block_bitmap_hi as u64) << 32) | (self.group_descs[g].bg_block_bitmap_lo as u64);
                let mut bitmap_buf = alloc::vec![0u8; self.block_size];
                self.read_block(b_bitmap, &mut bitmap_buf)?;

                for byte_idx in 0..bitmap_buf.len() {
                    if bitmap_buf[byte_idx] != 0xFF {
                        for bit in 0..8 {
                            if (bitmap_buf[byte_idx] & (1 << bit)) == 0 {
                                bitmap_buf[byte_idx] |= 1 << bit;
                                self.write_block(b_bitmap, &bitmap_buf)?;

                                self.group_descs[g].bg_free_blocks_count_lo = self.group_descs[g].bg_free_blocks_count_lo.saturating_sub(1);
                                self.sync_group_desc(g)?;

                                let block_in_group = (byte_idx * 8 + bit) as u64;
                                let global_block = (g as u64) * (self.sb.s_blocks_per_group as u64) + block_in_group + (self.sb.s_first_data_block as u64);
                                return Ok(global_block);
                            }
                        }
                    }
                }
            }
        }
        Err("No free blocks in ext4")
    }

    pub fn alloc_inode(&mut self) -> Result<u32, &'static str> {
        let num_groups = self.group_descs.len();
        for g in 0..num_groups {
            if self.group_descs[g].bg_free_inodes_count_lo > 0 {
                let i_bitmap = ((self.group_descs[g].bg_inode_bitmap_hi as u64) << 32) | (self.group_descs[g].bg_inode_bitmap_lo as u64);
                let mut bitmap_buf = alloc::vec![0u8; self.block_size];
                self.read_block(i_bitmap, &mut bitmap_buf)?;

                for byte_idx in 0..bitmap_buf.len() {
                    if bitmap_buf[byte_idx] != 0xFF {
                        for bit in 0..8 {
                            if (bitmap_buf[byte_idx] & (1 << bit)) == 0 {
                                bitmap_buf[byte_idx] |= 1 << bit;
                                self.write_block(i_bitmap, &bitmap_buf)?;

                                self.group_descs[g].bg_free_inodes_count_lo = self.group_descs[g].bg_free_inodes_count_lo.saturating_sub(1);
                                self.group_descs[g].bg_itable_unused_lo = self.group_descs[g].bg_itable_unused_lo.saturating_sub(1);
                                self.sync_group_desc(g)?;

                                let inode_in_group = (byte_idx * 8 + bit + 1) as u32;
                                let global_inode = (g as u32) * self.sb.s_inodes_per_group + inode_in_group;
                                return Ok(global_inode);
                            }
                        }
                    }
                }
            }
        }
        Err("No free inodes in ext4")
    }

    pub fn sync_superblock(&mut self) -> Result<(), &'static str> {
        let mut sb_bytes = [0u8; 1024];
        {
            let d = self.dev.lock();
            d.read_sectors(2, 2, &mut sb_bytes)?;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(
                &self.sb as *const Ext4Superblock as *const u8,
                sb_bytes.as_mut_ptr(),
                core::mem::size_of::<Ext4Superblock>(),
            );
        }
        self.dev.lock().write_sectors(2, 2, &sb_bytes)?;
        Ok(())
    }

    fn sync_group_desc(&self, group: usize) -> Result<(), &'static str> {
        let gd_block = if self.block_size == 1024 { 2u64 } else { 1u64 };
        let desc_size = if self.sb.s_desc_size >= 32 { self.sb.s_desc_size as usize } else { 32 };
        let byte_offset = group * desc_size;
        let block_offset = byte_offset / self.block_size;
        let offset_in_block = byte_offset % self.block_size;

        let mut buf = alloc::vec![0u8; self.block_size];
        self.read_block(gd_block + block_offset as u64, &mut buf)?;

        let target_ptr = unsafe { buf.as_mut_ptr().add(offset_in_block) as *mut Ext4GroupDesc };
        unsafe {
            core::ptr::write_unaligned(target_ptr, self.group_descs[group]);
        }

        self.write_block(gd_block + block_offset as u64, &buf)?;
        Ok(())
    }

    pub fn create_file(&mut self, parent_ino: u32, name: &str, data: &[u8]) -> Result<u32, &'static str> {
        let parent = self.read_inode(parent_ino)?;
        if !parent.is_dir() {
            return Err("Parent is not a directory");
        }

        // 1. Allocate inode
        let new_ino = self.alloc_inode()?;

        // 2. Allocate blocks for data (ensure at least 1 block allocated)
        let blocks_needed = if data.is_empty() { 1 } else { (data.len() + self.block_size - 1) / self.block_size };
        let mut allocated_blocks = Vec::new();
        for _ in 0..blocks_needed {
            let b = self.alloc_block()?;
            allocated_blocks.push(b);
        }

        // 3. Write data to allocated blocks
        let mut block_buf = alloc::vec![0u8; self.block_size];
        let mut bytes_written = 0;
        for &blk in &allocated_blocks {
            block_buf.fill(0);
            let remaining = data.len().saturating_sub(bytes_written);
            let to_write = remaining.min(self.block_size);
            if to_write > 0 {
                block_buf[..to_write].copy_from_slice(&data[bytes_written..bytes_written + to_write]);
                bytes_written += to_write;
            }
            self.write_block(blk, &block_buf)?;
        }

        // 4. Construct Inode with leaf extent
        let mut new_inode: Ext4Inode = unsafe { core::mem::zeroed() };
        new_inode.i_mode = S_IFREG | 0o644;
        new_inode.i_size_lo = data.len() as u32;
        new_inode.i_links_count = 1;
        new_inode.i_blocks_lo = (allocated_blocks.len() * (self.block_size / 512)) as u32;
        new_inode.i_flags = EXT4_EXTENTS_FL;

        let ext_header = Ext4ExtentHeader {
            eh_magic: EXT4_EXT_MAGIC,
            eh_entries: 1,
            eh_max: 4,
            eh_depth: 0,
            eh_generation: 0,
        };

        let first_block = allocated_blocks.first().cloned().unwrap_or(0);
        let ext = Ext4Extent {
            ee_block: 0,
            ee_len: allocated_blocks.len() as u16,
            ee_start_hi: (first_block >> 32) as u16,
            ee_start_lo: (first_block & 0xFFFF_FFFF) as u32,
        };

        unsafe {
            core::ptr::write_unaligned(new_inode.i_block.as_mut_ptr() as *mut Ext4ExtentHeader, ext_header);
            core::ptr::write_unaligned(new_inode.i_block.as_mut_ptr().add(12) as *mut Ext4Extent, ext);
        }

        self.write_inode(new_ino, &new_inode)?;

        // 5. Append directory entry to parent directory
        self.add_dir_entry(parent_ino, new_ino, name, 1)?; // 1 = regular file

        // 6. Update superblock stats and sync
        self.sb.s_free_inodes_count = self.sb.s_free_inodes_count.saturating_sub(1);
        self.sb.s_free_blocks_count_lo = self.sb.s_free_blocks_count_lo.saturating_sub(blocks_needed as u32);
        let _ = self.sync_superblock();

        crate::serial_println!("[Ext4] Created file '{}' (inode={}, size={} B, blocks={:?})", name, new_ino, data.len(), allocated_blocks);
        Ok(new_ino)
    }

    fn add_dir_entry(&mut self, dir_ino: u32, entry_ino: u32, name: &str, file_type: u8) -> Result<(), &'static str> {
        let parent = self.read_inode(dir_ino)?;
        let blocks = self.get_inode_blocks(&parent)?;
        if blocks.is_empty() {
            return Err("Directory has no blocks");
        }

        let last_blk = *blocks.last().unwrap();
        let mut buf = alloc::vec![0u8; self.block_size];
        self.read_block(last_blk, &mut buf)?;

        let mut offset = 0;
        let mut last_entry_offset = 0;

        while offset + 8 <= self.block_size {
            let rec_len = u16::from_le_bytes(buf[offset + 4..offset + 6].try_into().unwrap()) as usize;
            if rec_len == 0 {
                break;
            }
            last_entry_offset = offset;
            offset += rec_len;
        }

        let old_rec_len = u16::from_le_bytes(buf[last_entry_offset + 4..last_entry_offset + 6].try_into().unwrap()) as usize;
        let old_name_len = buf[last_entry_offset + 6] as usize;
        let old_actual_len = (8 + old_name_len + 3) & !3; // 4-byte aligned

        let needed_len = (8 + name.len() + 3) & !3;
        let remaining_space = old_rec_len.saturating_sub(old_actual_len);

        if remaining_space >= needed_len {
            // Shrink the old entry's rec_len to its actual length
            buf[last_entry_offset + 4..last_entry_offset + 6].copy_from_slice(&(old_actual_len as u16).to_le_bytes());

            // Place new entry right after
            let new_entry_offset = last_entry_offset + old_actual_len;
            let new_rec_len = remaining_space as u16;

            buf[new_entry_offset..new_entry_offset + 4].copy_from_slice(&entry_ino.to_le_bytes());
            buf[new_entry_offset + 4..new_entry_offset + 6].copy_from_slice(&new_rec_len.to_le_bytes());
            buf[new_entry_offset + 6] = name.len() as u8;
            buf[new_entry_offset + 7] = file_type;
            buf[new_entry_offset + 8..new_entry_offset + 8 + name.len()].copy_from_slice(name.as_bytes());

            self.write_block(last_blk, &buf)?;
            Ok(())
        } else {
            Err("Directory block full (multi-block directory extension not implemented)")
        }
    }
}
