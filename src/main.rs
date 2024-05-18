use std:: io::{stdout, Write};

use anyhow::Ok;
use crossterm::{cursor, event::{self}, style::{self, Color, Stylize}, terminal, ExecutableCommand, QueueableCommand};

#[derive(Debug)]
enum Mode {
    Normal,
    Insert,
}

struct Editor {
    mode: Mode,

    out: std::io::Stdout,
    rows: usize,
    cols: usize,

    row_off: usize,
    col_off: usize,

    current_row: usize,
    current_col: usize,

    buffer: Buffer,
}

impl Editor {
    fn new() -> anyhow::Result<Editor> {
        let stdout = stdout();
        let size = terminal::size()?;
        let mut buffer = Buffer::new()?;

        buffer.read_file(Some(String::from("src/main.rs")))?;

        Ok(Editor {
            mode: Mode::Normal,
            out: stdout,
            cols: (size.0) as usize,
            rows: (size.1 - 1) as usize,
            row_off: 0,
            col_off: 0,
            current_row: 0,
            current_col: 0,
            buffer,
        })
    }

    fn run(&mut self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        self.out.execute(terminal::EnterAlternateScreen)?;

        loop {
            self.out.queue(cursor::Hide)?;
            self.out.queue(terminal::Clear(terminal::ClearType::All))?;
            self.scroll()?;
            self.print()?;
            self.print_statusbar()?;
            self.move_caret()?;
            self.out.queue(cursor::Show)?;
            self.out.flush()?;
            if self.handle_event()? {
                break;
            }
        }

        Ok(())
    }

    fn print(&mut self) -> anyhow::Result<()>{
        for row in 0..self.rows {
            let row_buf = row + self.row_off;
            let mut chars = String::new();            
            for col in 0..self.cols {
                let col_buf = col + self.col_off;
                if row_buf < self.buffer.height() && col_buf < self.buffer.line_width(row_buf) {
                    chars.push(self.buffer.get(row_buf, col_buf));
                } else if row_buf >= self.buffer.height() && row_buf < self.rows {
                    self.out.queue(style::Print("~\n"))?;
                }
            }
            self.out.queue(cursor::MoveTo(0, row as u16))?;
            self.out.queue(style::Print(chars))?;
        }
        Ok(())
    }

    fn print_statusbar(&mut self) -> anyhow::Result<()> {
        self.out.queue(cursor::MoveTo(0, self.rows as u16))?;

        let mode: String = format!("{:?}", self.mode).to_uppercase();

        let bg_color = match self.mode {
            Mode::Normal => Color::Rgb { r: 184, g: 144, b: 243 },
            Mode::Insert => Color::Rgb { r: 194, g: 255, b: 102 },
        };

        self.out.queue(style::PrintStyledContent(format!(" {} ", mode)
            .with(Color::Rgb { 
                r: 0, 
                g: 0, 
                b: 0, 
            }).bold().on(bg_color))
        )?;

        // TODO change to opened file
        let file = self.buffer.file.as_deref().unwrap_or("no file");

        let pos = format!(" {}:{} ", self.current_col, self.current_row);
        let file_width = self.cols - mode.len() - 2 - pos.len() - 1;
        self.out.queue(style::PrintStyledContent(format!(" {:<width$}", file, width = file_width as usize)
            .with(Color::Rgb { 
                r: 255, 
                g: 255, 
                b: 255, 
            }).bold().on(Color::Rgb { 
                r: 67, 
                g: 70, 
                b: 89, 
            })))?;

        self.out.queue(style::PrintStyledContent(
            pos.with(Color::Rgb { 
                r: 0, 
                g: 0, 
                b: 0, 
            }).bold().on(bg_color))
        )?;

        Ok(())
    }

    fn move_caret(&mut self) -> anyhow::Result<()> {
        let col = (self.current_col - self.col_off) as u16;
        let row = (self.current_row - self.row_off) as u16;
        self.out.queue(cursor::MoveTo(col, row))?;
        Ok(())
    }

    fn scroll(&mut self) -> anyhow::Result<()> {
        let size = terminal::size()?;
        self.cols = size.0 as usize;
        self.rows = (size.1 - 1) as usize;
        
        if self.current_row < self.row_off {
            self.row_off = self.current_row;
        }
        if self.current_col < self.col_off {
            self.col_off = self.current_col;
        }
        if self.current_row >= self.row_off + self.rows{
            self.row_off = self.current_row - self.rows + 1;
        }
        if self.current_col >= self.col_off + self.cols {
            self.col_off = self.current_col - self.cols + 1;
        }
        Ok(())
    }

