use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use ratatui::{
    Frame, Terminal, backend::CrosstermBackend, layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph}
};
use std::{fs::{self, OpenOptions}, io::{self, Write}, time::Duration};
use tokio::time::sleep;

struct App {
    input: String,
    messages: Vec<String>,
    last_content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        input: String::new(),
        messages: Vec::new(),
        last_content: String::new(),
    };

    let out_file = "/tmp/council/minimax.out";
    let in_file = "/tmp/council/minimax.in";

    // Event stream for keyboard
    let mut reader = event::EventStream::new();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            // Handle Keyboard
            maybe_event = reader.next().fuse() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => {
                                if app.input == "/exit" { break; }

                                // Manually add your message to the UI history
                                app.messages.push(format!("you > {}", app.input));

                                // Write to the file as usual
                                let mut file = OpenOptions::new()
                                    .create(true).write(true).append(true)
                                    .open(in_file)?;
                                writeln!(file, "{}", app.input)?;

                                // Clear the input box
                                app.input.clear();
                            }
                            KeyCode::Char(c) => app.input.push(c),
                            KeyCode::Backspace => { app.input.pop(); }
                            KeyCode::Esc => break,
                            _ => {}
                        }
                    }
                }
            }
            // Handle File Watching
            _ = sleep(Duration::from_millis(100)) => {
                if let Ok(contents) = fs::read_to_string(out_file) {
                    if contents != app.last_content && !contents.trim().is_empty() {
                        let diff = contents[app.last_content.len()..].trim().to_string();
                        if !diff.is_empty() {
                            app.messages.push(format!("minimax > {}", diff));
                        }
                        app.last_content = contents;
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), 
            Constraint::Min(3),    
            Constraint::Length(3), 
        ])
        .split(f.size());

    // Define colors
    let dark_purple = Color::Rgb(75, 0, 130);
    let light_purple = Color::Rgb(186, 85, 211);
    let light_gray = Color::Rgb(169, 169, 169);
    let dark_gray = Color::Rgb(64, 64, 64);
    let ancient_red = Color::Rgb(200, 34, 34);

    // Build the stylized header content
    let header_lines = vec![
        Line::from(vec![Span::styled("================================================================================", Style::default().fg(dark_gray))]),
        Line::from(vec![
            Span::styled("         ἓν οἶδα     ", Style::default().fg(ancient_red)),
            Span::styled("· · ·    ", Style::default().fg(light_gray)),
            Span::styled("T H E    C O U N C I L", Style::default().fg(light_purple).add_modifier(Modifier::BOLD)),
            Span::styled("   · · ·    ", Style::default().fg(light_gray)),
            Span::styled("cogito  ", Style::default().fg(ancient_red)),
        ]),
        Line::from(vec![
            Span::styled("         ὅτι οὐδὲν   ", Style::default().fg(ancient_red)),
            Span::styled("· ·       ", Style::default().fg(light_gray)),
            Span::styled("ancient deliberation", Style::default().fg(dark_purple).add_modifier(Modifier::ITALIC)),
            Span::styled("      · ·     ", Style::default().fg(light_gray)),
            Span::styled("ergo   ", Style::default().fg(ancient_red)),
        ]),
        Line::from(vec![
            Span::styled("         οἶδα        ", Style::default().fg(ancient_red)),
            Span::styled("·                                     ·      ", Style::default().fg(light_gray)),
            Span::styled("sum    ", Style::default().fg(ancient_red)),
        ]),
        Line::from(vec![Span::styled("================================================================================", Style::default().fg(dark_gray))]),
    ];

    let header = Paragraph::new(header_lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(header, chunks[0]);

    // --- Output History ---
    // Combine all messages into a single string with newlines
    let message_text = app.messages.join("\n");

    let messages = Paragraph::new(message_text) // Now it's a String, which Paragraph accepts
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Output ")
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(messages, chunks[1]);

    // --- Input Box ---
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Input "));
    f.render_widget(input, chunks[2]);
}