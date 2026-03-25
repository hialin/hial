#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EditOp {
    Match(usize, usize),
    Delete(usize),
    Insert(usize),
}

pub(crate) fn diff<T: Eq>(left: &[T], right: &[T]) -> Vec<EditOp> {
    let n = left.len();
    let m = right.len();
    let max = n + m;
    let offset = max as isize + 1;
    let mut v = vec![0isize; 2 * max + 3];
    let mut trace = Vec::with_capacity(max + 1);
    let mut final_d = 0usize;

    'outer: for d in 0..=max {
        for k in (-(d as isize)..=(d as isize)).step_by(2) {
            let idx = (k + offset) as usize;
            let x =
                if k == -(d as isize) || (k != d as isize && v[idx.wrapping_sub(1)] < v[idx + 1]) {
                    v[idx + 1]
                } else {
                    v[idx - 1] + 1
                };
            let mut x = x;
            let mut y = x - k;
            while x < n as isize && y < m as isize && left[x as usize] == right[y as usize] {
                x += 1;
                y += 1;
            }
            v[idx] = x;
            if x >= n as isize && y >= m as isize {
                final_d = d;
                trace.push(v.clone());
                break 'outer;
            }
        }
        trace.push(v.clone());
    }

    backtrack(&trace, final_d, n as isize, m as isize)
}

fn backtrack(trace: &[Vec<isize>], final_d: usize, n: isize, m: isize) -> Vec<EditOp> {
    let max = n + m;
    let offset = max + 1;
    let mut x = n;
    let mut y = m;
    let mut ops = Vec::new();

    for d in (1..=final_d).rev() {
        let v = &trace[d - 1];
        let k = x - y;
        let d_isize = d as isize;
        let prev_k = if k == -d_isize
            || (k != d_isize && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize])
        {
            k + 1
        } else {
            k - 1
        };
        let prev_x = v[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            ops.push(EditOp::Match(x as usize, y as usize));
        }

        if x == prev_x {
            y -= 1;
            ops.push(EditOp::Insert(y as usize));
        } else {
            x -= 1;
            ops.push(EditOp::Delete(x as usize));
        }
    }

    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        ops.push(EditOp::Match(x as usize, y as usize));
    }
    while x > 0 {
        x -= 1;
        ops.push(EditOp::Delete(x as usize));
    }
    while y > 0 {
        y -= 1;
        ops.push(EditOp::Insert(y as usize));
    }

    ops.reverse();
    ops
}

#[cfg(test)]
mod tests {
    use super::{EditOp, diff};

    // TODO: add more tests

    #[test]
    fn aligns_middle_insertion_for_distinct_values() {
        let left = [1, 3];
        let right = [1, 2, 3];

        assert_eq!(
            diff(&left, &right),
            vec![EditOp::Match(0, 0), EditOp::Insert(1), EditOp::Match(1, 2)]
        );
    }

    #[test]
    fn aligns_by_equality_not_position() {
        let left = ["a", "b"];
        let right = ["a", "x", "b"];

        assert_eq!(
            diff(&left, &right),
            vec![EditOp::Match(0, 0), EditOp::Insert(1), EditOp::Match(1, 2)]
        );
    }

    #[test]
    fn reports_replacement_as_delete_plus_insert_when_not_equal() {
        let left = ["a"];
        let right = ["b"];

        assert_eq!(
            diff(&left, &right),
            vec![EditOp::Delete(0), EditOp::Insert(0)]
        );
    }
}
