mod app;
mod cache;
mod config;
mod git;
mod github;
mod notes;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Quick capture: vex add <title>
    if args.len() >= 3 && args[1] == "add" {
        let title = &args[2];
        let mut body = None::<String>;
        let mut priority = "medium";

        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--body" | "-b" => {
                    if i + 1 < args.len() {
                        body = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--priority" | "-p" => {
                    if i + 1 < args.len() {
                        priority = &args[i + 1];
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        let project_dir = std::env::current_dir()?;
        match notes::create_note(&project_dir, title, body.as_deref(), priority, None) {
            Ok(note) => {
                println!("Created note: {}", note.title);
            }
            Err(e) => eprintln!("Error: {e}"),
        }
        return Ok(());
    }

    let config = config::load()?;
    let cache = cache::Cache::new()?;

    let mut app = App::new(config, cache).await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = app.run(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e:#}");
    }

    Ok(())
}
