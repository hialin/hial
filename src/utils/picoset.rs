use std::collections::HashSet;
use std::hash::Hash;

const N: usize = 4;

#[derive(Debug, Clone)]
pub enum PicoSet<T: Default + Eq + Hash> {
    Inline { buffer: [T; N], len: u32 },
    Heap(HashSet<T>),
}

impl<T: Default + Eq + Hash + Copy, X: AsRef<[T]>> From<X> for PicoSet<T> {
    fn from(set: X) -> PicoSet<T> {
        let mut p = PicoSet::new();
        for x in set.as_ref() {
            p.insert(*x);
        }
        p
    }
}

impl<T: Default + Eq + Hash + Copy> PicoSet<T> {
    pub fn new() -> PicoSet<T> {
        PicoSet::Inline {
            buffer: Default::default(),
            len: 0,
        }
    }

    pub fn insert(&mut self, x: T) {
        if self.contains(&x) {
            return;
        }
        match self {
            PicoSet::Inline { buffer, len } => {
                if *len < N as u32 {
                    buffer[*len as usize] = x;
                    *len += 1;
                } else {
                    let mut set = HashSet::new();
                    for i in 0..*len {
                        set.insert(buffer[i as usize]);
                    }
                    set.insert(x);
                    *self = PicoSet::Heap(set);
                }
            }
            PicoSet::Heap(set) => {
                set.insert(x);
            }
        }
    }

    pub fn remove(&mut self, x: &T) {
        match self {
            PicoSet::Inline { buffer, len } => {
                for i in 0..(*len as usize) {
                    if buffer[i] == *x {
                        for j in i..(*len as usize) - 1 {
                            buffer[j] = buffer[j + 1];
                        }
                        *len -= 1;
                        return;
                    }
                }
            }
            PicoSet::Heap(set) => {
                set.remove(x);
            }
        }
    }

    pub fn contains(&self, x: &T) -> bool {
        match self {
            PicoSet::Inline { buffer, len } => buffer.iter().take(*len as usize).any(|y| y == x),
            PicoSet::Heap(set) => set.contains(x),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            PicoSet::Inline { len, .. } => *len as usize,
            PicoSet::Heap(set) => set.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn from_array(arg: impl AsRef<[T]>) -> PicoSet<T> {
        let mut p = PicoSet::new();
        for x in arg.as_ref() {
            p.insert(*x);
        }
        p
    }

    pub fn iter(&self) -> PicoSetIter<'_, T> {
        match self {
            PicoSet::Inline { buffer, len } => PicoSetIter::Inline {
                set: self,
                index: 0,
            },
            PicoSet::Heap(set) => PicoSetIter::Heap(set.iter()),
        }
    }
}

impl<T: Default + Eq + Hash + Copy> Default for PicoSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum PicoSetIter<'a, T: Default + Eq + Hash + Copy> {
    Inline { set: &'a PicoSet<T>, index: usize },
    Heap(std::collections::hash_set::Iter<'a, T>),
}

impl<'a, T: Default + Eq + Hash + Copy> Iterator for PicoSetIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PicoSetIter::Inline { set, index } => match set {
                PicoSet::Inline { buffer, len } => {
                    if *index < *len as usize {
                        let x = buffer[*index];
                        *index += 1;
                        Some(x)
                    } else {
                        None
                    }
                }
                // should never happen, the original set cannot be changed since we have a reference to it
                _ => panic!("PicoSetIter::Inline: set is not Inline as expected."),
            },
            PicoSetIter::Heap(iter) => iter.next().copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picoset() {
        let mut p = PicoSet::new();
        assert!(matches!(p, PicoSet::Inline { buffer: _, len: 0 }));
        p.insert(1);
        assert!(p.len() == 1);
        assert!(p.contains(&1));
        assert!(p.iter().collect::<Vec<_>>() == vec![1]);
        p.insert(2);
        assert!(p.len() == 2);
        assert!(p.contains(&1));
        assert!(p.contains(&2));
        assert!(p.iter().collect::<Vec<_>>() == vec![1, 2]);
        p.insert(2);
        assert!(p.len() == 2);
        assert!(p.contains(&1));
        assert!(p.contains(&2));
        assert!(p.iter().collect::<Vec<_>>() == vec![1, 2]);
        p.insert(3);
        assert!(p.len() == 3);
        assert!(p.contains(&1));
        assert!(p.contains(&2));
        assert!(p.contains(&3));
        assert!(p.iter().collect::<Vec<_>>() == vec![1, 2, 3]);
        println!("{:?}", p);
        p.remove(&1);
        println!("{:?}", p);
        assert!(p.len() == 2);
        assert!(!p.contains(&1));
        assert!(p.contains(&2));
        assert!(p.contains(&3));
        assert!(p.iter().collect::<Vec<_>>() == vec![2, 3]);
        p.remove(&2);
        assert!(p.len() == 1);
        assert!(!p.contains(&1));
        assert!(!p.contains(&2));
        assert!(p.contains(&3));
        assert!(p.iter().collect::<Vec<_>>() == vec![3]);
        p.remove(&3);
        assert!(p.is_empty());
        assert!(!p.contains(&1));
        assert!(!p.contains(&2));
        assert!(!p.contains(&3));
        assert!(p.iter().collect::<Vec<_>>() == Vec::<usize>::new());
    }

    #[test]
    fn insert_transitions_to_heap() {
        let mut set = PicoSet::<i32>::new();
        for i in 0..(N + 1) {
            set.insert(i as i32);
        }

        assert!(
            matches!(set, PicoSet::Heap(_)),
            "Set did not transition to Heap storage."
        );
        assert_eq!(
            set.len(),
            N + 1,
            "Incorrect length after inserting elements."
        );

        for i in 0..(N + 1) {
            assert!(
                set.contains(&(i as i32)),
                "Set does not contain inserted element: {}",
                i
            );
        }
    }
}
