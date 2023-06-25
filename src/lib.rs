use core::ops::Range;

type Coord = u64;
type Value = u64;

const DIMBITS: usize = 4;
const DIMSIZE: usize = 1 << DIMBITS;

fn calc_bits(n: Coord) -> usize {
    let mut n = n;
    let mut b = 0;
    while n >= DIMSIZE as Coord {
        b += DIMBITS;
        n <<= DIMBITS;
    }
    b
}

type Cell1D = [Cell; DIMSIZE];
type Cell2D = [Cell1D; DIMSIZE];

pub enum Node {
    Bottom,
    SubY(Cell1D),
    SubX(Cell1D),
    SubYX(Cell2D),
}

pub struct Cell {
    children: Box<Node>,
    value: Value,
}

pub struct Grid {
    ny: Coord,
    nx: Coord,
    by: usize,
    bx: usize,
    top: Cell2D,
}

impl Cell {
    fn new() -> Cell {
        Cell {
            children: Box::new(Node::Bottom),
            value: 0,
        }
    }

    fn new_1d() -> Cell1D {
        core::array::from_fn(|i| Cell::new())
    }

    fn new_2d() -> Cell2D {
        core::array::from_fn(|i| Cell::new_1d())
    }
}

impl Grid {
    fn new(ny: Coord, nx: Coord) -> Grid {
        Grid {
            ny,
            nx,
            by: calc_bits(ny),
            bx: calc_bits(nx),
            top: Cell::new_2d(),
        }
    }

    fn add(&mut self, yy: Range<Coord>, xx: Range<Coord>, v: Value) {
        let y0 = yy.start;
        let y1 = yy.end;
        let x0 = xx.start;
        let x1 = xx.end;
        assert!(y1 <= self.ny);
        assert!(x1 <= self.nx);
        // FIXME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_basic() {
        let mut grid = Grid::new(1234, 5678);
        grid.add(111..222, 3333..4444, 999);
    }
}
