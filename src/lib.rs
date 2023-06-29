use core::ops::Range;
use std::{cmp::Ordering, collections::HashMap};

type Coord = u64;
type Value = i64;

#[derive(Debug)]
pub struct Grid {
    values: HashMap<(Coord, Coord), Value>,
}

/// Represents the change of the sum for one point
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RawPoint {
    pub x: Coord,
    pub v: Value,
}

/// Represents the change of the sums for one line
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

/// Represents sums for one horizontal line segment, for x coordinates less than `x`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SumPoint {
    pub x: Coord,
    pub sum: Value,
}

/// Represents sums for one horizontal slice, for y coordinates less than `y`
#[derive(Debug)]
pub struct SumLine {
    pub y: Coord,
    pub sums: Vec<SumPoint>,
}

#[derive(Debug)]
pub struct Sums {
    pub ny: Coord,
    pub nx: Coord,
    pub lines: Vec<SumLine>,
}

pub fn cum_sum(a: &[RawPoint]) -> Vec<SumPoint> {
    let mut sums = Vec::new();
    let mut sum = 0;
    for &RawPoint { x, v } in a {
        debug_assert!(v != 0);
        sums.push(SumPoint { x, sum });
        sum += v;
    }
    assert_eq!(sum, 0);
    sums
}

fn push_or_change(r: &mut Vec<SumPoint>, v: SumPoint) {
    match r.last_mut() {
        None => {
            r.push(v);
        }
        Some(l) => {
            debug_assert!(l.x <= v.x);
            if l.x == v.x {
                l.sum = v.sum;
            } else if l.sum == v.sum {
                l.x = v.x;
            } else {
                r.push(v);
            }
        }
    }
}

