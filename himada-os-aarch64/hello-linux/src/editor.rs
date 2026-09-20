// Interactive Full-Screen VT100 Text Editor (nano / himada-edit) for HimadaOS

pub const MAX_EDITOR_BUF: usize = 8192;
pub const SCREEN_ROWS: usize = 24;
pub const SCREEN_COLS: usize = 80;

pub struct TextEditor {
    pub buf: [u8; MAX_EDITOR_BUF],
    pub len: usize,
    pub cursor_pos: usize, // byte offset in buf
    pub top_line: usize,
    pub filename: [u8; 64],
    pub filename_len: usize,
    pub modified: bool,
    pub status_msg: [u8; 64],
    pub status_msg_len: usize,
}

impl TextEditor {
    pub const fn new() -> Self {
        Self {
            buf: [0; MAX_EDITOR_BUF],
            len: 0,
            cursor_pos: 0,
            top_line: 0,
            filename: [0; 64],
            filename_len: 0,
            modified: false,
            status_msg: [0; 64],
            status_msg_len: 0,
        }
    }

    pub fn set_filename(&mut self, name: &str) {
        let b = name.as_bytes();
        let l = b.len().min(63);
        self.filename[..l].copy_from_slice(&b[..l]);
        self.filename[l] = 0;
        self.filename_len = l;
    }

    pub fn set_status(&mut self, msg: &str) {
        let b = msg.as_bytes();
        let l = b.len().min(63);
        self.status_msg[..l].copy_from_slice(&b[..l]);
        self.status_msg[l] = 0;
        self.status_msg_len = l;
    }

    pub fn load_content(&mut self, content: &[u8]) {
        let l = content.len().min(MAX_EDITOR_BUF);
        self.buf[..l].copy_from_slice(&content[..l]);
        self.len = l;
        self.cursor_pos = 0;
        self.top_line = 0;
        self.modified = false;
    }

    pub fn insert_char(&mut self, c: u8) {
        if self.len >= MAX_EDITOR_BUF - 1 {
            return;
        }

        let mut i = self.len;
        while i > self.cursor_pos {
            self.buf[i] = self.buf[i - 1];
            i -= 1;
        }
        self.buf[self.cursor_pos] = c;
        self.cursor_pos += 1;
        self.len += 1;
        self.modified = true;
    }

