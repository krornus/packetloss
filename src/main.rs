use tui::Terminal;
use tui::widgets::Widget;
use tui::backend::TermionBackend;
use termion::raw::IntoRawMode;
use termion::event::Key;

use std::io;
use std::time::{Duration};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::fmt;

mod ping;
mod term;
mod event;

use crate::ping::Ping;
use crate::term::LogList;
use crate::event::{Event, Events};

#[derive(Debug)]
enum Error {
    IO(io::Error),
    Ping(oping::PingError),
    Event(std::sync::mpsc::RecvError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IO(e) => write!(f, "IO Error: {}", e),
            Error::Ping(e) => write!(f, "Ping error: {}", e),
            Error::Event(e) => write!(f, "Event error: {}", e),
        }
    }
}


impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IO(e)
    }
}

impl From<oping::PingError> for Error {
    fn from(e: oping::PingError) -> Error {
        Error::Ping(e)
    }
}

impl From<std::sync::mpsc::RecvError> for Error {
    fn from(e: std::sync::mpsc::RecvError) -> Error {
        Error::Event(e)
    }
}

fn main() -> Result<(), Error> {

    let ping = Ping::new("8.8.8.8", Duration::from_millis(500));

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let mut list = LogList::default();
    let mut sleep = Sleep::new();
    let mut internal_size = terminal.size()?;

    loop {

        if sleep.done() {

            let chunk = ping.ping(100)?;
            list.insert(chunk);

            let size = terminal.size()?;
            if size != internal_size {
                terminal.resize(size)?;
                internal_size = size;
                terminal.clear()?;
            }
            terminal.draw(|mut f| {
                list.render(&mut f, size);
            })?;

            sleep = Sleep::sleep(Duration::from_millis(50));
        }


        match events.next()? {
            Event::Input(input) => match input {
                Key::Char('q') | Key::Esc => {
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }

    terminal.clear()?;

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
