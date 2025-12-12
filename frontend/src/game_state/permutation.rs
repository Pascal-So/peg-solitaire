#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Permutation<const N: usize> {
    forward: [u8; N],
    backward: [u8; N],
}

impl<const N: usize> Permutation<N> {
    pub fn new() -> Self {
        const { assert!(N < 256, "N must be < 256") };

        Self {
            forward: std::array::from_fn(|i| i as u8),
            backward: std::array::from_fn(|i| i as u8),
        }
    }

    pub fn forward(&self, pos: u8) -> u8 {
        self.forward[pos as usize]
    }

    pub fn backward(&self, pos: u8) -> u8 {
        self.backward[pos as usize]
    }

    /// Swap the values of p(a) and p(b).
    pub fn swap(&mut self, a: u8, b: u8) {
        let pa = self.forward(a);
        let pb = self.forward(b);

        self.forward.swap(a as usize, b as usize);
        self.backward.swap(pa as usize, pb as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{collection::vec, proptest};

    #[test]
    fn test_identity() {
        const N: usize = 5;
        let p = Permutation::<N>::new();
        for i in 0..N as u8 {
            assert_eq!(p.forward(i), i);
            assert_eq!(p.backward(i), i);
        }
    }

    #[test]
    fn test_simple_case() {
        // permutation: (401)(32)
        let mut p = Permutation::<5>::new();
        p.swap(0, 1);
        p.swap(1, 4);
        p.swap(2, 3);

        assert_eq!(p.forward(4), 0);
        assert_eq!(p.forward(0), 1);
        assert_eq!(p.forward(1), 4);
        assert_eq!(p.forward(3), 2);
        assert_eq!(p.forward(2), 3);
    }

    proptest! {
        #[test]
        fn test_backward_inverts_forward(swaps in vec((0u8..20, 0u8..20), 0..123)) {
            const N: usize = 20;
            let mut p = Permutation::<N>::new();
            for (a, b) in swaps {
                p.swap(a, b);
            }
            for i in 0..N as u8 {
                assert_eq!(p.forward(p.backward(i)), i);
                assert_eq!(p.backward(p.forward(i)), i);
            }
        }
    }
}
