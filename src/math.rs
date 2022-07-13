pub mod signal;
pub mod parabola;
pub mod noise; 


pub use signal::*; 
pub use parabola::*; 
pub use noise::*; 

#[allow(dead_code)]
pub fn compute_mse(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len()) as f32;
    a.iter()
        .zip(b.iter())
        .fold(0.0, |acc, (y, y_est)| acc + (y - y_est).powf(2.0))
        / n
}
