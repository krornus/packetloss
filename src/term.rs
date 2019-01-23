use std::iter::Iterator;
use std::cmp::min;
use std::f64::INFINITY;
use std::collections::vec_deque::VecDeque;

use tui::layout::Rect;
use tui::buffer::Buffer;
use tui::widgets::{Block, Widget, Borders};
use tui::style::{Style, Color};

use crate::ping::{DrawablePacket, PacketChunk};

pub struct LogList<'b> {
    block: Option<Block<'b>>,
    items: VecDeque<PacketChunk>,
    min_latency: f64,
    max: usize,
}


impl<'b> LogList<'b> {
    pub fn new(max: usize) -> Self {
        LogList {
            block: None,
            items: VecDeque::new(),
            min_latency: INFINITY,
            max: max,
        }
    }
}


impl<'b> LogList<'b> {
    pub fn insert(&mut self, item: PacketChunk) {
        if item.latency() < self.min_latency {
            self.min_latency = item.latency();
        }

        self.items.push_front(item);

        /* prevent oom */
        if self.items.len() >= self.max {
            self.items.pop_back();
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn block(&mut self, block: Block<'b>) {
        self.block = Some(block);
    }

    pub fn partition(&mut self, size: Rect) -> LogListPartitioner {
        LogListPartitioner {
            x: 0,
            y: 0,
            offset_x: size.x,
            offset_y: size.y,
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
    offset_x: u16,
    offset_y: u16,
    width: u16,
    max_width: u16,
    height: u16,
    length: u16,
}

fn ceil(a: u16, b: u16) -> u16 {
    if a == 0 {
        0
    } else {
        1 + ((a - 1) / b)
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

        let mut wdiv = (self.length - after) + 1;
        let hdiv = min(self.height, self.length);

        if self.height == 1 && wdiv > 0 {
            wdiv = wdiv - 1;
        }

        let width = ceil(self.width, wdiv);
        let height = ceil(self.height, hdiv);

        self.width -= width;
        self.height -= height - 1;

        /* if the line's width was consumed consume one more line and reset width */
        if self.width == 0 && self.height > 1 {
            self.width = self.max_width;
            self.height -= 1;
            self.y += 1;
        }

        self.x += width;
        self.y += height - 1;

        if self.x == self.max_width {
            self.x = 0;
        }

        self.length -= 1;

        Some(Rect::new(x+self.offset_x, y+self.offset_y, width, height))
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
            let mut drawable = DrawablePacket::new(item, self.min_latency);
            drawable.draw(area, buf);
        }

    }
}

pub struct SelectableLogList<'b> {
    selection: Option<usize>,
    block: Option<Block<'b>>,
    list: LogList<'b>,
    min_height: u16,
}

impl<'b> SelectableLogList<'b> {
    pub fn new(max: usize) -> Self {
        SelectableLogList {
            list: LogList::new(max),
            selection: None,
            block: None,
            min_height: 5,
        }
    }

    pub fn insert(&mut self, item: PacketChunk) {
        self.list.insert(item);

        if let Some(i) = self.selection {
            /* sticky top */
            if i > 0 {
                self.select(i+1);
            } else {
                self.list.items[1].tint_weight(0.0);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn select(&mut self, i: usize) {

        if let Some(i) = self.selection {
            self.list.items[i].tint_weight(0.0);
        }

        self.selection = Some(i);
        self.list.items[i].tint_weight(0.5);
    }

    pub fn clear(&mut self) {

        if let Some(i) = self.selection {
            self.list.items[i].tint_weight(0.0);
        }

        self.selection = None;
    }

    pub fn has_selection(&self) -> bool {
        self.selection.is_some() && self.selection.unwrap() < self.len()
    }

    pub fn select_next(&mut self) {
        if let Some(i) = self.selection {
            if i < self.len() - 1 {
                self.select(i+1);
            }
        } else {
            self.select(0);
        }
    }

    pub fn select_prev(&mut self) {
        if let Some(i) = self.selection {
            if i > 0 {
                self.select(i-1);
            }
        } else {
            self.select(0);
        }
    }

    pub fn select_last(&mut self) {
        self.select(self.len() - 1);
    }

    pub fn select_first(&mut self) {
        self.select(0);
    }
}

impl<'b> Widget for SelectableLogList<'b> {
    fn draw(&mut self, mut area: Rect, buf: &mut Buffer) {

        if area.width == 0 || area.height == 0 {
            return;
        }

        if !self.has_selection() {
            self.list.draw(area, buf);
            return;
        }

        let i = self.selection.unwrap();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default()
                .fg(Color::White))
            .style(Style::default()
                .bg(Color::Black));

        let mut inspect_block = block.clone().title(" Inspect packet ");

        let mut rect = self.list.partition(area).next().unwrap();

        if rect.height < self.min_height {
            rect.height = self.min_height;
        }

        /* keep it centered */
        if rect.height % 2 == 0 {
            rect.height += 1;
        }

        if rect.height > area.height || rect.width > area.width {
            self.list.draw(area, buf);
            return;
        }

        inspect_block.draw(rect, buf);
        let inner = inspect_block.inner(rect);

        self.list.items[i].tint_weight(0.0);
        let mut drawable = DrawablePacket::new(&self.list.items[i], self.list.min_latency);
        drawable.draw(inner, buf);
        self.list.items[i].tint_weight(0.5);

        self.block = None;

        area.height -= rect.height;
        area.y += rect.height;

        self.list.block(block.clone().title(" Packet list "));
        self.list.draw(area, buf);
        self.list.block = None;
    }
}
