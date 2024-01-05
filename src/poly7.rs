use bytemuck::{Pod, Zeroable};
use cgmath::Vector2;
use egui_plot::PlotPoints;
use std::array;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Poly7 {
    pub coeffs: [f32; 8],
}

impl Poly7 {
    pub fn new() -> Self {
        Poly7 { coeffs: [0.0; 8] }
    }

    pub fn coeff_names() -> [&'static str; 8] {
        ["h", "g", "f", "e", "d", "c", "b", "a"]
    }

    pub fn invert(&mut self) {
        for c in self.coeffs.iter_mut() {
            *c = -*c;
        }
    }

    pub fn zero() -> Self {
        Poly7 { coeffs: [0.0; 8] }
    }

    pub fn const_val(val: f32) -> Self {
        let mut coeffs = [0.0; 8];
        coeffs[0] = val;
        Poly7 { coeffs }
    }

    pub fn from_points(points: [Vector2<f32>; 8]) -> Option<Self> {
        // check that x coords are different from each other
        // create vandermonde
        let mut m: [[f32; 8]; 8] = std::array::from_fn(|i| {
            let x = points[i].x;
            std::array::from_fn(|j| x.powi(j as i32))
        });
        // invert it
        let inv = inverse(&mut m);
        for col in inv.iter() {
            for elem in col {
                if !elem.is_finite() || elem.is_nan() {
                    return None;
                }
            }
        }
        // multiply with y coords
        let ys = points.map(|p| p.y);
        let coeffs = multiply_vector(&inv, &ys);
        // return poly
        Some(Poly7 { coeffs })
    }

    pub fn eval(&self, x: f32) -> f32 {
        let mut res = 0.0;
        let mut x_to_the_i = 1.0;
        for c in self.coeffs.iter() {
            res += x_to_the_i * c;
            x_to_the_i *= x;
        }
        res
    }

    pub fn plot_points(&self) -> PlotPoints {
        (0..100)
            .map(|x| [x as f64 * 0.01, self.eval(x as f32 * 0.01) as f64])
            .collect()
    }
}

// from and into are row indices
pub fn matrix_row_mul_add<const N: usize>(
    mul: f32,
    m: &mut [[f32; N]; N],
    from: usize,
    into: usize,
) {
    dbg!(from, into);
    print_matrix(m);
    for i in 0..N {
        let fr = m[from][i];
        m[into][i] += mul * fr;
    }
    print_matrix(m)
}

pub fn matrix_row_div<const N: usize>(div: f32, m: &mut [[f32; N]; N], row: usize) {
    for i in 0..N {
        m[row][i] /= div;
    }
}

fn print_matrix<const N: usize>(m: &[[f32; N]; N]) {
    println!("[");
    for line in m.iter() {
        for elem in line.iter() {
            print!("{elem}, ");
        }
        println!();
    }
    println!("]");
}

fn multiply_vector<const N: usize>(m: &[[f32; N]; N], v: &[f32; N]) -> [f32; N] {
    array::from_fn(|row_index| {
        let mut acc = 0.0;
        for i in 0..N {
            acc += m[row_index][i] * v[i];
        }
        acc
    })
}

pub fn inverse<const N: usize>(m: &mut [[f32; N]; N]) -> [[f32; N]; N] {
    // identity
    let mut res = [[0.; N]; N];
    for i in 0..N {
        res[i][i] = 1.0;
    }
    for i in 0..N {
        for j in (i + 1)..N {
            println!("pivot: {}", -m[j][i] / m[i][i]);
            let pivot = -m[j][i] / m[i][i];
            matrix_row_mul_add(pivot, m, i, j);
            matrix_row_mul_add(pivot, &mut res, i, j);
        }
        let div = m[i][i];
        matrix_row_div(div, m, i);
        matrix_row_div(div, &mut res, i);
    }
    // we are now in an upper right triangle configuration
    // now go bottom to top
    for i in (0..N).rev() {
        for j in (0..i).rev() {
            let pivot = -m[j][i] / m[i][i];
            matrix_row_mul_add(pivot, m, i, j);
            matrix_row_mul_add(pivot, &mut res, i, j);
        }
    }
    res
}
