// Interactive ANSI Terminal Line Editor for HimadaOS

pub const MAX_LINE: usize = 2048;
pub const MAX_HISTORY: usize = 32;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum EscState {
    Normal,
    Esc,
    Bracket,
    Bracket3, // for Delete key (\x1b[3~)
    O,        // for \x1bOH / \x1bOF
}

pub enum EditorAction {
    None,
    Submit,
    Interrupt,
    Eof,
}

pub struct LineEditor {
    pub buf: [u8; MAX_LINE],
    pub len: usize,
    pub cursor: usize,
    pub esc_state: EscState,
    pub history: [[u8; MAX_LINE]; MAX_HISTORY],
    pub history_lens: [usize; MAX_HISTORY],
    pub history_count: usize,
    pub history_idx: usize,
    pub saved_buf: [u8; MAX_LINE],
    pub saved_len: usize,
}

impl LineEditor {
    pub const fn new() -> Self {
        Self {
            buf: [0; MAX_LINE],
            len: 0,
            cursor: 0,
            esc_state: EscState::Normal,
            history: [[0; MAX_LINE]; MAX_HISTORY],
            history_lens: [0; MAX_HISTORY],
            history_count: 0,
            history_idx: 0,
            saved_buf: [0; MAX_LINE],
            saved_len: 0,
        }
    }

    pub fn reset_line(&mut self) {
        self.len = 0;
        self.cursor = 0;
        self.esc_state = EscState::Normal;
        self.history_idx = self.history_count;
        self.saved_len = 0;
    }

    pub fn add_history(&mut self) {
        if self.len == 0 {
            return;
        }

        // Avoid duplicate consecutive commands
        if self.history_count > 0 {
            let last_idx = (self.history_count - 1) % MAX_HISTORY;
            if self.history_lens[last_idx] == self.len
                && &self.history[last_idx][..self.len] == &self.buf[..self.len]
            {
                self.history_idx = self.history_count;
                return;
            }
        }

        let slot = self.history_count % MAX_HISTORY;
        self.history[slot][..self.len].copy_from_slice(&self.buf[..self.len]);
        self.history_lens[slot] = self.len;
        self.history_count += 1;
        self.history_idx = self.history_count;
    }

