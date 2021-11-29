
/// A parabola sampled on the points: \
///  ( 0 , `f[0]` ) , ( 1 , `f[1]` ), ( 2, `f[2]` )
#[derive(Copy, Clone)]
pub struct FixedParabola {
    pub f: [f32; 3],
    /// coefs for the equation: `coefs[2]*x^2 + coefs[1]*x + coefs[0]`
    pub coefs: [f32; 3],
}
impl FixedParabola {
    pub fn new() -> Self {
        Self {
            f: [0f32; 3],
            coefs: [0f32; 3],
        }
    }

    pub fn from_samples(f: [f32; 3]) -> Self {
        Self {
            f,
            coefs: [0f32; 3],
        }
    }

    pub fn compute_coefs(&mut self) {
        let f = &self.f;
        let coefs = &mut self.coefs;
        let d = f[1] - f[0];
        let e = f[2] - f[0];
        coefs[0] = f[0];
        coefs[1] = (4.0 * d - e) * 0.5;
        coefs[2] = (e - 2.0 * d) * 0.5;
    }

    pub fn eval(&self, x: f32) -> f32 {
        let coefs = &self.coefs;
        coefs[0] + (coefs[1] + coefs[2] * x) * x
    }
}




mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn fixed_parabolla() {
        let mut p = FixedParabola::from_samples([10f32, 12.0, 15.0]);
        p.compute_coefs();

        println!("f({}) = {}", 0.0, p.eval(0.0));
        println!("f({}) = {}", 1.0, p.eval(1.0));
        println!("f({}) = {}", 2.0, p.eval(2.0));
        println!("f({}) = {}", 3.0, p.eval(3.0));

        (0..p.f.len())
            .map(|x| x as f32)
            .zip(p.f.iter())
            .for_each(|(x, &f_expected)| {
                let f_eval = p.eval(x);
                assert_eq!(
                    (f_eval - f_expected).abs() < 0.001,
                    true,
                    "accuracy threshold not met"
                )
            });
    }
}