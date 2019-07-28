use crate::diff::edit::{Edit, Line};
use crate::diff::myers_graph::MyersGraph;

#[derive(Debug, PartialEq)]
enum RunningEdit {
    Running,
    Completed,
}

pub(crate) struct Myers {
    a: Vec<Line>,
    b: Vec<Line>,
}

impl Myers {
    pub fn from(a: &str, b: &str) -> Self {
        let a = a
            .lines()
            .enumerate()
            .map(|(num, text)| Line {
                content: text.to_owned(),
                number: num,
            })
            .collect::<Vec<Line>>();
        let b = b
            .lines()
            .enumerate()
            .map(|(num, text)| Line {
                content: text.to_owned(),
                number: num,
            })
            .collect::<Vec<Line>>();
        Myers { a, b }
    }

    pub fn diff(&self) -> Vec<Edit> {
        let mut diff = vec![];
        let a_size = self.a.len() as isize;
        let b_size = self.b.len() as isize;
        for (prev_x, prev_y, x, y) in self.backtrack() {
            let a_line = if prev_x < a_size {
                Some(self.a[prev_x as usize].clone())
            } else {
                None
            };
            let b_line = if prev_y < b_size {
                Some(self.b[prev_y as usize].clone())
            } else {
                None
            };

            if x == prev_x {
                diff.push(Edit::insert(None, b_line));
            } else if y == prev_y {
                diff.push(Edit::delete(a_line, None));
            } else {
                diff.push(Edit::equals(a_line, b_line));
            }
        }
        diff.reverse();
        diff
    }

    fn backtrack(&self) -> Vec<(isize, isize, isize, isize)> {
        let mut x = self.a.len() as isize;
        let mut y = self.b.len() as isize;
        let mut ret = vec![];
        let edits = self.shortest_edit();
        let range = (0..edits.len()).rev();

        for (v, d) in edits.iter().rev().zip(range) {
            let d = d as isize;
            let k = x - y;

            let prev_k = if k == -d || (k != d && v[k - 1] < v[k + 1]) {
                k + 1
            } else {
                k - 1
            };
            let prev_x = v[prev_k].unwrap();
            let prev_y = prev_x - prev_k;

            while x > prev_x && y > prev_y {
                ret.push((x - 1, y - 1, x, y));
                x -= 1;
                y -= 1;
            }

            if d > 0 {
                ret.push((prev_x, prev_y, x, y));
            }
            x = prev_x;
            y = prev_y;
        }
        ret
    }

    fn shortest_edit(&self) -> Vec<MyersGraph> {
        let n = self.a.len() as isize;
        let m = self.b.len() as isize;
        let max = n + m;
        let mut v = MyersGraph::new(max);
        v[1] = Some(0);
        let mut trace = vec![];
        let mut state: RunningEdit;

        trace.push(v.clone());
        state = self.shortest_edit_step(n, m, &mut v, 0, 0);
        if state == RunningEdit::Completed {
            return trace;
        }

        for d in 1..=max {
            trace.push(v.clone());
            for k in (-d..=d).step_by(2) {
                state = self.shortest_edit_step(n, m, &mut v, d, k);
                if state == RunningEdit::Completed {
                    return trace;
                }
            }
        }
        trace
    }

    #[allow(clippy::many_single_char_names)]
    fn shortest_edit_step(
        &self,
        n: isize,
        m: isize,
        v: &mut MyersGraph,
        d: isize,
        k: isize,
    ) -> RunningEdit {
        let opt_x = if k == -d || (k != d && v[k - 1] < v[k + 1]) {
            v[k + 1]
        } else {
            v[k - 1].map(|x| x + 1)
        };

        let mut y = if let Some(x) = opt_x { x - k } else { 0 };
        let mut x = opt_x.unwrap_or(0);

        // a diagonal move: if both sides are the same we can keep moving without bumping the score
        while x < n && y < m && self.a[x as usize].content == self.b[y as usize].content {
            x += 1;
            y += 1;
        }
        v[k] = Some(x);

        if x >= n && y >= m {
            return RunningEdit::Completed;
        }

        RunningEdit::Running
    }
}

#[cfg(test)]
mod tests {
    use super::Myers;
    use crate::diff::edit::Edit;
    use crate::diff::edit::EditKind::{Delete, Equals, Insert};
    use crate::diff::edit::Line;
    use crate::diff::myers_graph::MyersGraph;

    #[test]
    fn test_no_edit() {
        let a = "A\n";
        let b = "A\n";
        let algo = Myers::from(a, b);
        let vals = algo.shortest_edit();
        let expected = MyersGraph::from(vec![None, None, None, Some(0), None]);

        assert_eq!(vals.len(), 1);
        let frame = vals.last().unwrap();
        assert_eq!(frame, &expected)
    }

