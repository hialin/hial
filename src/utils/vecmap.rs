use crate::guard_some;
use core::hash::Hash;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct VecMap<K, V>
where
    K: Eq + Hash + Clone,
{
    vec: Vec<(K, V)>,
    map: HashMap<K, usize>,
}

impl<K, V> VecMap<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> VecMap<K, V> {
        VecMap {
            vec: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn at(&self, pos: usize) -> Option<(&K, &V)> {
        let x = guard_some!(self.vec.get(pos), { return None });
        Some((&x.0, &x.1))
    }

    pub fn at_mut(&mut self, pos: usize) -> Option<(&K, &mut V)> {
        let x = guard_some!(self.vec.get_mut(pos), { return None });
        Some((&x.0, &mut x.1))
    }

    pub fn iter(&self) -> VecMapIter<'_, '_, K, V, K> {
        VecMapIter {
            map: self,
            pos: 0,
            key: None,
        }
    }

    pub fn put(&mut self, key: K, value: V) -> usize {
        let pos = self.vec.len();
        self.map.insert(key.clone(), pos);
        self.vec.push((key, value));
        pos
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<(usize, &K, &V)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        let pos = *guard_some!(self.map.get(key), { return None });
        let x = guard_some!(self.vec.get(pos), { return None });
        Some((pos, &x.0, &x.1))
    }

    pub fn get_all<'q, Q>(&self, key: &'q Q) -> VecMapIter<'_, 'q, K, V, Q>
    where
        K: std::borrow::Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        VecMapIter {
            map: self,
            pos: 0,
            key: Some(key),
        }
    }

    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        let pos = *guard_some!(self.map.get(key), { return None });
        let x = guard_some!(self.vec.get_mut(pos), { return None });
        Some(&mut x.1)
    }
}

#[derive(Debug)]
pub struct VecMapIter<'a, 'q, K, V, Q>
where
    K: Eq + Hash + Clone,
    K: std::borrow::Borrow<Q>,
    Q: ?Sized + Hash + Eq,
{
    map: &'a VecMap<K, V>,
    pos: usize,
    key: Option<&'q Q>,
}

impl<'a, 'q, K, V, Q> Iterator for VecMapIter<'a, 'q, K, V, Q>
where
    K: Eq + Hash + Clone,
    K: std::borrow::Borrow<Q>,
    Q: ?Sized + Hash + Eq,
{
    type Item = (usize, &'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.pos += 1;
            match self.map.vec.get(self.pos - 1) {
                Some((k, v)) => match self.key {
                    Some(q) => {
                        if k.borrow() == q {
                            return Some((self.pos - 1, k, v));
                        }
                    }
                    None => return Some((self.pos - 1, k, v)),
                },
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Res;

    #[test]
    fn test() -> Res<()> {
        let mut v: VecMap<String, isize> = VecMap::new();
        let a = "a".to_string();
        let b = "b".to_string();
        assert_eq!(v.len(), 0);
        assert_eq!(v.get("1"), None);
        assert_eq!(v.at(0), None);

        v.put(a.clone(), 100);
        assert_eq!(v.len(), 1);
        assert_eq!(v.at(1), None);
        assert_eq!(v.at(0), Some((&a, &100)));
        assert_eq!(v.get("1"), None);
        assert_eq!(v.get("a"), Some((0, &a, &100)));
        assert_eq!(v.get("b"), None);

        v.put(b.clone(), 200);
        assert_eq!(v.len(), 2);
        assert_eq!(v.at(2), None);
        assert_eq!(v.at(0), Some((&a, &100)));
        assert_eq!(v.at(1), Some((&b, &200)));
        assert_eq!(v.get("1"), None);
        assert_eq!(v.get("a"), Some((0, &a, &100)));
        assert_eq!(v.get("b"), Some((1, &b, &200)));

        let w: Vec<_> = v.iter().collect();
        assert_eq!(w.len(), 2);
        assert_eq!(w[0], (0, &a, &100));
        assert_eq!(w[1], (1, &b, &200));

        Ok(())
    }
}
