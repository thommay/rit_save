use crate::diff::edit::{Edit, EditKind};
use std::convert::TryInto;

const HUNK_CONTEXT: i32 = 3;

pub struct Hunk {
    a_start: usize,
    b_start: usize,
    pub edits: Vec<Edit>,
}

impl Hunk {
    pub fn filter(edits: Vec<Edit>) -> Vec<Hunk> {
        let mut hunks = Vec::new();
        let mut offset: i32 = 0;
        let edit_len = &edits.len();
        loop {
            while (offset as usize) < *edit_len && edits[offset as usize].is_equals() {
                offset += 1;
            }
            if (offset as usize) >= *edit_len {
                return hunks;
            }

            offset -= HUNK_CONTEXT + 1;
            let a_start = if offset < 0 {
                0
            } else if let Some(ref a) = edits[offset as usize].a {
                a.number
            } else {
                0
            };

            let b_start = if offset < 0 {
                0
            } else if let Some(ref b) = edits[offset as usize].b {
                b.number
            } else {
                0
            };

            let h = Hunk::build(a_start, b_start, &edits, &mut offset);
            hunks.push(h);
        }
    }

    fn build(a: usize, b: usize, edits: &Vec<Edit>, offset: &mut i32) -> Hunk {
        let mut e = vec![];
        let mut counter = -1;

        while counter != 0 {
            if *offset >= 0 && counter > 0 {
                e.push(edits[*offset as usize].clone())
            }
            *offset += 1;
            if *offset >= edits.len().try_into().unwrap() {
                break;
            }
            counter = if *offset + HUNK_CONTEXT >= edits.len().try_into().unwrap() {
                counter - 1
            } else {
                let peak = &edits[(*offset + HUNK_CONTEXT) as usize];
                match peak.kind {
                    EditKind::Insert | EditKind::Delete => 2 * HUNK_CONTEXT + 1,
                    EditKind::Equals => counter - 1,
                }
            };
        }
        Hunk {
            a_start: a,
            b_start: b,
            edits: e,
        }
    }

    pub fn header(&self) -> String {
        let a_lines = self
            .edits
            .iter()
            .filter(|&x| x.a.is_some())
            .collect::<Vec<&Edit>>();
        let a_start = a_lines
            .first()
            .and_then(|&x| Some(x.a.clone().unwrap().number + 1))
            .or(Some(self.a_start))
            .unwrap();

        let b_lines = self
            .edits
            .iter()
            .filter(|&x| x.b.is_some())
            .collect::<Vec<&Edit>>();
        let b_start = b_lines
            .first()
            .and_then(|&x| Some(x.b.clone().unwrap().number + 1))
            .or(Some(self.b_start))
            .unwrap();

        format!(
            "@@ -{},{} +{},{} @@",
            a_start,
            a_lines.len(),
            b_start,
            b_lines.len()
        )
    }
}