    pub fn delete_backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        self.cursor_pos -= 1;
        let mut i = self.cursor_pos;
        while i + 1 < self.len {
            self.buf[i] = self.buf[i + 1];
            i += 1;
        }
        self.len -= 1;
        self.modified = true;
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos >= self.len {
            return;
        }
        let mut i = self.cursor_pos;
        while i + 1 < self.len {
            self.buf[i] = self.buf[i + 1];
            i += 1;
        }
        self.len -= 1;
        self.modified = true;
    }

    // Cursor movement helpers
    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_pos < self.len {
            self.cursor_pos += 1;
        }
    }

    pub fn move_home(&mut self) {
        while self.cursor_pos > 0 && self.buf[self.cursor_pos - 1] != b'\n' {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_end(&mut self) {
        while self.cursor_pos < self.len && self.buf[self.cursor_pos] != b'\n' {
            self.cursor_pos += 1;
        }
    }

    pub fn move_up(&mut self) {
        // Find start of current line
        let mut cur_line_start = self.cursor_pos;
        while cur_line_start > 0 && self.buf[cur_line_start - 1] != b'\n' {
            cur_line_start -= 1;
        }
        if cur_line_start == 0 {
            return; // Already on first line
        }
        let col = self.cursor_pos - cur_line_start;

        // Find start of previous line
        let prev_line_end = cur_line_start - 1;
        let mut prev_line_start = prev_line_end;
        while prev_line_start > 0 && self.buf[prev_line_start - 1] != b'\n' {
            prev_line_start -= 1;
        }
        let prev_line_len = prev_line_end - prev_line_start;
        self.cursor_pos = prev_line_start + col.min(prev_line_len);
    }

    pub fn move_down(&mut self) {
        // Find end of current line
        let mut cur_line_end = self.cursor_pos;
        while cur_line_end < self.len && self.buf[cur_line_end] != b'\n' {
            cur_line_end += 1;
        }
        if cur_line_end >= self.len {
            return; // Already on last line
        }
        let mut cur_line_start = self.cursor_pos;
        while cur_line_start > 0 && self.buf[cur_line_start - 1] != b'\n' {
            cur_line_start -= 1;
        }
        let col = self.cursor_pos - cur_line_start;

        let next_line_start = cur_line_end + 1;
        let mut next_line_end = next_line_start;
        while next_line_end < self.len && self.buf[next_line_end] != b'\n' {
            next_line_end += 1;
        }
        let next_line_len = next_line_end - next_line_start;
        self.cursor_pos = next_line_start + col.min(next_line_len);
    }

    pub fn render<F: FnMut(&str)>(&self, mut print_fn: F) {
        // 1. Move to home position
        print_fn("\x1b[H");

        // 2. Top Header Bar
        print_fn("\x1b[7m Himada-Edit 1.0      File: ");
        if let Ok(name) = core::str::from_utf8(&self.filename[..self.filename_len]) {
            print_fn(name);
        } else {
            print_fn("new_file.txt");
        }
        if self.modified {
            print_fn(" [Modified]");
        } else {
            print_fn("           ");
        }
        print_fn("                                   \x1b[0m\r\n");

        // 3. Viewport lines
        let text = &self.buf[..self.len];
        let mut line_num = 0;
        let mut char_idx = 0;
        let view_rows = SCREEN_ROWS.saturating_sub(4); // reserved for header & footer

        // Skip to top_line
        while line_num < self.top_line && char_idx < text.len() {
            if text[char_idx] == b'\n' {
                line_num += 1;
            }
            char_idx += 1;
        }

        let mut printed_rows = 0;
        while printed_rows < view_rows {
            print_fn("\x1b[2K"); // Clear row
            if char_idx < text.len() {
                let start = char_idx;
                while char_idx < text.len() && text[char_idx] != b'\n' {
                    char_idx += 1;
                }
                let line_bytes = &text[start..char_idx];
                if let Ok(s) = core::str::from_utf8(line_bytes) {
                    print_fn(s);
                }
                if char_idx < text.len() && text[char_idx] == b'\n' {
                    char_idx += 1;
                }
            } else {
                print_fn("~");
            }
            print_fn("\r\n");
            printed_rows += 1;
        }

        // 4. Status Bar
        print_fn("\x1b[7m");
        if self.status_msg_len > 0 {
            if let Ok(msg) = core::str::from_utf8(&self.status_msg[..self.status_msg_len]) {
                print_fn(" ");
                print_fn(msg);
            }
        } else {
            print_fn(" ^O WriteOut     ^X Exit Editor     ^L Refresh View");
        }
        print_fn("                                              \x1b[0m\r\n");

        // 5. Calculate cursor row and col on screen
        let mut cur_row = 2; // after header (1-indexed row 2)
        let mut cur_col = 1;
        let mut idx = 0;
        let mut l_count = 0;

        while idx < self.cursor_pos && idx < self.len {
            if self.buf[idx] == b'\n' {
                l_count += 1;
                cur_col = 1;
            } else {
                cur_col += 1;
            }
            idx += 1;
        }

        if l_count >= self.top_line {
            cur_row = 2 + (l_count - self.top_line);
        }

        // Position terminal hardware cursor
        let mut pos_buf = [0u8; 32];
        let mut p = 0;
        pos_buf[p] = b'\x1b'; p += 1;
        pos_buf[p] = b'['; p += 1;
        p += write_u64(&mut pos_buf[p..], cur_row as u64);
        pos_buf[p] = b';'; p += 1;
        p += write_u64(&mut pos_buf[p..], cur_col as u64);
        pos_buf[p] = b'H'; p += 1;
        if let Ok(s) = core::str::from_utf8(&pos_buf[..p]) {
            print_fn(s);
        }
    }
}

fn write_u64(buf: &mut [u8], mut val: u64) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut temp = [0u8; 20];
    let mut i = 0;
    while val > 0 {
        temp[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    for j in 0..i {
        buf[j] = temp[i - 1 - j];
    }
    i
}
