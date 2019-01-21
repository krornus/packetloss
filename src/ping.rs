use oping::{PingItem, PingError};

use std::time::{Duration, SystemTime};

use tui::buffer::Buffer;
use tui::widgets::Widget;
use tui::layout::Rect;
use tui::style::Color;

pub struct Ping {
    addr: String,
    timeout: Duration,
}

impl Ping {
    pub fn new(addr: &str, timeout: Duration) -> Self {

        Ping {
            addr: addr.to_string(),
            timeout: timeout,
        }
    }

    pub fn ping(&self, count: u64) -> Result<PacketChunk, PingError> {
        let mut chunk = PacketChunk::new((self.timeout.as_secs() * 1000 + self.timeout.subsec_millis() as u64) as f64);

        chunk.packets = (0..count)
            .map(|_| self.do_ping().ok())
            .collect();

        Ok(chunk)
    }

    fn do_ping(&self) -> Result<PingItem, PingError> {
        let mut ping = oping::Ping::new();

        let ms = self.timeout.subsec_millis();
        let timeout = self.timeout.as_secs() as f64 + (ms as f64 / 1000_f64);

        ping.set_timeout(timeout)?;
        ping.add_host(self.addr.as_str())?;

        Ok(ping.send()?.next().unwrap())
    }
}

#[derive(Clone)]
pub struct PacketChunk {
    packets: Vec<Option<PingItem>>,
    time: SystemTime,
    timeout: f64,
}

impl PacketChunk {
    pub fn new(timeout: f64) -> Self {
        PacketChunk {
            packets: vec![],
            time: SystemTime::now(),
            timeout: timeout,
        }
    }

    pub fn sent(&self) -> usize {
        self.packets.len()
    }

    pub fn received(&self) -> usize {
        self.packets.iter()
            .filter(|x| x.is_some())
            .filter(|x| x.as_ref().unwrap().dropped == 0)
            .collect::<Vec<_>>().len()
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

        let mut acc = 0.0;
        for packet in self.packets.iter() {
            acc += match packet {
                Some(ref packet) => {
                    if packet.dropped != 0 {
                        self.timeout
                    } else {
                        packet.latency_ms
                    }
                },
                None => {
                    self.timeout
                }
            };
        }

        acc
    }

    pub fn color(&self, min: f64) -> (u8, u8, u8) {

        let loss = self.loss();
        let mut lat = min / self.latency();

        if lat > 1.0 {
            lat = 1.0;
        }

        /* 100% = green */
        let mix = (1.0 - loss)*lat;

        let red: (f64, f64, f64) = (224.0, 15.0, 71.0);
        let green: (f64, f64, f64) = (14.0, 204.0, 80.0);

        let r = ((green.0)*(mix) + (red.0)*(1.0-mix)) as u8;
        let g = ((green.1)*(mix) + (red.1)*(1.0-mix)) as u8;
        let b = ((green.2)*(mix) + (red.2)*(1.0-mix)) as u8;

        (r,g,b)
    }
}

/* seperate struct for drawing - need min response time dynamically */
pub struct DrawablePacket<'a> {
    packet: &'a PacketChunk,
    min_latency: f64,
}

impl<'a> DrawablePacket<'a> {
    pub fn new(packet: &'a PacketChunk, min: f64) -> Self {
        DrawablePacket {
            packet: packet,
            min_latency: min,
        }
    }
}

impl<'a> Widget for DrawablePacket<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let (r,g,b) = self.packet.color(self.min_latency);
        let color = Color::Rgb(r,g,b);

        if area.width == 0 || area.height == 0 {
            return;
        }

        self.background(&area, buf, color);

        let pct = (self.packet.loss()*100f64) as u32;

        let long = format!(" {} packets transmitted, {} received, {}% packet loss, time {:.01}ms ",
              self.packet.sent(), self.packet.received(), pct, self.packet.latency());
        let short = format!(" {}% [{:.0}ms] ", pct, self.packet.latency());

        let info = if area.width >= long.len() as u16 {
            long
        } else if area.width >= short.len() as u16 {
            short
        } else {
            return;
        };

        let x = area.x + (area.width / 2).saturating_sub(info.len() as u16 / 2);
        let y = area.y + (area.height / 2);

        let style = tui::style::Style::default()
            .bg(color);

        buf.set_stringn(x, y, info, area.width as usize, style);
    }
}
