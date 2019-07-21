use std::ops::{Index, IndexMut};

#[derive(Clone, Debug, PartialEq)]
pub struct MyersGraph {
    array: Vec<Option<isize>>,
    max: isize,
}

impl MyersGraph {
    pub fn new(max: isize) -> Self {
        let array = vec![None; (1 + max * 2) as usize];
        Self { array, max }
    }

    pub fn from(array: Vec<Option<isize>>) -> Self {
        let max = ((array.len() - 1) / 2) as isize;
        Self { array, max }
    }

    pub fn len(&self) -> usize {
        self.array.len()
    }
}

impl Index<isize> for MyersGraph {
    type Output = Option<isize>;

    fn index(&self, index: isize) -> &Self::Output {
        let index = index + self.max;
        assert!(index >= 0);
        assert!(index < (2 * self.max + 1));
        &self.array[index as usize]
    }
}

impl IndexMut<isize> for MyersGraph {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        let index = index + self.max;
        &mut self.array[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use crate::diff::myers_graph::MyersGraph;

    #[test]
    fn test_mg_index() {
        let arr = vec![Some(1), None, Some(0), None, Some(2)];
        let mg = MyersGraph { array: arr, max: 2 };
        assert_eq!(mg[-2], Some(1));
        assert_eq!(mg[0], Some(0));
        assert_eq!(mg[2], Some(2));
    }

    #[test]
    fn test_insert_empty() {
        let mut mg = MyersGraph::new(3);
        mg[-1] = Some(2);
        assert_eq!(mg[-1], Some(2))
    }
}
