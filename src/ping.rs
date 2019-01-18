use oping::{PingItem, PingError};

use std::time::{Duration, SystemTime};
use std::fmt;
use std::borrow::Cow;
use rand::prelude::*;

use tui::buffer::Buffer;
use tui::widgets::{Widget, Text};
use tui::layout::Rect;
use tui::style::Color;

pub struct Ping {
    addr: String,
    timeout: f64,
}

impl Ping {
    pub fn new(addr: &str, timeout: Duration) -> Self {
        let mut ping = oping::Ping::new();

        let ms = timeout.subsec_millis();
        let timeout = timeout.as_secs() as f64 + (ms as f64 / 1000_f64);

        Ping {
            addr: addr.to_string(),
            timeout: timeout,
        }
    }

    pub fn addr(&mut self, addr: String) {
        self.addr = addr
    }

    pub fn timeout(&mut self, timeout: Duration) {
        let ms = timeout.subsec_millis();
        self.timeout = timeout.as_secs() as f64 + (ms as f64 / 1000_f64);
    }

    pub fn ping(&self, count: u64) -> PacketChunk {
        let mut chunk = PacketChunk::new();

        for _ in 0..count {
            let mut ping = oping::Ping::new();
            ping.set_timeout(self.timeout);
            ping.add_host(self.addr.as_str());

            let item = ping.send().expect("Failed to send ping").next().unwrap();
            chunk.packets.push(item);
        }

        chunk
    }
}

#[derive(Clone)]
pub struct PacketChunk {
    packets: Vec<PingItem>,
    time: SystemTime,
    loss: f64,
}

impl PacketChunk {
    pub fn new() -> Self {
        PacketChunk {
            packets: vec![],
            time: SystemTime::now(),
            loss: rand::random(),
        }
    }

    pub fn sent(&self) -> usize {
        self.packets.len()
    }

    pub fn received(&self) -> usize {
        self.packets.iter().filter(|x| x.dropped == 0).collect::<Vec<_>>().len()
    }

    pub fn loss(&self) -> f64 {

        let sent = self.sent();
        if sent == 0 {
            0_f64
        } else {
            1f64 - (self.received() as f64 / self.sent() as f64)
        }
    }

    pub fn latency(&self) -> f64 {
        self.packets.iter().fold(0f64, |acc, item| acc + item.latency_ms)
    }

    pub fn color(&self) -> (u8, u8, u8) {

        let mix = self.loss;

        let red: (f64, f64, f64) = (200.0, 0.0, 30.0);
        let green: (f64, f64, f64) = (0.0, 200.0, 30.0);

        let r = ((green.0)*(1f64-mix) + (red.0)*(mix)) as u8;
        let g = ((green.1)*(1f64-mix) + (red.1)*(mix)) as u8;
        let b = ((green.2)*(1f64-mix) + (red.2)*(mix)) as u8;

        (r,g,b)
    }
}

impl fmt::Display for PacketChunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} packets transmitted, {} received, {}% packet loss, time {:.01}ms",
              self.sent(), self.received(), (self.loss()*100f64) as u32, self.latency())
    }
}

impl Widget for PacketChunk {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let (r,g,b) = self.color();
        let color = Color::Rgb(r,g,b);

        if area.width == 0 || area.height == 0 {
            return;
        }


        self.background(&area, buf, color);

        let info = self.to_string();
        if area.width < info.len() as u16 {
            return;
        }

        let x = area.x + (area.width / 2).saturating_sub(info.len() as u16 / 2);
        let y = area.y + (area.height / 2);

        let style = tui::style::Style::default()
            .bg(color);

        buf.set_stringn(x, y, info, area.width as usize, style);
    }
}

