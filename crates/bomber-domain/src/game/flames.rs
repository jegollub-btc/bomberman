use crate::board::TileGrid;
use crate::shared::Cell;

/// Per-cell remaining lethal ticks. Zero means no fire.
///
/// A dense array rather than a map: boards are small, the field is touched
/// every tick, and dense iteration keeps the order deterministic for free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameField {
    width: u8,
    height: u8,
    ticks: Vec<u8>,
}

impl FlameField {
    pub fn for_grid(grid: &TileGrid) -> Self {
        Self {
            width: grid.width(),
            height: grid.height(),
            ticks: vec![0; grid.width() as usize * grid.height() as usize],
        }
    }

    fn index(&self, cell: Cell) -> Option<usize> {
        (cell.x < self.width && cell.y < self.height)
            .then(|| cell.y as usize * self.width as usize + cell.x as usize)
    }

    pub fn at(&self, cell: Cell) -> u8 {
        self.index(cell).map_or(0, |i| self.ticks[i])
    }

    pub fn is_lethal(&self, cell: Cell) -> bool {
        self.at(cell) > 0
    }

    /// Set a cell alight. An existing, longer-burning flame is never shortened.
    pub fn kindle(&mut self, cell: Cell, ticks: u8) {
        if let Some(i) = self.index(cell) {
            if self.ticks[i] < ticks {
                self.ticks[i] = ticks;
            }
        }
    }

    pub fn extinguish(&mut self, cell: Cell) {
        if let Some(i) = self.index(cell) {
            self.ticks[i] = 0;
        }
    }

    /// Age every flame by one tick, reporting the cells that just went out.
    pub fn decay(&mut self) -> Vec<Cell> {
        let mut extinguished = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = Cell::new(x, y);
                let i = cell.y as usize * self.width as usize + cell.x as usize;
                if self.ticks[i] == 0 {
                    continue;
                }
                self.ticks[i] -= 1;
                if self.ticks[i] == 0 {
                    extinguished.push(cell);
                }
            }
        }
        extinguished
    }

    pub fn burning(&self) -> impl Iterator<Item = (Cell, u8)> + '_ {
        let w = self.width;
        self.ticks
            .iter()
            .enumerate()
            .filter(|(_, &t)| t > 0)
            .map(move |(i, &t)| {
                (
                    Cell::new((i % w as usize) as u8, (i / w as usize) as u8),
                    t,
                )
            })
    }

    pub fn raw(&self) -> &[u8] {
        &self.ticks
    }
}