    #[test]
    fn test_one_edit() {
        let a = "A\n";
        let b = "B\n";
        let algo = Myers::from(a, b);
        let vals = algo.shortest_edit();
        let expected = MyersGraph::from(vec![None, Some(0), Some(0), Some(1), None]);

        assert_eq!(vals.len(), 3);
        let frame = vals.last().unwrap();
        assert_eq!(frame, &expected)
    }

    #[test]
    fn test_an_edit() {
        let a = "A\nB\nA\n";
        let b = "B\nB\nB\n";
        let algo = Myers::from(a, b);
        let vals = algo.shortest_edit();
        let expected = vec![
            MyersGraph::from(vec![
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(0),
                None,
                None,
                None,
                None,
                None,
            ]),
            MyersGraph::from(vec![
                None,
                None,
                None,
                None,
                None,
                None,
                Some(0),
                Some(0),
                None,
                None,
                None,
                None,
                None,
            ]),
            MyersGraph::from(vec![
                None,
                None,
                None,
                None,
                None,
                Some(0),
                Some(0),
                Some(2),
                None,
                None,
                None,
                None,
                None,
            ]),
            MyersGraph::from(vec![
                None,
                None,
                None,
                None,
                Some(0),
                Some(0),
                Some(2),
                Some(2),
                Some(3),
                None,
                None,
                None,
                None,
            ]),
            MyersGraph::from(vec![
                None,
                None,
                None,
                Some(0),
                Some(0),
                Some(2),
                Some(2),
                Some(3),
                Some(3),
                Some(4),
                None,
                None,
                None,
            ]),
        ];

        assert_eq!(vals.len(), 5);
        assert_eq!(&vals, &expected)
    }
    #[test]
    fn test_shortest_edit() {
        let a = "A\nB\nC\nA\nB\nB\nA\n";
        let b = "C\nB\nA\nB\nA\nC\n";
        let algo = Myers::from(a, b);
        let vals = algo.shortest_edit();
        dbg!(&vals);
        assert_eq!(vals.len(), 6);
        let expected = MyersGraph::from(vec![
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(3),
            Some(3),
            Some(4),
            Some(4),
            Some(5),
            Some(5),
            Some(7),
            Some(5),
            Some(7),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]);
        let frame = vals.last().unwrap();
        assert_eq!(frame, &expected)
    }

    #[test]
    fn test_diff() {
        let a = "A\nB\nC\nA\nB\nB\nA\n";
        let b = "C\nB\nA\nB\nA\nC\n";
        let algo = Myers::from(a, b);
        let vals = algo.diff();

        let expected = vec![
            Edit {
                kind: Delete,
                a: Some(Line {
                    content: "A".to_owned(),
                    number: 0,
                }),
                b: None,
            },
            Edit {
                kind: Delete,
                a: Some(Line {
                    content: "B".to_owned(),
                    number: 1,
                }),
                b: None,
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "C".to_owned(),
                    number: 2,
                }),
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 0,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 1,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "A".to_owned(),
                    number: 3,
                }),
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 2,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "B".to_owned(),
                    number: 4,
                }),
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 3,
                }),
            },
            Edit {
                kind: Delete,
                a: Some(Line {
                    content: "B".to_owned(),
                    number: 5,
                }),
                b: None,
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "A".to_owned(),
                    number: 6,
                }),
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 4,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 5,
                }),
            },
        ];
        assert_eq!(vals, expected)
    }

    #[test]
    fn test_lopsided_diff() {
        let a = "A\nB\nC\nA\nB\nB\nA\n";
        let b = "C\nB\nA\nB\nA\nC\nC\nB\nA\nB\nA\nC\nC\nB\nA\nB\nA\nC\nC\nB\nA\nB\nA\nC\nC\nB\nA\nB\nA\nC\n";
        let algo = Myers::from(a, b);
        let vals = algo.diff();

        let expected = vec![
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 0,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 1,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "A".to_owned(),
                    number: 0,
                }),
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 2,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "B".to_owned(),
                    number: 1,
                }),
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 3,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 4,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "C".to_owned(),
                    number: 2,
                }),
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 5,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 6,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 7,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "A".to_owned(),
                    number: 3,
                }),
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 8,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "B".to_owned(),
                    number: 4,
                }),
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 9,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 10,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 11,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 12,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "B".to_owned(),
                    number: 5,
                }),
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 13,
                }),
            },
            Edit {
                kind: Equals,
                a: Some(Line {
                    content: "A".to_owned(),
                    number: 6,
                }),
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 14,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 15,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 16,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 17,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 18,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 19,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 20,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 21,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 22,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 23,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 24,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 25,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 26,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "B".to_owned(),
                    number: 27,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "A".to_owned(),
                    number: 28,
                }),
            },
            Edit {
                kind: Insert,
                a: None,
                b: Some(Line {
                    content: "C".to_owned(),
                    number: 29,
                }),
            },
        ];
        assert_eq!(vals, expected)
    }

}
