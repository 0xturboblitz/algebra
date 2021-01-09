// The code below is a port of the excellent library of https://github.com/kwantam/fffft by Riad S. Wahby
// to the arkworks APIs

use crate::domain::{radix2::*, DomainCoeff};
use ark_ff::FftField;
use ark_std::vec::Vec;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[derive(PartialEq, Eq, Debug)]
enum FFTOrder {
    /// Both the input and the output of the FFT must be in-order.
    II,
    /// The input of the FFT must be in-order, but the output does not have to be.
    IO,
    /// The input of the FFT can be out of order, but the output must be in-order.
    OI,
}

impl<F: FftField> Radix2EvaluationDomain<F> {
    pub(crate) fn in_order_fft_in_place<T: DomainCoeff<F>>(&self, x_s: &mut [T]) {
        self.fft_helper_in_place(x_s, FFTOrder::II)
    }

    pub(crate) fn in_order_ifft_in_place<T: DomainCoeff<F>>(&self, x_s: &mut [T]) {
        self.ifft_helper_in_place(x_s, FFTOrder::II)
    }

    fn fft_helper_in_place<T: DomainCoeff<F>>(&self, x_s: &mut [T], ord: FFTOrder) {
        use FFTOrder::*;

        let log_len = ark_std::log2(x_s.len());

        if ord == OI {
            self.oi_helper(x_s, self.group_gen);
        } else {
            self.io_helper(x_s, self.group_gen);
        }

        if ord == II {
            derange(x_s, log_len);
        }
    }

    fn ifft_helper_in_place<T: DomainCoeff<F>>(&self, x_s: &mut [T], ord: FFTOrder) {
        use FFTOrder::*;

        let log_len = ark_std::log2(x_s.len());

        if ord == II {
            derange(x_s, log_len);
        }

        if ord == IO {
            self.io_helper(x_s, self.group_gen_inv);
        } else {
            self.oi_helper(x_s, self.group_gen_inv);
        }
        ark_std::cfg_iter_mut!(x_s).for_each(|val| *val *= self.size_inv);
    }

    #[cfg(not(feature = "parallel"))]
    fn roots_of_unity(&self, root: F) -> Vec<F> {
        crate::domain::utils::compute_powers_serial(self.size as usize, root)
    }

    #[cfg(feature = "parallel")]
    fn roots_of_unity(&self, root: F) -> Vec<F> {
        crate::domain::utils::compute_powers(self.size as usize, root)
    }

    fn io_helper<T: DomainCoeff<F>>(&self, xi: &mut [T], root: F) {
        let roots = self.roots_of_unity(root);

        let mut gap = xi.len() / 2;
        while gap > 0 {
            // each butterfly cluster uses 2*gap positions
            let nchunks = xi.len() / (2 * gap);
            ark_std::cfg_chunks_mut!(xi, 2 * gap).for_each(|cxi| {
                let (lo, hi) = cxi.split_at_mut(gap);
                ark_std::cfg_iter_mut!(lo, 1000) // threshold of 1000 was determined empirically
                    .zip(hi)
                    .enumerate()
                    .for_each(|(idx, (lo, hi))| {
                        let neg = *lo - *hi;
                        *lo += *hi;

                        *hi = neg;
                        *hi *= roots[nchunks * idx];
                    });
            });
            gap /= 2;
        }
    }

    fn oi_helper<T: DomainCoeff<F>>(&self, xi: &mut [T], root: F) {
        let roots = self.roots_of_unity(root);

        let mut gap = 1;
        while gap < xi.len() {
            let nchunks = xi.len() / (2 * gap);

            ark_std::cfg_chunks_mut!(xi, 2 * gap).for_each(|cxi| {
                let (lo, hi) = cxi.split_at_mut(gap);
                ark_std::cfg_iter_mut!(lo, 1000) // threshold of 1000 was determined empirically
                    .zip(hi)
                    .enumerate()
                    .for_each(|(idx, (lo, hi))| {
                        *hi *= roots[nchunks * idx];
                        let neg = *lo - *hi;
                        *lo += *hi;
                        *hi = neg;
                    });
            });
            gap *= 2;
        }
    }
}

#[inline]
fn bitrev(a: u64, log_len: u32) -> u64 {
    a.reverse_bits() >> (64 - log_len)
}

fn derange<T>(xi: &mut [T], log_len: u32) {
    for idx in 1..(xi.len() as u64 - 1) {
        let ridx = bitrev(idx, log_len);
        if idx < ridx {
            xi.swap(idx as usize, ridx as usize);
        }
    }
}