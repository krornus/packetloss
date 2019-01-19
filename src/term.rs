use std::iter::Iterator;
use std::cmp::min;
use std::f64::INFINITY;

use tui::layout::Rect;
use tui::buffer::Buffer;
use tui::widgets::{Block, Widget};

use crate::ping::PacketChunk;

pub struct LogList<'b> {
    block: Option<Block<'b>>,
    items: Vec<PacketChunk>,
    min_latency: f64,
}


impl<'b> Default for LogList<'b> {
    fn default() -> Self {
        LogList {
            block: None,
            items: vec![],
            min_latency: INFINITY,
        }
    }
}


impl<'b> LogList<'b> {
    pub fn insert(&mut self, item: PacketChunk) {
        if item.latency() < self.min_latency {
            self.min_latency = item.latency();
        }
        self.items.insert(0, item);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn block(mut self, block: Block<'b>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn partition(&mut self, size: Rect) -> LogListPartitioner {
        LogListPartitioner {
            x: 0,
            y: 0,
            width: size.width,
            max_width: size.width,
            height: size.height,
            length: self.len() as u16,
        }
    }
}

#[derive(Debug)]
pub struct LogListPartitioner {
    x: u16,
    y: u16,
    width: u16,
    max_width: u16,
    height: u16,
    length: u16,
}

impl LogListPartitioner {
    fn ceil(a: u16, b: u16) -> u16 {
        if a == 0 {
            0
        } else {
            1 + ((a - 1) / b)
        }
    }
}

impl Iterator for LogListPartitioner {
    type Item = Rect;

    /*
     * we want to use up all of size
     * dont exceed size
     */
    fn next(&mut self) -> Option<Self::Item> {

        if self.height == 0 || self.length == 0 {
            return None;
        }

        let x = self.x;
        let y = self.y;

        let after = min(self.length, (self.height - 1) * self.max_width);

        let mut wdiv = dbg!((self.length - after) + 1);
        let hdiv = min(self.height, self.length);

        if self.height == 1 && wdiv > 0 {
            wdiv = dbg!(wdiv - 1);
        }

        let width = LogListPartitioner::ceil(self.width, wdiv);
        let height = LogListPartitioner::ceil(self.height, hdiv);

        dbg!(self.width);
        self.width -= width;
        self.height -= height - 1;

        /* if the line's width was consumed consume one more line and reset width */
        if self.width == 0 && self.height > 1 {
            self.width = self.max_width;
            dbg!(self.height -= 1);
            self.y += 1;
        }

        self.x += width;
        dbg!(self.y += height - 1);

        if self.x == self.max_width {
            self.x = 0;
        }

        dbg!(self.length -= 1);

        Some(Rect::new(x,y,width,height))
    }
}

impl<'b> Widget for LogList<'b> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {

        let area = self.block.map(|ref mut x| {
            x.draw(area, buf);
            x.inner(area)
        }).unwrap_or(area);

        if area.width == 0 || area.height == 0 {
            return;
        }

        let partitions = self.partition(area);
        for (item, area) in self.items.iter_mut().zip(partitions) {
            item.min = self.min_latency;
            item.draw(area, buf);
        }

    }
}
