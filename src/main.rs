use color_eyre::Result;
use uuid::Uuid;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    const fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            character_index: 0,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        let trimmed = self.input.trim();
        let input_type = Self::input_type_validation(trimmed);
        let result: String;

        if input_type == "Guid" {
            match Uuid::parse_str(trimmed) {
                Ok(uuid) => {
                    let raw_hex = Self::guid_to_raw_hex(&uuid);
                    result = format!("✅ Raw Hex: {}", raw_hex);
                }
                Err(_) => {
                    result = "❌ Invalid GUID".to_string();
                }
            }
        } else if input_type == "RawHex" {
            match Self::raw_hex_to_guid(trimmed) {
                Some(guid) => {
                    result = format!("✅ GUID: {}", guid);
                }
                None => {
                    result = "❌ Invalid Raw Hex.".to_string();
                }
            }
        } else {
            result = "❌ Invalid input. It's neither raw hex nor guid.".to_string();
        }


        self.messages.push(result);
        self.input.clear();
        self.reset_cursor();
    }

    fn raw_hex_to_guid(hex: &str) -> Option<Uuid> {
        if hex.len() != 32 {
            return None;
        }        
    
        let raw_bytes = (0..16)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
            .collect::<Option<Vec<u8>>>()?;
    
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&raw_bytes);
    
        let mut reordered = [0u8; 16];
        reordered[0..4].copy_from_slice(&bytes[0..4]);
        reordered[0..4].reverse();
        reordered[4..6].copy_from_slice(&bytes[4..6]);
        reordered[4..6].reverse();
        reordered[6..8].copy_from_slice(&bytes[6..8]);
        reordered[6..8].reverse();
        reordered[8..16].copy_from_slice(&bytes[8..16]);
    
        Some(Uuid::from_bytes(reordered))
    }
    
    fn guid_to_raw_hex(uuid: &Uuid) -> String {
        let bytes = uuid.as_bytes();
    
        let mut reordered = [0u8; 16];
        reordered[0..4].copy_from_slice(&bytes[0..4]);
        reordered[0..4].reverse();
        reordered[4..6].copy_from_slice(&bytes[4..6]);
        reordered[4..6].reverse();
        reordered[6..8].copy_from_slice(&bytes[6..8]);
        reordered[6..8].reverse();
        reordered[8..16].copy_from_slice(&bytes[8..16]);
    
        reordered.iter().map(|b| format!("{:02X}", b)).collect()
    }

    fn input_type_validation(input: &str) -> String {
        let trimmed_string = input.trim();
        if trimmed_string.len() == 36 && trimmed_string.chars().filter(|&c| c == '-').count() == 4 {
            return "Guid".to_string();
        } else if trimmed_string.len() == 32 && trimmed_string.chars().all(|c| c.is_ascii_hexdigit()) {
            return "RawHex".to_string();
        } else {
            return "None".to_string();
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => self.submit_message(),
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);
        let [additional_area, help_area, input_area, messages_area] = vertical.areas(frame.area());

        let (add_msg, msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Welcome to GUID Converter".bold().into()
                ],
                vec![
                    "Press ".into(),
                    "[q]".bold().fg(Color::Cyan),
                    " to exit, ".into(),
                    "[e]".bold().fg(Color::Cyan),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "[Esc]".bold().fg(Color::Cyan),
                    " to stop editing, ".into(),
                    "[Enter]".bold().fg(Color::Cyan),
                    " to start the process".into(),
                ],
                vec![
                    "You are in editing mode. Press ".into(),
                    "[Ctrl]".bold().fg(Color::Cyan),
                    " + ".into(),
                    "[Shift]".bold().fg(Color::Cyan),
                    " + ".into(),
                    "[V]".bold().fg(Color::Cyan),
                    " to paste Guid / Raw Hex into the".into(),
                    " input box".bold().fg(Color::Yellow)
                ],
                Style::default(),
            ),
        };

        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);

        let additional_text = Text::from(Line::from(add_msg)).patch_style(style);
        let additional_message = Paragraph::new(additional_text);

        frame.render_widget(additional_message, additional_area);
        frame.render_widget(help_message, help_area);
        

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input Guid / Raw Hex"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            InputMode::Normal => {}

            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            #[allow(clippy::cast_possible_truncation)]
            InputMode::Editing => frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                input_area.x + self.character_index as u16 + 1,
                // Move one line down, from the border to the input line
                input_area.y + 1,
            )),
        }

        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(_i, m)| {
                let content = Line::from(Span::raw(format!("{m}")));
                ListItem::new(content)
            })
            .collect();
        let messages = List::new(messages).block(Block::bordered().title("Result"));
        frame.render_widget(messages, messages_area);
    }
}