    fn handle_normal_event(&mut self) -> anyhow::Result<bool> {
        match event::read()? {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Up => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    if self.current_row != 0 {
                        self.current_row -= 1;
                        if self.current_col > self.buffer.line_width(self.current_row) {
                            self.current_col = self.buffer.line_width(self.current_row);
                        }
                    }
                    Ok(false)
                },
                event::KeyCode::Down => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    if self.current_row != self.buffer.height() - 1 {
                        self.current_row += 1;
                        if self.current_col > self.buffer.line_width(self.current_row) {
                            self.current_col = self.buffer.line_width(self.current_row);
                        }
                    }
                    Ok(false)
                },
                event::KeyCode::Left => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    if self.current_col != 0 {
                        self.current_col -= 1;
                    } else if self.current_row != 0 {
                        self.current_row -= 1;
                        self.current_col = self.buffer.line_width(self.current_row);
                    }
                    Ok(false)
                },
                event::KeyCode::Right => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    if self.current_col != self.buffer.line_width(self.current_row) {
                        self.current_col += 1;
                    } else if self.current_row < self.buffer.height() - 1 {
                        self.current_row += 1;
                        self.current_col = 0;
                    }
                    Ok(false)
                },
                event::KeyCode::Char('q') => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    Ok(true)
                },
                event::KeyCode::Char('s') => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    self.buffer.write_file()?;
                    Ok(false)
                },
                event::KeyCode::Char('i') => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    self.mode = Mode::Insert;
                    Ok(false)
                }
                _ => Ok(false)
            }
            _ => Ok(false)
        }
    }

    fn handle_event(&mut self) -> anyhow::Result<bool> {
        match self.mode {
            Mode::Normal => self.handle_normal_event(),
            Mode::Insert => self.handle_insert_event(),
        }
    }

    fn handle_insert_event(&mut self) -> anyhow::Result<bool> {
        match event::read()? {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    self.mode = Mode::Normal;
                    Ok(false)
                }
                event::KeyCode::Char(c) => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    self.buffer.insert(c, self.current_row, self.current_col);
                    self.current_col += 1;
                    Ok(false)
                },
                event::KeyCode::Enter => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    self.buffer.insert_line(self.current_row, self.current_col);
                    self.current_row += 1;
                    self.current_col = 0;
                    Ok(false)
                },
                event::KeyCode::Backspace => {
                    if event.kind != event::KeyEventKind::Press {
                        return Ok(false)
                    }
                    let col = self.buffer.line_width(self.current_row.saturating_sub(1));
                    self.buffer.delete(self.current_row, self.current_col);

                    if self.current_col > 0 {
                        self.current_col = self.current_col.saturating_sub(1);
                    } else if self.current_row > 0 {
                        self.current_row = self.current_row.saturating_sub(1);
                        self.current_col = col;
                    }
                    Ok(false)
                }
                _ => Ok(false),
            }
            _ => Ok(false),
        } 
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.out.flush().unwrap();
        self.out.execute(terminal::LeaveAlternateScreen).expect("unsupported terminal");
        terminal::disable_raw_mode().expect("unsupported terminal");
    }
}

struct Buffer {
    file: Option<String>,
    buffer: Vec<Vec<char>>
}

impl Buffer {
    fn new() -> anyhow::Result<Buffer> {
        Ok(
            Buffer {
                file: None,
                buffer: vec![vec![]]
            }
        )
    }

    fn read_file(&mut self, filename: Option<String>) -> anyhow::Result<()> {
        let lines: Vec<Vec<char>> = match &filename {
            Some(file) => std::fs::read_to_string(file)
                .unwrap_or(String::from(""))
                .lines()
                .map(|s| s.chars().collect()).collect()
            ,
            None => vec![vec![]]
        };
        self.file = filename;
        self.buffer = lines;
        Ok(())
    }

    fn write_file(&mut self) -> anyhow::Result<()> {
        let lines: Vec<String>  = self.buffer.iter().map(|s| s.iter().collect()).collect();
        let filename = self.file.clone().unwrap();
        std::fs::write(filename, lines.join("\n"))?;
        Ok(())
    }

    fn height(&self) -> usize {
        self.buffer.len()
    }

    fn line_width(&self, row: usize) -> usize {
        if row < self.height() {
            return self.buffer[row].len()
        }
        0
    }

    fn get(&self, row: usize, col: usize) -> char {
        if row < self.height() {
            if col < self.line_width(row) {
                return self.buffer[row][col]
            }
        }
        ' '
    }

    fn insert(&mut self, c: char, row: usize, col: usize) {
        let line = self.buffer.get_mut(row).unwrap();
        line.insert(col, c);
    }
    
    fn insert_line(&mut self, row: usize, col: usize) {
        let right_line: Vec<char> = self.buffer[row].drain(col..).collect();
        let left_line: Vec<char> = self.buffer[row].drain(..col).collect();
        self.buffer[row] = left_line;
        self.buffer.insert(row + 1, right_line);
    }

    fn delete(&mut self, row: usize, col: usize) {
        if col > 0 {
            let line = self.buffer.get_mut(row).unwrap();
            line.remove(col - 1);
        } else if row > 0 {
            let line = self.buffer[row].clone();
            for c in line.iter() {
                self.buffer.get_mut(row - 1).unwrap().push(*c);
            }
            self.buffer.remove(row);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut editor = Editor::new()?;
    editor.run()?;
    Ok(())
}