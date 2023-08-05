use itertools::Itertools;
use log::debug;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::cmp::Ordering;

pub type Coord = u64;
pub type Value = i64;
pub type CRange = (Coord, Coord);

#[derive(Debug)]
pub struct Counter {
    values: Vec<FxHashMap<Coord, Value>>,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { values: Vec::new() }
    }

    pub fn add(&mut self, y: Coord, xx: CRange, v: Value) {
        let y = y as usize;
        if self.values.len() <= y {
            self.values.resize_with(y + 1, Default::default);
        }
        let (x0, x1) = xx;
        self.values[y]
            .entry(x0)
            .and_modify(|e| *e += v)
            .or_insert(v);
        self.values[y]
            .entry(x1)
            .and_modify(|e| *e -= v)
            .or_insert(-v);
    }

    pub fn merge(&mut self, other: &Counter) {
        debug!("merge start");
        if self.values.len() < other.values.len() {
            self.values
                .resize_with(other.values.len(), Default::default);
        }
        for y in 0..other.values.len() {
            for (&x, &v) in &other.values[y] {
                if v != 0 {
                    self.values[y].entry(x).and_modify(|e| *e += v).or_insert(v);
                }
            }
        }
        debug!("merge finished");
    }

    pub fn to_rawlines(&self) -> RawLines {
        let mut ny = 0;
        let mut nx = 0;
        let mut lines = Vec::new();
        for y in 0..self.values.len() {
            let mut values = self.values[y]
                .iter()
                .filter_map(|(&x, &v)| {
                    if v == 0 {
                        None
                    } else {
                        Some(RawPoint { x, v })
                    }
                })
                .collect_vec();
            if values.is_empty() {
                continue;
            }
            let y = y as Coord;
            values.sort_unstable_by_key(|p| p.x);
            ny = ny.max(y + 1);
            nx = nx.max(values.last().unwrap().x);
            lines.push(RawLine { y, values });
        }
        RawLines { ny, nx, lines }
    }

    pub fn to_sums(&self) -> Sums {
        self.to_rawlines().to_sums()
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
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

impl RawLine {
    pub fn to_sumline(&self) -> SumLine {
        SumLine {
            y: self.y + 1,
            sums: cum_sum(&self.values),
        }
    }
}

#[derive(Debug)]
pub struct RawLines {
    pub ny: Coord,
    pub nx: Coord,
    pub lines: Vec<RawLine>,
}

impl RawLines {
    pub fn to_sums(&self) -> Sums {
        let mut lines: Vec<SumLine> = self.lines.iter().map(|x| x.to_sumline()).collect();
        let n = lines.len();
        if n > 0 {
            for i in (0..n - 1).rev() {
                lines[i].sums = add_lines(&lines[i].sums, &lines[i + 1].sums);
            }
        }
        Sums {
            ny: self.ny,
            nx: self.nx,
            lines,
        }
    }
}

/// Represents sums for one horizontal line segment, for x coordinates less than `x`
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
pub struct SumPoint {
    pub x: Coord,
    pub sum: Value,
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

fn add_lines_to(a: &[SumPoint], b: &[SumPoint], r: &mut Vec<SumPoint>) {
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

fn add_lines(a: &[SumPoint], b: &[SumPoint]) -> Vec<SumPoint> {
    let mut r = Vec::new();
    add_lines_to(a, b, &mut r);
    r
}

/// Represents sums for one horizontal slice, for y coordinates less than `y`
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SumLine {
    pub y: Coord,
    pub sums: Vec<SumPoint>,
}

#[derive(Debug, Serialize)]
pub struct Sums {
    pub ny: Coord,
    pub nx: Coord,
    pub lines: Vec<SumLine>,
}

impl Sums {
    pub fn total_points(&self) -> usize {
        self.lines.iter().map(|x| x.sums.len()).sum()
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
    fn counter_basic() {
        let mut counter = Counter::new();
        counter.add(111, (4000, 4444), 1);
        counter.add(111, (3333, 4000), 999);
        counter.add(222, (3111, 4111), 9999);
        counter.add(111, (4000, 4444), 998);
        counter.add(333, (5555, 6666), 1);
        counter.add(333, (5555, 6666), -1);
        let lines = counter.to_rawlines();
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
        assert_eq!(lines.lines[1].values.len(), 2);
        assert_eq!(lines.lines[1].values[0].x, 3111);
        assert_eq!(lines.lines[1].values[0].v, 9999);
        assert_eq!(lines.lines[1].values[1].x, 4111);
        assert_eq!(lines.lines[1].values[1].v, -9999);
    }

    #[test]
    fn counter_sums_basic() {
        let mut counter = Counter::new();
        counter.add(111, (4000, 4444), 1);
        counter.add(111, (3333, 4000), 999);
        counter.add(222, (3111, 4111), 9999);
        counter.add(111, (4000, 4444), 998);
        counter.add(333, (5555, 6666), 1);
        counter.add(333, (5555, 6666), -1);
        let sums = counter.to_sums();
        assert_eq!(sums.ny, 223);
        assert_eq!(sums.nx, 4444);
        assert_eq!(sums.lines.len(), 2);
        assert_eq!(sums.lines[0].y, 112);
        assert_eq!(sums.lines[0].sums.len(), 4);
        assert_eq!(sums.lines[0].sums[0].x, 3111);
        assert_eq!(sums.lines[0].sums[0].sum, 0);
        assert_eq!(sums.lines[0].sums[1].x, 3333);
        assert_eq!(sums.lines[0].sums[1].sum, 9999);
        assert_eq!(sums.lines[0].sums[2].x, 4111);
        assert_eq!(sums.lines[0].sums[2].sum, 9999 + 999);
        assert_eq!(sums.lines[0].sums[3].x, 4444);
        assert_eq!(sums.lines[0].sums[3].sum, 999);
        assert_eq!(sums.lines[1].y, 223);
        assert_eq!(sums.lines[1].sums.len(), 2);
        assert_eq!(sums.lines[1].sums[0].x, 3111);
        assert_eq!(sums.lines[1].sums[0].sum, 0);
        assert_eq!(sums.lines[1].sums[1].x, 4111);
        assert_eq!(sums.lines[1].sums[1].sum, 9999);
    }

    #[test]
    fn counter_sums_one_curve() {
        let mut counter = Counter::new();
        counter.add(0, (0, 100), 1);
        counter.add(10, (100, 200), 1);
        counter.add(20, (200, 300), 1);

        let sums = counter.to_sums();
        assert_eq!(sums.ny, 21);
        assert_eq!(sums.nx, 300);
        assert_eq!(sums.lines.len(), 3);
        assert_eq!(sums.lines[0].y, 1);
        assert_eq!(sums.lines[0].sums, &[sp(0, 0), sp(300, 1)]);
        assert_eq!(sums.lines[1].y, 11);
        assert_eq!(sums.lines[1].sums, &[sp(100, 0), sp(300, 1)]);
        assert_eq!(sums.lines[2].y, 21);
        assert_eq!(sums.lines[2].sums, &[sp(200, 0), sp(300, 1)]);
    }

    #[test]
    fn counter_sums_one_fat_curve() {
        let mut counter = Counter::new();
        counter.add(0, (0, 100), 1000);
        counter.add(10, (100, 200), 1000);
        counter.add(20, (200, 300), 1000);

        let sums = counter.to_sums();
        assert_eq!(sums.ny, 21);
        assert_eq!(sums.nx, 300);
        assert_eq!(sums.lines.len(), 3);
        assert_eq!(sums.lines[0].y, 1);
        assert_eq!(sums.lines[0].sums, &[sp(0, 0), sp(300, 1000)]);
        assert_eq!(sums.lines[1].y, 11);
        assert_eq!(sums.lines[1].sums, &[sp(100, 0), sp(300, 1000)]);
        assert_eq!(sums.lines[2].y, 21);
        assert_eq!(sums.lines[2].sums, &[sp(200, 0), sp(300, 1000)]);
    }

    #[test]
    fn counter_sums_two_curves() {
        let mut counter = Counter::new();
        counter.add(0, (0, 100), 1);
        counter.add(10, (100, 200), 1);
        counter.add(20, (200, 300), 1);
        counter.add(0, (0, 150), 1);
        counter.add(30, (150, 300), 1);

        let sums = counter.to_sums();
        assert_eq!(sums.ny, 31);
        assert_eq!(sums.nx, 300);
        assert_eq!(sums.lines.len(), 4);
        assert_eq!(sums.lines[0].y, 1);
        assert_eq!(sums.lines[0].sums, &[sp(0, 0), sp(300, 2)]);
        assert_eq!(sums.lines[1].y, 11);
        assert_eq!(sums.lines[1].sums, &[sp(100, 0), sp(150, 1), sp(300, 2)]);
        assert_eq!(sums.lines[2].y, 21);
        assert_eq!(sums.lines[2].sums, &[sp(150, 0), sp(200, 1), sp(300, 2)]);
        assert_eq!(sums.lines[3].y, 31);
        assert_eq!(sums.lines[3].sums, &[sp(150, 0), sp(300, 1)]);
    }

    #[test]
    fn counter_merge() {
        let mut counter1 = Counter::new();
        let mut counter2 = Counter::new();
        counter1.add(0, (0, 100), 1);
        counter2.add(10, (100, 200), 1);
        counter1.add(20, (200, 300), 1);
        counter2.add(0, (0, 150), 1);
        counter1.add(30, (150, 300), 1);

        let mut counter = Counter::new();
        counter.merge(&counter1);
        counter.merge(&counter2);

        let sums = counter.to_sums();
        assert_eq!(sums.ny, 31);
        assert_eq!(sums.nx, 300);
        assert_eq!(sums.lines.len(), 4);
        assert_eq!(sums.lines[0].y, 1);
        assert_eq!(sums.lines[0].sums, &[sp(0, 0), sp(300, 2)]);
        assert_eq!(sums.lines[1].y, 11);
        assert_eq!(sums.lines[1].sums, &[sp(100, 0), sp(150, 1), sp(300, 2)]);
        assert_eq!(sums.lines[2].y, 21);
        assert_eq!(sums.lines[2].sums, &[sp(150, 0), sp(200, 1), sp(300, 2)]);
        assert_eq!(sums.lines[3].y, 31);
        assert_eq!(sums.lines[3].sums, &[sp(150, 0), sp(300, 1)]);
    }
}
