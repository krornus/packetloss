use tui::Terminal;
use tui::widgets::Widget;
use tui::backend::TermionBackend;
use termion::raw::IntoRawMode;
use termion::event::Key;

use ctrlc;

use std::io;
use std::time::{Duration};
use std::net::IpAddr;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod ping;
mod term;
mod event;

use crate::ping::Ping;
use crate::term::LogList;
use crate::event::{Event, Events};

type Term = Terminal<TermionBackend<io::Stdout>>;

fn main() -> Result<(), io::Error> {

    let addr: IpAddr = "8.8.8.8".parse().expect("failed to parse ip");
    let ping = Ping::new("8.8.8.8", Duration::from_secs(1));

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor();

    let events = Events::new();

    let mut list = LogList::default();
    let mut sleep = Sleep::new();
    let mut internal_size = terminal.size()?;

    loop {

        if sleep.done() {

            let chunk = ping.ping(5);
            list.insert(chunk);


            sleep = Sleep::sleep(Duration::from_secs(2));
        }

        let size = terminal.size()?;
        if size != internal_size {
            terminal.resize(size);
            internal_size = size;
        }

        terminal.draw(|mut f| {
            list.render(&mut f, size);
        });

        match events.next().unwrap() {
            Event::Input(input) => match input {
                Key::Char('q') | Key::Esc => {
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }

    terminal.clear();

    Ok(())
}

struct Sleep {
    ready: Arc<AtomicBool>,
}

impl Sleep {
    fn new() -> Self {
        let ready = Arc::new(AtomicBool::new(true));
        Sleep { ready }
    }

    fn sleep(time: Duration) -> Self {

        let ready = Arc::new(AtomicBool::new(false));
        let notify = ready.clone();

        thread::spawn(move || {
            thread::sleep(time);
            notify.store(true, Ordering::Relaxed);
        });

        Sleep { ready }
    }

    fn done(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }
}
