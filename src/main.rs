use std::{any, io::{stdout, Write}};

use crossterm::{cursor, event::{self, read}, style, terminal, ExecutableCommand, QueueableCommand};

struct Editor {
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
        self.out.execute(terminal::Clear(terminal::ClearType::All))?;

        loop {
            self.out.queue(terminal::Clear(terminal::ClearType::All))?;
            self.scroll()?;
            self.print()?;
            self.move_caret()?;
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
            for col in 0..self.cols {
                let col_buf = col + self.col_off;
                if row_buf < self.buffer.height() && col_buf < self.buffer.line_width(row_buf) {
                    self.out.queue(cursor::MoveTo(col as u16, row as u16))?;
                    self.out.queue(style::Print(self.buffer.get(row, col)))?;
                } else if row_buf >= self.buffer.height() && row_buf < self.rows {
                    self.out.queue(style::Print('~'))?;
                }
            }
        }
        self.out.flush()?;
        Ok(())
    }

    fn move_caret(&mut self) -> anyhow::Result<()> {
        let col = (self.current_col - self.col_off) as u16;
        let row = (self.current_row - self.row_off) as u16;
        self.out.queue(cursor::MoveTo(col, row))?;
        Ok(())
    }

    fn scroll(&mut self) -> anyhow::Result<()> {
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

    fn handle_event(&mut self) -> anyhow::Result<bool> {
        match event::read()? {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Char('q') => Ok(true),
                event::KeyCode::Down => {
                    if self.current_row != self.buffer.height() - 1 {
                        self.current_row += 1;
                    }
                    Ok(false)
                },
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

    fn write_file(&mut self, filename: Option<String>) -> anyhow::Result<()> {
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
}

fn main() -> anyhow::Result<()> {
    let mut editor = Editor::new()?;
    editor.run()?;
    Ok(())
}
