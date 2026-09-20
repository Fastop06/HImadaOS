// RFC 4648 Base64 encoder and decoder in pure no_std Rust

const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode_base64(input: &[u8], output: &mut [u8]) -> usize {
    let mut out_idx = 0;
    let mut i = 0;

    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0x0f) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0x3f) as usize;

        if out_idx + 4 > output.len() {
            break;
        }

        output[out_idx] = CHARS[idx0];
        output[out_idx + 1] = CHARS[idx1];
        output[out_idx + 2] = if i + 1 < input.len() { CHARS[idx2] } else { b'=' };
        output[out_idx + 3] = if i + 2 < input.len() { CHARS[idx3] } else { b'=' };

        out_idx += 4;
        i += 3;
    }

    out_idx
}

fn decode_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        b'=' => Some(0), // padding
        _ => None,
    }
}

pub fn decode_base64(input: &[u8], output: &mut [u8]) -> usize {
    let mut out_idx = 0;
    let mut i = 0;

    while i + 3 < input.len() {
        let c0 = input[i];
        let c1 = input[i + 1];
        let c2 = input[i + 2];
        let c3 = input[i + 3];

        let v0 = match decode_char(c0) { Some(v) => v, None => { i += 1; continue; } };
        let v1 = match decode_char(c1) { Some(v) => v, None => { i += 1; continue; } };
        let v2 = match decode_char(c2) { Some(v) => v, None => { i += 1; continue; } };
        let v3 = match decode_char(c3) { Some(v) => v, None => { i += 1; continue; } };

        if out_idx < output.len() {
            output[out_idx] = (v0 << 2) | (v1 >> 4);
            out_idx += 1;
        }

        if c2 != b'=' && out_idx < output.len() {
            output[out_idx] = ((v1 & 0x0f) << 4) | (v2 >> 2);
            out_idx += 1;
        }

        if c3 != b'=' && out_idx < output.len() {
            output[out_idx] = ((v2 & 0x03) << 6) | v3;
            out_idx += 1;
        }

        i += 4;
    }

    out_idx
}
