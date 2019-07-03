use crate::diff::edit::Edit;

#[derive(Debug, PartialEq)]
enum RunningEdit {
    Running,
    Completed,
}

struct Myers {
    a: Vec<&'static str>,
    b: Vec<&'static str>,
}

impl Myers {
    pub fn from(a: &'static str, b: &'static str) -> Self {
        let a = a.lines().collect::<Vec<&str>>();
        let b = b.lines().collect::<Vec<&str>>();
        Myers { a, b }
    }

    pub fn diff(&self) -> Vec<(Edit, &str)> {
        let mut diff = vec![];
        let a_size = self.a.len() as i32;
        let b_size = self.b.len() as i32;
        for (prev_x, prev_y, x, y) in self.backtrack() {
            let a_line = if prev_x < a_size {
                self.a[prev_x as usize]
            } else {
                ""
            };
            let b_line = if prev_y < b_size {
                self.b[prev_y as usize]
            } else {
                ""
            };

            if x == prev_x {
                diff.push((Edit::Insert, b_line));
            } else if y == prev_y {
                diff.push((Edit::Delete, a_line))
            } else {
                diff.push((Edit::Equals, a_line))
            }
        }
        diff.reverse();
        diff
    }

    fn backtrack(&self) -> Vec<(i32, i32, i32, i32)> {
        let mut x = self.a.len() as i32;
        let mut y = self.b.len() as i32;
        let mut ret = vec![];
        let edits = self.shortest_edit();
        let range = (0..edits.len()).rev();

        for (v, d) in edits.iter().rev().zip(range) {
            let d = d as i32;
            let k = x - y;
            let size = v.len();
            let (_i, minus_1, plus_1) = Myers::bounds(size as i32 - 1, k);

            let prev_k = if k == -d || (k != d && v[minus_1] < v[plus_1]) {
                k + 1
            } else {
                k - 1
            };
            let prev_x = v[prev_k as usize].unwrap();
            let prev_y = prev_x - prev_k;

            while x > prev_x && y > prev_y {
                ret.push((x - 1, y - 1, x, y));
                x = x - 1;
                y = y - 1;
            }

            if d > 0 {
                ret.push((prev_x, prev_y, x, y));
            }
            x = prev_x;
            y = prev_y;
        }
        ret
    }

    fn shortest_edit(&self) -> Vec<Vec<Option<i32>>> {
        let n = self.a.len() as i32;
        let m = self.b.len() as i32;
        let max = n + m;
        let mut v: Vec<Option<i32>> = vec![None; max as usize * 2 + 1];
        v[1] = Some(0);
        let mut trace = vec![];
        let mut state: RunningEdit;

        'outer: for d in 0..=max as i32 {
            trace.push(v.clone());
            if d == 0 {
                state = self.shortest_edit_step(n, m, max, &mut v, 0, 0);
                if state == RunningEdit::Completed {
                    break 'outer;
                }
            } else {
                for k in (-d..=d).step_by(2) {
                    state = self.shortest_edit_step(n, m, max, &mut v, d, k);
                    if state == RunningEdit::Completed {
                        break 'outer;
                    }
                }
            }
        }
        trace
    }

    fn shortest_edit_step(
        &self,
        n: i32,
        m: i32,
        max: i32,
        v: &mut Vec<Option<i32>>,
        d: i32,
        k: i32,
    ) -> RunningEdit {
        // if k is negative, start filling out the array from the end rather than the beginning
        let (i, minus_1, plus_1) = Myers::bounds(max * 2, k);

        let opt_x = if k == -d {
            v[plus_1]
        } else if k != d && v[minus_1] < v[plus_1] {
            v[plus_1]
        } else {
            v[minus_1].map(|x| x + 1)
        };

        let mut y: i32 = if let Some(x) = opt_x { x - k } else { 0 };
        let mut x: i32 = opt_x.unwrap_or(0);

        // a diagonal move: if both sides are the same we can keep moving without bumping the score
        while x < n && y < m && self.a[x as usize] == self.b[y as usize] {
            x = x + 1;
            y = y + 1;
        }
        v[i] = Some(x);

        if x >= n && y >= m {
            return RunningEdit::Completed;
        }

        return RunningEdit::Running;
    }

    fn bounds(max: i32, k: i32) -> (usize, usize, usize) {
        let i = if k < 0 { (max + 1) + k } else { k } as usize;
        let minus_1 = if i == 0 { max as usize } else { i - 1 };
        let plus_1 = if i == max as usize { 1 } else { i + 1 };
        (i, minus_1, plus_1)
    }
}

#[cfg(test)]
mod tests {
    use super::Myers;
    use crate::diff::edit::Edit;

    #[test]
    fn test_no_edit() {
        let a = "A\n";
        let b = "A\n";
        let algo = Myers::from(a, b);
        let vals = algo.shortest_edit();
        assert_eq!(1, vals.len())
    }

    #[test]
    fn test_one_edit() {
        let a = "A\n";
        let b = "B\n";
        let algo = Myers::from(a, b);
        let vals = algo.shortest_edit();
        assert_eq!(3, vals.len())
    }

    #[test]
    fn test_shortest_edit() {
        let a = "A\nB\nC\nA\nB\nB\nA\n";
        let b = "C\nB\nA\nB\nA\nC\n";
        let algo = Myers::from(a, b);
        assert_eq!(6, algo.shortest_edit().len())
    }

    #[test]
    fn test_backtrack() {
        let a = "A\nB\nC\nA\nB\nB\nA\n";
        let b = "C\nB\nA\nB\nA\nC\n";
        let algo = Myers::from(a, b);
        let vals = algo.backtrack();
        dbg!(&vals);
    }

    #[test]
    fn test_diff() {
        let a = "A\nB\nC\nA\nB\nB\nA\n";
        let b = "C\nB\nA\nB\nA\nC\n";
        let algo = Myers::from(a, b);
        let vals = algo.diff();

        let expected = vec![
            (Edit::Delete, "A"),
            (Edit::Delete, "B"),
            (Edit::Equals, "C"),
            (Edit::Insert, "B"),
            (Edit::Equals, "A"),
            (Edit::Equals, "B"),
            (Edit::Delete, "B"),
            (Edit::Equals, "A"),
            (Edit::Insert, "C"),
        ];
        assert_eq!(vals, expected)
    }
}
