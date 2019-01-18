use std::iter;
use std::iter::{Extend, Iterator};
use std::cmp::max;
use std::io;

use tui::Terminal;
use tui::backend::TermionBackend;
use termion::raw::IntoRawMode;

use tui::layout::{Corner, Rect};
use tui::style::Style;
use tui::buffer::Buffer;
use tui::widgets::{Block, Text, Widget};

use crate::ping::PacketChunk;

pub struct LogList<'b> {
    block: Option<Block<'b>>,
    items: Vec<PacketChunk>,
    style: Style,
    start_corner: Corner,
}


impl<'b> Default for LogList<'b> {
    fn default() -> Self {
        LogList {
            block: None,
            items: vec![],
            style: Style::default(),
            start_corner: Corner::TopLeft,
        }
    }
}


impl<'b> LogList<'b> {
    pub fn new(items: Vec<PacketChunk>) -> Self {
        LogList {
            block: None,
            items,
            style: Style::default(),
            start_corner: Corner::TopLeft,
        }
    }

    pub fn insert(&mut self, item: PacketChunk) {
        self.items.insert(0, item);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn block(mut self, block: Block<'b>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn start_corner(mut self, corner: Corner) -> Self {
        self.start_corner = corner;
        self
    }

    pub fn partition(&mut self, size: Rect) -> LogListPartitioner {
        LogListPartitioner {
            max_length: self.len() as u16,
            length: self.len() as u16,
            size: size,
        }
    }
}

struct LogListPartitioner {
    max_length: u16,
    length: u16,
    size: Rect,
}

impl LogListPartitioner {
    fn divisor(&self) -> u16 {
        (self.max_length - self.length) + 1
    }
}

impl Iterator for LogListPartitioner {
    type Item = Rect;

    /*
     * we want to use up all of size
     * dont exceed size
     */
    fn next(&mut self) -> Option<Self::Item> {

        if self.size.height == 0 || self.size.width == 0 || self.length == 0 {
            return None;
        }

        self.length -= 1;
        let divisor = self.divisor();

        /* height and width are both > 1 */
        /* we have mutliple items remaining requiring space */
        /* divide available space */
        /* subtract used space from self.size */
        let rect = if self.size.height < divisor {

            /* if we cant divide again, just return all space remaining */
            if self.size.width < divisor {
                return Some(self.size.clone());
            }

            let width = self.size.width / divisor;
            let rect = Rect::new(self.size.x, self.size.y, width, 1);
            /* move forward by width */
            self.size.x += width;
            self.size.width -= width;

            rect
        } else {
            let height = self.size.height / divisor;
            let rect = Rect::new(self.size.x, self.size.y, self.size.width, height);
            /* move down by height */
            self.size.y += height;
            self.size.height -= height;

            rect
        };

        Some(rect)
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

        let single = self.items.len() == 1;

        self.background(&area, buf, self.style.bg);

        let mut consumed = 0;

        let partitions = self.partition(area);

        for (item, area) in self.items.iter_mut().zip(partitions) {
            item.draw(area, buf);
        }

    }
}
