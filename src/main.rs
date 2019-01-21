use tui::Terminal;
use tui::widgets::Widget;
use tui::backend::TermionBackend;
use termion::raw::IntoRawMode;
use termion::event::Key;
use clap::{App, Arg};

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

fn is_int(v: String) -> Result<(), String> {
    v.parse::<u64>()
        .map(|_| ())
        .map_err(|_| String::from("Value must be an integer"))
}

fn main() -> Result<(), Error> {

    let matches = App::new("packetloss")
        .version("0.1")
        .author("Spencer Powell")
        .about("Show a colored graph of packet loss over time")
        .arg(Arg::with_name("address")
            .help("Host to ping")
            .required(true))
        .arg(Arg::with_name("chunk-size")
            .long("chunk-size")
            .short("n")
            .help("number of pings per chunk")
            .validator(is_int)
            .default_value("10"))
        .arg(Arg::with_name("interval")
            .long("interval")
            .short("i")
            .help("interval between pings (s)")
            .validator(is_int)
            .default_value("60"))
        .arg(Arg::with_name("timeout")
            .long("timeout")
            .short("t")
            .help("ping timeout duration (ms)")
            .validator(is_int)
            .default_value("100"))
        .get_matches();

    let address = matches.value_of("address").unwrap();
    let chunk_size = matches.value_of("chunk-size").unwrap()
        .parse::<u64>().unwrap();
    let interval = matches.value_of("interval").unwrap()
        .parse::<u64>().unwrap();
    let timeout = matches.value_of("timeout").unwrap()
        .parse::<u64>().unwrap();

    let ping = Ping::new(address, Duration::from_millis(timeout));

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let mut list = LogList::default();
    let mut sleep = Sleep::new();
    let mut internal_size = terminal.size()?;

    loop {
        /* redraw, retry if the size is incorrect */
        let size = terminal.size()?;
        if size != internal_size {

            terminal.resize(size)?;
            internal_size = size;

            terminal.clear()?;

            terminal.draw(|mut f| {
                list.render(&mut f, size);
            })?;
        } else if sleep.done() {

            let chunk = ping.ping(chunk_size)?;
            list.insert(chunk);

            terminal.draw(|mut f| {
                list.render(&mut f, size);
            })?;

            sleep = Sleep::sleep(Duration::from_secs(interval));
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