    /// Feeds one byte from stdin. Returns EditorAction.
    pub fn feed_byte<F: FnMut(&str)>(
        &mut self,
        b: u8,
        mut print_fn: F,
        prompt_fn: &dyn Fn(),
        completer_fn: &dyn Fn(&str, &mut [([u8; 64], usize); 16]) -> usize,
    ) -> EditorAction {
        match self.esc_state {
            EscState::Normal => {
                match b {
                    0x1b => {
                        self.esc_state = EscState::Esc;
                        EditorAction::None
                    }
                    b'\r' | b'\n' => {
                        print_fn("\n");
                        self.add_history();
                        EditorAction::Submit
                    }
                    0x08 | 0x7f => { // Backspace
                        self.handle_backspace(&mut print_fn);
                        EditorAction::None
                    }
                    0x01 => { // Ctrl+A (Home)
                        self.handle_home(&mut print_fn);
                        EditorAction::None
                    }
                    0x05 => { // Ctrl+E (End)
                        self.handle_end(&mut print_fn);
                        EditorAction::None
                    }
                    0x03 => { // Ctrl+C
                        print_fn("^C\n");
                        self.reset_line();
                        EditorAction::Interrupt
                    }
                    0x04 => { // Ctrl+D
                        if self.len == 0 {
                            EditorAction::Eof
                        } else {
                            self.handle_delete(&mut print_fn);
                            EditorAction::None
                        }
                    }
                    0x0b => { // Ctrl+K (Kill to end)
                        self.handle_kill_to_end(&mut print_fn);
                        EditorAction::None
                    }
                    0x15 => { // Ctrl+U (Kill line to start)
                        self.handle_kill_to_start(&mut print_fn);
                        EditorAction::None
                    }
                    0x17 => { // Ctrl+W (Erase word)
                        self.handle_erase_word(&mut print_fn);
                        EditorAction::None
                    }
                    0x0c => { // Ctrl+L (Clear screen)
                        print_fn("\x1b[2J\x1b[H");
                        prompt_fn();
                        if let Ok(s) = core::str::from_utf8(&self.buf[..self.len]) {
                            print_fn(s);
                        }
                        if self.cursor < self.len {
                            self.move_cursor_back(self.len - self.cursor, &mut print_fn);
                        }
                        EditorAction::None
                    }
                    0x09 => { // Tab completion
                        self.handle_tab(prompt_fn, &mut print_fn, completer_fn);
                        EditorAction::None
                    }
                    0x20..=0x7e => { // Printable ASCII
                        self.insert_char(b, &mut print_fn);
                        EditorAction::None
                    }
                    _ => EditorAction::None,
                }
            }
            EscState::Esc => {
                match b {
                    b'[' => {
                        self.esc_state = EscState::Bracket;
                        EditorAction::None
                    }
                    b'O' => {
                        self.esc_state = EscState::O;
                        EditorAction::None
                    }
                    _ => {
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                }
            }
            EscState::Bracket => {
                match b {
                    b'A' => { // Arrow Up
                        self.history_up(&mut print_fn);
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                    b'B' => { // Arrow Down
                        self.history_down(&mut print_fn);
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                    b'C' => { // Arrow Right
                        self.move_right(&mut print_fn);
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                    b'D' => { // Arrow Left
                        self.move_left(&mut print_fn);
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                    b'H' | b'1' => { // Home
                        self.handle_home(&mut print_fn);
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                    b'F' | b'4' => { // End
                        self.handle_end(&mut print_fn);
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                    b'3' => { // Delete key (\x1b[3~)
                        self.esc_state = EscState::Bracket3;
                        EditorAction::None
                    }
                    _ => {
                        self.esc_state = EscState::Normal;
                        EditorAction::None
                    }
                }
            }
            EscState::Bracket3 => {
                if b == b'~' {
                    self.handle_delete(&mut print_fn);
                }
                self.esc_state = EscState::Normal;
                EditorAction::None
            }
            EscState::O => {
                match b {
                    b'H' => self.handle_home(&mut print_fn),
                    b'F' => self.handle_end(&mut print_fn),
                    _ => {}
                }
                self.esc_state = EscState::Normal;
                EditorAction::None
            }
        }
    }

    pub fn insert_char<F: FnMut(&str)>(&mut self, b: u8, print_fn: &mut F) {
        if self.len >= MAX_LINE - 1 {
            return;
        }

        if self.cursor == self.len {
            self.buf[self.cursor] = b;
            self.cursor += 1;
            self.len += 1;
            let ch = [b];
            if let Ok(s) = core::str::from_utf8(&ch) {
                print_fn(s);
            }
        } else {
            // Shift characters to the right
            let mut i = self.len;
            while i > self.cursor {
                self.buf[i] = self.buf[i - 1];
                i -= 1;
            }
            self.buf[self.cursor] = b;
            self.len += 1;
            self.cursor += 1;

            // Redraw tail from current position
            if let Ok(s) = core::str::from_utf8(&self.buf[self.cursor - 1..self.len]) {
                print_fn(s);
            }
            // Move cursor back to proper position
            let shift = self.len - self.cursor;
            self.move_cursor_back(shift, print_fn);
        }
    }

    pub fn handle_backspace<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor == 0 {
            return;
        }

        self.cursor -= 1;
        // Shift left
        let mut i = self.cursor;
        while i + 1 < self.len {
            self.buf[i] = self.buf[i + 1];
            i += 1;
        }
        self.len -= 1;

        // Move cursor back 1
        print_fn("\x1b[D");
        // Redraw tail + trailing space to erase last char
        if let Ok(s) = core::str::from_utf8(&self.buf[self.cursor..self.len]) {
            print_fn(s);
        }
        print_fn(" ");
        // Move cursor back to insertion point
        let shift = self.len - self.cursor + 1;
        self.move_cursor_back(shift, print_fn);
    }

    pub fn handle_delete<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor >= self.len {
            return;
        }

        let mut i = self.cursor;
        while i + 1 < self.len {
            self.buf[i] = self.buf[i + 1];
            i += 1;
        }
        self.len -= 1;

        if let Ok(s) = core::str::from_utf8(&self.buf[self.cursor..self.len]) {
            print_fn(s);
        }
        print_fn(" ");
        let shift = self.len - self.cursor + 1;
        self.move_cursor_back(shift, print_fn);
    }

    pub fn move_left<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor > 0 {
            self.cursor -= 1;
            print_fn("\x1b[D");
        }
    }

    pub fn move_right<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor < self.len {
            self.cursor += 1;
            print_fn("\x1b[C");
        }
    }

    pub fn handle_home<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor > 0 {
            self.move_cursor_back(self.cursor, print_fn);
            self.cursor = 0;
        }
    }

    pub fn handle_end<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor < self.len {
            let shift = self.len - self.cursor;
            self.move_cursor_forward(shift, print_fn);
            self.cursor = self.len;
        }
    }

    pub fn handle_kill_to_end<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor < self.len {
            self.len = self.cursor;
            print_fn("\x1b[K");
        }
    }

