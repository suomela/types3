use core::ops::Range;
use std::collections::HashMap;

type Coord = u64;
type Value = i64;

#[derive(Debug)]
pub struct Grid {
    values: HashMap<(Coord, Coord), Value>,
}

#[derive(Debug)]
pub struct RawPoint {
    pub x: Coord,
    pub v: Value,
}

#[derive(Debug)]
pub struct RawLine {
    pub y: Coord,
    pub values: Vec<RawPoint>,
}

#[derive(Debug)]
pub struct RawLines {
    pub ny: Coord,
    pub nx: Coord,
    pub lines: Vec<RawLine>,
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            values: HashMap::new(),
        }
    }

    pub fn add(&mut self, y: Coord, xx: Range<Coord>, v: Value) {
        let x0 = xx.start;
        let x1 = xx.end;
        *self.values.entry((y, x0)).or_insert(0) += v;
        *self.values.entry((y, x1)).or_insert(0) -= v;
    }

    pub fn to_rawlines(&self) -> RawLines {
        let mut points: Vec<_> = self.values.iter().collect();
        points.sort();
        let mut lines = Vec::new();
        let mut ocurline = None;
        let mut ny = 0;
        let mut nx = 0;
        for (&(y, x), &v) in points {
            if v == 0 {
                continue;
            }
            ny = ny.max(y + 1);
            nx = nx.max(x);
            let lp = RawPoint { x, v };
            ocurline = match ocurline {
                None => Some(RawLine {
                    y,
                    values: vec![lp],
                }),
                Some(mut curline) => {
                    if curline.y == y {
                        curline.values.push(lp);
                        Some(curline)
                    } else {
                        lines.push(curline);
                        Some(RawLine {
                            y,
                            values: vec![lp],
                        })
                    }
                }
            };
        }
        match ocurline {
            None => {}
            Some(curline) => {
                lines.push(curline);
            }
        }
        RawLines { ny, nx, lines }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_basic() {
        let mut grid = Grid::new();
        grid.add(111, 4000..4444, 1);
        grid.add(111, 3333..4000, 999);
        grid.add(222, 3111..4111, 9999);
        grid.add(111, 4000..4444, 998);
        grid.add(333, 5555..6666, 1);
        grid.add(333, 5555..6666, -1);
        let lines = grid.to_rawlines();
        assert_eq!(lines.ny, 223);
        assert_eq!(lines.nx, 4444);
        assert_eq!(lines.lines.len(), 2);
        assert_eq!(lines.lines[0].y, 111);
        assert_eq!(lines.lines[0].values.len(), 2);
        assert_eq!(lines.lines[0].values[0].x, 3333);
        assert_eq!(lines.lines[0].values[0].v, 999);
        assert_eq!(lines.lines[0].values[1].x, 4444);
        assert_eq!(lines.lines[0].values[1].v, -999);
        assert_eq!(lines.lines[1].y, 222);
        assert_eq!(lines.lines[1].values[0].x, 3111);
        assert_eq!(lines.lines[1].values[0].v, 9999);
        assert_eq!(lines.lines[1].values[1].x, 4111);
        assert_eq!(lines.lines[1].values[1].v, -9999);
    }
}
