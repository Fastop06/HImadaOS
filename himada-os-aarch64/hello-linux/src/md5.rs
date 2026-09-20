// RFC 1321 MD5 Hash implementation in pure no_std Rust

pub struct Md5 {
    state: [u32; 4],
    count: u64,
    buffer: [u8; 64],
}

impl Md5 {
    pub const fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            count: 0,
            buffer: [0; 64],
        }
    }

    pub fn update(&mut self, input: &[u8]) {
        let mut idx = (self.count as usize) & 0x3f;
        self.count += input.len() as u64;

        let mut i = 0;
        let part_len = 64 - idx;

        if input.len() >= part_len {
            self.buffer[idx..64].copy_from_slice(&input[..part_len]);
            let blk = self.buffer;
            self.transform(&blk);
            i = part_len;
            while i + 63 < input.len() {
                let mut blk = [0u8; 64];
                blk.copy_from_slice(&input[i..i + 64]);
                self.transform(&blk);
                i += 64;
            }
            idx = 0;
        }

        if i < input.len() {
            self.buffer[idx..idx + (input.len() - i)].copy_from_slice(&input[i..]);
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        let mut bits = [0u8; 8];
        let bit_count = self.count * 8;
        for j in 0..8 {
            bits[j] = ((bit_count >> (j * 8)) & 0xff) as u8;
        }

        let idx = (self.count as usize) & 0x3f;
        let pad_len = if idx < 56 { 56 - idx } else { 120 - idx };

        let mut padding = [0u8; 64];
        padding[0] = 0x80;
        self.update(&padding[..pad_len]);
        self.update(&bits);

        let mut out = [0u8; 16];
        for i in 0..4 {
            let s = self.state[i];
            out[i * 4] = s as u8;
            out[i * 4 + 1] = (s >> 8) as u8;
            out[i * 4 + 2] = (s >> 16) as u8;
            out[i * 4 + 3] = (s >> 24) as u8;
        }
        out
    }

    fn transform(&mut self, block: &[u8; 64]) {
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];

        let mut x = [0u32; 16];
        for i in 0..16 {
            x[i] = (block[i * 4] as u32)
                | ((block[i * 4 + 1] as u32) << 8)
                | ((block[i * 4 + 2] as u32) << 16)
                | ((block[i * 4 + 3] as u32) << 24);
        }

        #[inline(always)]
        fn f(x: u32, y: u32, z: u32) -> u32 { (x & y) | (!x & z) }
        #[inline(always)]
        fn g(x: u32, y: u32, z: u32) -> u32 { (x & z) | (y & !z) }
        #[inline(always)]
        fn h(x: u32, y: u32, z: u32) -> u32 { x ^ y ^ z }
        #[inline(always)]
        fn i_fn(x: u32, y: u32, z: u32) -> u32 { y ^ (x | !z) }

        macro_rules! ff {
            ($a:expr, $b:expr, $c:expr, $d:expr, $k:expr, $s:expr, $t:expr) => {
                $a = $b.wrapping_add(($a.wrapping_add(f($b, $c, $d)).wrapping_add($k).wrapping_add($t)).rotate_left($s));
            };
        }
        macro_rules! gg {
            ($a:expr, $b:expr, $c:expr, $d:expr, $k:expr, $s:expr, $t:expr) => {
                $a = $b.wrapping_add(($a.wrapping_add(g($b, $c, $d)).wrapping_add($k).wrapping_add($t)).rotate_left($s));
            };
        }
        macro_rules! hh {
            ($a:expr, $b:expr, $c:expr, $d:expr, $k:expr, $s:expr, $t:expr) => {
                $a = $b.wrapping_add(($a.wrapping_add(h($b, $c, $d)).wrapping_add($k).wrapping_add($t)).rotate_left($s));
            };
        }
        macro_rules! ii {
            ($a:expr, $b:expr, $c:expr, $d:expr, $k:expr, $s:expr, $t:expr) => {
                $a = $b.wrapping_add(($a.wrapping_add(i_fn($b, $c, $d)).wrapping_add($k).wrapping_add($t)).rotate_left($s));
            };
        }

        // Round 1
        ff!(a, b, c, d, x[0], 7, 0xd76aa478);
        ff!(d, a, b, c, x[1], 12, 0xe8c7b756);
        ff!(c, d, a, b, x[2], 17, 0x242070db);
        ff!(b, c, d, a, x[3], 22, 0xc1bdceee);
        ff!(a, b, c, d, x[4], 7, 0xf57c0faf);
        ff!(d, a, b, c, x[5], 12, 0x4787c62a);
        ff!(c, d, a, b, x[6], 17, 0xa8304613);
        ff!(b, c, d, a, x[7], 22, 0xfd469501);
        ff!(a, b, c, d, x[8], 7, 0x698098d8);
        ff!(d, a, b, c, x[9], 12, 0x8b44f7af);
        ff!(c, d, a, b, x[10], 17, 0xffff5bb1);
        ff!(b, c, d, a, x[11], 22, 0x895cd7be);
        ff!(a, b, c, d, x[12], 7, 0x6b901122);
        ff!(d, a, b, c, x[13], 12, 0xfd987193);
        ff!(c, d, a, b, x[14], 17, 0xa679438e);
        ff!(b, c, d, a, x[15], 22, 0x49b40821);

        // Round 2
        gg!(a, b, c, d, x[1], 5, 0xf61e2562);
        gg!(d, a, b, c, x[6], 9, 0xc040b340);
        gg!(c, d, a, b, x[11], 14, 0x265e5a51);
        gg!(b, c, d, a, x[0], 20, 0xe9b6c7aa);
        gg!(a, b, c, d, x[5], 5, 0xd62f105d);
        gg!(d, a, b, c, x[10], 9, 0x02441453);
        gg!(c, d, a, b, x[15], 14, 0xd8a1e681);
        gg!(b, c, d, a, x[4], 20, 0xe7d3fbc8);
        gg!(a, b, c, d, x[9], 5, 0x21e1cde6);
        gg!(d, a, b, c, x[14], 9, 0xc33707d6);
        gg!(c, d, a, b, x[3], 14, 0xf4d50d87);
        gg!(b, c, d, a, x[8], 20, 0x455a14ed);
        gg!(a, b, c, d, x[13], 5, 0xa9e3e905);
        gg!(d, a, b, c, x[2], 9, 0xfcefa3f8);
        gg!(c, d, a, b, x[7], 14, 0x676f02d9);
        gg!(b, c, d, a, x[12], 20, 0x8d2a4c8a);

        // Round 3
        hh!(a, b, c, d, x[5], 4, 0xfffa3942);
        hh!(d, a, b, c, x[8], 11, 0x8771f681);
        hh!(c, d, a, b, x[11], 16, 0x6d9d6122);
        hh!(b, c, d, a, x[14], 23, 0xfde5380c);
        hh!(a, b, c, d, x[1], 4, 0xa4beea44);
        hh!(d, a, b, c, x[4], 11, 0x4bdecfa9);
        hh!(c, d, a, b, x[7], 16, 0xf6bb4b60);
        hh!(b, c, d, a, x[10], 23, 0xbebfbc70);
        hh!(a, b, c, d, x[13], 4, 0x289b7ec6);
        hh!(d, a, b, c, x[0], 11, 0xeaa127fa);
        hh!(c, d, a, b, x[3], 16, 0xd4ef3085);
        hh!(b, c, d, a, x[6], 23, 0x04881d05);
        hh!(a, b, c, d, x[9], 4, 0xd9d4d039);
        hh!(d, a, b, c, x[12], 11, 0xe6db99e5);
        hh!(c, d, a, b, x[15], 16, 0x1fa27cf8);
        hh!(b, c, d, a, x[2], 23, 0xc4ac5665);

        // Round 4
        ii!(a, b, c, d, x[0], 6, 0xf4292244);
        ii!(d, a, b, c, x[7], 10, 0x432aff97);
        ii!(c, d, a, b, x[14], 15, 0xab9423a7);
        ii!(b, c, d, a, x[5], 21, 0xfc93a039);
        ii!(a, b, c, d, x[12], 6, 0x655b59c3);
        ii!(d, a, b, c, x[3], 10, 0x8f0ccc92);
        ii!(c, d, a, b, x[10], 15, 0xffeff47d);
        ii!(b, c, d, a, x[1], 21, 0x85845dd1);
        ii!(a, b, c, d, x[8], 6, 0x6fa87e4f);
        ii!(d, a, b, c, x[15], 10, 0xfe2ce6e0);
        ii!(c, d, a, b, x[6], 15, 0xa3014314);
        ii!(b, c, d, a, x[13], 21, 0x4e0811a1);
        ii!(a, b, c, d, x[4], 6, 0xf7537e82);
        ii!(d, a, b, c, x[11], 10, 0xbd3af235);
        ii!(c, d, a, b, x[2], 15, 0x2ad7d2bb);
        ii!(b, c, d, a, x[9], 21, 0xeb86d391);

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

pub fn md5_digest(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize()
}

pub fn md5_hex(data: &[u8], out: &mut [u8; 32]) {
    let digest = md5_digest(data);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for i in 0..16 {
        out[i * 2] = HEX[(digest[i] >> 4) as usize];
        out[i * 2 + 1] = HEX[(digest[i] & 0x0f) as usize];
    }
}