pub fn add_lines_to(a: &[SumPoint], b: &[SumPoint], r: &mut Vec<SumPoint>) {
    let mut i = 0;
    let mut j = 0;
    while i < a.len() && j < b.len() {
        let sum = a[i].sum + b[j].sum;
        match a[i].x.cmp(&b[j].x) {
            Ordering::Equal => {
                push_or_change(r, SumPoint { x: a[i].x, sum });
                i += 1;
                j += 1;
            }
            Ordering::Less => {
                push_or_change(r, SumPoint { x: a[i].x, sum });
                i += 1;
            }
            Ordering::Greater => {
                push_or_change(r, SumPoint { x: b[j].x, sum });
                j += 1;
            }
        }
    }
    while i < a.len() {
        push_or_change(r, a[i]);
        i += 1;
    }
    while j < b.len() {
        push_or_change(r, b[j]);
        j += 1;
    }
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

impl RawLines {
    pub fn to_sums(&self) -> Sums {
        Sums {
            ny: self.ny,
            nx: self.nx,
            lines: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rp(x: Coord, v: Value) -> RawPoint {
        RawPoint { x, v }
    }

    fn sp(x: Coord, sum: Value) -> SumPoint {
        SumPoint { x, sum }
    }

    fn add_lines(a: &[SumPoint], b: &[SumPoint]) -> Vec<SumPoint> {
        let mut r = Vec::new();
        add_lines_to(a, b, &mut r);
        r
    }

    #[test]
    fn push_or_change_basic() {
        let mut r = Vec::new();
        push_or_change(&mut r, sp(100, 2));
        assert_eq!(&r, &[sp(100, 2)]);
        push_or_change(&mut r, sp(200, 3));
        assert_eq!(&r, &[sp(100, 2), sp(200, 3)]);
        push_or_change(&mut r, sp(300, 2));
        assert_eq!(&r, &[sp(100, 2), sp(200, 3), sp(300, 2)]);
        push_or_change(&mut r, sp(400, 3));
        assert_eq!(&r, &[sp(100, 2), sp(200, 3), sp(300, 2), sp(400, 3)]);
        push_or_change(&mut r, sp(500, 3));
        assert_eq!(&r, &[sp(100, 2), sp(200, 3), sp(300, 2), sp(500, 3)]);
        push_or_change(&mut r, sp(500, 4));
        assert_eq!(&r, &[sp(100, 2), sp(200, 3), sp(300, 2), sp(500, 4)]);
    }

    #[test]
    fn cum_sum_basic() {
        assert_eq!(cum_sum(&[]), &[]);
        assert_eq!(
            cum_sum(&[rp(100, 2), rp(200, -2)]),
            &[sp(100, 0), sp(200, 2)]
        );
        assert_eq!(
            cum_sum(&[rp(100, 2), rp(200, 1), rp(300, -3)]),
            &[sp(100, 0), sp(200, 2), sp(300, 3)]
        );
    }

    #[test]
    fn add_lines_same_length() {
        assert_eq!(add_lines(&[], &[]), &[]);
        assert_eq!(
            add_lines(&[sp(100, 0), sp(200, 2)], &[sp(100, 0), sp(200, 3)]),
            &[sp(100, 0), sp(200, 5)]
        );
        assert_eq!(
            add_lines(&[sp(110, 0), sp(200, 2)], &[sp(100, 0), sp(200, 3)]),
            &[sp(100, 0), sp(110, 3), sp(200, 5)]
        );
        assert_eq!(
            add_lines(&[sp(100, 0), sp(200, 2)], &[sp(110, 0), sp(200, 3)]),
            &[sp(100, 0), sp(110, 2), sp(200, 5)]
        );
    }

    #[test]
    fn add_lines_different_length() {
        assert_eq!(add_lines(&[], &[]), &[]);
        assert_eq!(
            add_lines(&[sp(100, 0), sp(200, 2)], &[sp(100, 0), sp(300, 3)]),
            &[sp(100, 0), sp(200, 5), sp(300, 3)]
        );
        assert_eq!(
            add_lines(&[sp(100, 0), sp(300, 2)], &[sp(100, 0), sp(200, 3)]),
            &[sp(100, 0), sp(200, 5), sp(300, 2)]
        );
        assert_eq!(
            add_lines(&[sp(110, 0), sp(200, 2)], &[sp(100, 0), sp(200, 3)]),
            &[sp(100, 0), sp(110, 3), sp(200, 5)]
        );
        assert_eq!(
            add_lines(&[sp(110, 0), sp(200, 2)], &[sp(100, 0), sp(300, 3)]),
            &[sp(100, 0), sp(110, 3), sp(200, 5), sp(300, 3)]
        );
        assert_eq!(
            add_lines(&[sp(110, 0), sp(300, 2)], &[sp(100, 0), sp(200, 3)]),
            &[sp(100, 0), sp(110, 3), sp(200, 5), sp(300, 2)]
        );
        assert_eq!(
            add_lines(&[sp(100, 0), sp(200, 2)], &[sp(110, 0), sp(300, 3)]),
            &[sp(100, 0), sp(110, 2), sp(200, 5), sp(300, 3)]
        );
        assert_eq!(
            add_lines(&[sp(100, 0), sp(300, 2)], &[sp(110, 0), sp(200, 3)]),
            &[sp(100, 0), sp(110, 2), sp(200, 5), sp(300, 2)]
        );
    }

    #[test]
    fn add_lines_no_redundant_points() {
        assert_eq!(
            add_lines(
                &[sp(100, 0), sp(200, 2), sp(300, 3)],
                &[sp(100, 0), sp(200, 3), sp(300, 2)]
            ),
            &[sp(100, 0), sp(300, 5)]
        );
        assert_eq!(
            add_lines(
                &[sp(100, 0), sp(200, 4), sp(300, 3)],
                &[sp(100, 0), sp(200, 3), sp(300, 2)]
            ),
            &[sp(100, 0), sp(200, 7), sp(300, 5)]
        );
        assert_eq!(
            add_lines(
                &[sp(100, 0), sp(150, 1), sp(200, 2), sp(300, 3)],
                &[sp(100, 0), sp(200, 3), sp(250, 2), sp(300, 4)]
            ),
            &[sp(100, 0), sp(150, 4), sp(250, 5), sp(300, 7)]
        );
        assert_eq!(
            add_lines(
                &[sp(100, 0), sp(150, 1), sp(200, 2), sp(300, 3)],
                &[sp(100, 0), sp(201, 3), sp(250, 2), sp(300, 4)]
            ),
            &[
                sp(100, 0),
                sp(150, 4),
                sp(200, 5),
                sp(201, 6),
                sp(250, 5),
                sp(300, 7)
            ]
        );
        assert_eq!(
            add_lines(
                &[sp(100, 0), sp(150, 1), sp(200, 2), sp(300, 3)],
                &[sp(100, 0), sp(199, 3), sp(250, 2), sp(300, 4)]
            ),
            &[
                sp(100, 0),
                sp(150, 4),
                sp(199, 5),
                sp(200, 4),
                sp(250, 5),
                sp(300, 7)
            ]
        );
    }

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
