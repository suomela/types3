#[derive(PartialEq, Eq, Debug)]
pub struct SToken {
    pub id: usize,
    pub count: u64,
    pub marked_count: u64,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Sample {
    pub x: u64,
    pub token_count: u64,
    pub tokens: Vec<SToken>,
}

impl Sample {
    pub fn verify(&self) {
        let mut tc = 0;
        let mut prev_id = None;
        for t in &self.tokens {
            if let Some(prev_id) = prev_id {
                assert!(prev_id < t.id);
            }
            prev_id = Some(t.id);
            assert!(t.marked_count <= t.count);
            assert!(1 <= t.count);
            tc += t.count;
        }
        assert_eq!(tc, self.token_count);
    }
}

pub fn verify_samples(samples: &[Sample]) {
    for s in samples {
        s.verify();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn verify_ok_1() {
        Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken {
                id: 0,
                count: 10,
                marked_count: 0,
            }],
        }
        .verify();
    }

    #[test]
    fn verify_ok_2() {
        Sample {
            x: 1234,
            token_count: 12,
            tokens: vec![
                SToken {
                    id: 0,
                    count: 10,
                    marked_count: 5,
                },
                SToken {
                    id: 1,
                    count: 2,
                    marked_count: 0,
                },
            ],
        }
        .verify();
    }

    #[test]
    #[should_panic(expected = "assert")]
    fn verify_bad_total() {
        Sample {
            x: 1234,
            token_count: 12,
            tokens: vec![SToken {
                id: 0,
                count: 10,
                marked_count: 0,
            }],
        }
        .verify();
    }

    #[test]
    #[should_panic(expected = "assert")]
    fn verify_bad_marked_count() {
        Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken {
                id: 0,
                count: 10,
                marked_count: 20,
            }],
        }
        .verify();
    }

    #[test]
    #[should_panic(expected = "assert")]
    fn verify_bad_count() {
        Sample {
            x: 1234,
            token_count: 2,
            tokens: vec![
                SToken {
                    id: 0,
                    count: 0,
                    marked_count: 0,
                },
                SToken {
                    id: 1,
                    count: 2,
                    marked_count: 0,
                },
            ],
        }
        .verify();
    }

    #[test]
    #[should_panic(expected = "assert")]
    fn verify_bad_order() {
        Sample {
            x: 1234,
            token_count: 12,
            tokens: vec![
                SToken {
                    id: 1,
                    count: 2,
                    marked_count: 0,
                },
                SToken {
                    id: 0,
                    count: 10,
                    marked_count: 5,
                },
            ],
        }
        .verify();
    }
}