    pub fn handle_kill_to_start<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor == 0 {
            return;
        }
        let remaining = self.len - self.cursor;
        let mut i = 0;
        while i < remaining {
            self.buf[i] = self.buf[self.cursor + i];
            i += 1;
        }
        self.move_cursor_back(self.cursor, print_fn);
        self.len = remaining;
        self.cursor = 0;

        if let Ok(s) = core::str::from_utf8(&self.buf[..self.len]) {
            print_fn(s);
        }
        print_fn("\x1b[K");
        if self.len > 0 {
            self.move_cursor_back(self.len, print_fn);
        }
    }

    pub fn handle_erase_word<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.cursor == 0 {
            return;
        }

        let mut new_cursor = self.cursor;
        // Skip trailing spaces
        while new_cursor > 0 && self.buf[new_cursor - 1] == b' ' {
            new_cursor -= 1;
        }
        // Skip word characters
        while new_cursor > 0 && self.buf[new_cursor - 1] != b' ' {
            new_cursor -= 1;
        }

        let erase_count = self.cursor - new_cursor;
        let mut i = new_cursor;
        while i + erase_count < self.len {
            self.buf[i] = self.buf[i + erase_count];
            i += 1;
        }
        self.len -= erase_count;
        self.move_cursor_back(erase_count, print_fn);
        self.cursor = new_cursor;

        if let Ok(s) = core::str::from_utf8(&self.buf[self.cursor..self.len]) {
            print_fn(s);
        }
        // Print spaces to clear old trailing characters
        let mut s_spaces = 0;
        while s_spaces < erase_count {
            print_fn(" ");
            s_spaces += 1;
        }
        self.move_cursor_back(self.len - self.cursor + erase_count, print_fn);
    }

    pub fn history_up<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.history_count == 0 {
            return;
        }

        if self.history_idx == self.history_count {
            // Save current line
            self.saved_buf[..self.len].copy_from_slice(&self.buf[..self.len]);
            self.saved_len = self.len;
        }

        let min_idx = if self.history_count > MAX_HISTORY {
            self.history_count - MAX_HISTORY
        } else {
            0
        };

        if self.history_idx > min_idx {
            self.history_idx -= 1;
            let slot = self.history_idx % MAX_HISTORY;
            let hist_len = self.history_lens[slot];

            // Clear current line on screen
            self.handle_home(print_fn);
            if let Ok(s) = core::str::from_utf8(&self.history[slot][..hist_len]) {
                print_fn(s);
            }
            // Clear any trailing characters if previous line was longer
            if self.len > hist_len {
                let diff = self.len - hist_len;
                let mut d = 0;
                while d < diff {
                    print_fn(" ");
                    d += 1;
                }
                self.move_cursor_back(diff, print_fn);
            }

            self.buf[..hist_len].copy_from_slice(&self.history[slot][..hist_len]);
            self.len = hist_len;
            self.cursor = hist_len;
        }
    }

    pub fn history_down<F: FnMut(&str)>(&mut self, print_fn: &mut F) {
        if self.history_idx >= self.history_count {
            return;
        }

        self.history_idx += 1;

        if self.history_idx == self.history_count {
            // Restore saved buffer
            self.handle_home(print_fn);
            if let Ok(s) = core::str::from_utf8(&self.saved_buf[..self.saved_len]) {
                print_fn(s);
            }
            if self.len > self.saved_len {
                let diff = self.len - self.saved_len;
                let mut d = 0;
                while d < diff {
                    print_fn(" ");
                    d += 1;
                }
                self.move_cursor_back(diff, print_fn);
            }
            self.buf[..self.saved_len].copy_from_slice(&self.saved_buf[..self.saved_len]);
            self.len = self.saved_len;
            self.cursor = self.saved_len;
        } else {
            let slot = self.history_idx % MAX_HISTORY;
            let hist_len = self.history_lens[slot];

            self.handle_home(print_fn);
            if let Ok(s) = core::str::from_utf8(&self.history[slot][..hist_len]) {
                print_fn(s);
            }
            if self.len > hist_len {
                let diff = self.len - hist_len;
                let mut d = 0;
                while d < diff {
                    print_fn(" ");
                    d += 1;
                }
                self.move_cursor_back(diff, print_fn);
            }
            self.buf[..hist_len].copy_from_slice(&self.history[slot][..hist_len]);
            self.len = hist_len;
            self.cursor = hist_len;
        }
    }

    pub fn handle_tab<F: FnMut(&str)>(
        &mut self,
        prompt_fn: &dyn Fn(),
        print_fn: &mut F,
        completer_fn: &dyn Fn(&str, &mut [([u8; 64], usize); 16]) -> usize,
    ) {
        let current_text = match core::str::from_utf8(&self.buf[..self.cursor]) {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut matches: [([u8; 64], usize); 16] = [([0; 64], 0); 16];
        let count = completer_fn(current_text, &mut matches);

        if count == 1 {
            let (ref match_bytes, match_len) = matches[0];
            // Insert matched suffix into line buffer
            let word_start = current_text.rfind(' ').map(|p| p + 1).unwrap_or(0);
            let typed_len = self.cursor - word_start;
            if match_len > typed_len {
                let suffix = &match_bytes[typed_len..match_len];
                for &b in suffix {
                    self.insert_char(b, print_fn);
                }
                self.insert_char(b' ', print_fn);
            }
        } else if count > 1 {
            print_fn("\n");
            for i in 0..count {
                let (ref match_bytes, match_len) = matches[i];
                if let Ok(s) = core::str::from_utf8(&match_bytes[..match_len]) {
                    print_fn(s);
                    print_fn("  ");
                }
            }
            print_fn("\n");
            prompt_fn();
            if let Ok(s) = core::str::from_utf8(&self.buf[..self.len]) {
                print_fn(s);
            }
            if self.cursor < self.len {
                self.move_cursor_back(self.len - self.cursor, print_fn);
            }
        }
    }

    fn move_cursor_back<F: FnMut(&str)>(&self, count: usize, print_fn: &mut F) {
        let mut i = 0;
        while i < count {
            print_fn("\x1b[D");
            i += 1;
        }
    }

    fn move_cursor_forward<F: FnMut(&str)>(&self, count: usize, print_fn: &mut F) {
        let mut i = 0;
        while i < count {
            print_fn("\x1b[C");
            i += 1;
        }
    }
}
