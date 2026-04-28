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
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    agents: Vec<AgentConfig>,
}

#[derive(Deserialize)]
struct AgentConfig {
    name: String,
    in_file: String,
    out_file: String,
    #[serde(skip)] // This field isn't in the JSON; we'll initialize it ourselves
    last_content: String,
}

struct App {
    input: String,
    messages: Vec<String>,
    agents: Vec<AgentConfig>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the "Council" from JSON
    let config_data = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&config_data)?;

    // Setup the App state with the agents
    let mut app = App {
        input: String::new(),
        messages: Vec::new(),
        agents: config.agents,
    };
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

                                // Log your message locally
                                app.messages.push(format!("Facilitator > {}", app.input));

                                // BROADCAST: Write to every agent's input file
                                for agent in &app.agents {
                                    if let Ok(mut file) = OpenOptions::new().append(true).open(&agent.in_file) {
                                        let _ = writeln!(file, "{}", app.input);
                                    }
                                }

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
                for i in 0..app.agents.len() {
                    let agent_name = app.agents[i].name.clone();
                    let out_file = app.agents[i].out_file.clone();
                    let last_len = app.agents[i].last_content.len();

                    if let Ok(contents) = fs::read_to_string(&out_file) {
                        if contents.len() > last_len {
                            let diff = contents[last_len..].trim().to_string();
                            if !diff.is_empty() {
                                let formatted_msg = format!("{} > {}", agent_name, diff);
                                app.messages.push(formatted_msg.clone());

                                // THE "HEARING" LOGIC: Broadcast this agent's message to everyone else
                                for j in 0..app.agents.len() {
                                    if i == j { continue; } // Don't send an agent's message back to itself
                                    if let Ok(mut file) = OpenOptions::new().append(true).open(&app.agents[j].in_file) {
                                        let _ = writeln!(file, "{}", formatted_msg);
                                    }
                                }
                            }
                            app.agents[i].last_content = contents;
                        }
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

    // We subtract 2 to account for the top and bottom borders
    let area_height = chunks[1].height.saturating_sub(2) as usize;
    
    // 2. Count the lines. Note: If you use .wrap(), long lines count as multiple.
    // For simplicity, we'll start by counting the number of messages
    let message_count = app.messages.len();

    // 3. Calculate scroll: if we have more messages than height, scroll the difference
    let scroll_y = if message_count > area_height {
        (message_count - area_height) as u16
    } else {
        0
    };

    let messages = Paragraph::new(message_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Output ")
        )
        .wrap(ratatui::widgets::Wrap { trim: true })
        .scroll((scroll_y, 0)); // Apply the vertical scroll

    f.render_widget(messages, chunks[1]);

    // --- Input Box ---
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Input "));
    f.render_widget(input, chunks[2]);
}