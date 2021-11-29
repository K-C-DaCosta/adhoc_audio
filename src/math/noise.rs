use std::fmt;

pub struct PseudoRandom {
    sequence: u64,
}
impl PseudoRandom {
    pub fn new(seed: u64) -> Self {
        Self { sequence: seed }
    }

    /// # Description
    /// uniform distrubution of numbers between -1 to 1
    pub fn uniform(&mut self) -> impl Iterator<Item = f32> + '_ {
        (0u64..).map(|_| ((self.rand() & 1023) as f32 / 1024.0) * 2.0 - 1.0)
    }
    /// # Description
    /// a normalized sequence of numbers with tringular distribution
    pub fn triangle(&mut self) -> impl Iterator<Item = f32> + '_ {
        (0u64..).map(|_| {
            //uniform dist range -1..0
            let a = ((self.rand() & 16383) as f32 / 16384.0) * -1.0;
            //uniform dist range 0..1
            let b = ((self.rand() & 16383) as f32 / 16384.0) * 1.0;
            // adding results in triangle distribution
            a + b
        })
    }

    fn rand(&mut self) -> u64 {
        #[allow(dead_code)]
        const A: u64 = 1_000_003;

        #[allow(dead_code)]
        const B: u64 = 314_159;

        #[allow(dead_code)]
        const M: u64 = 507_961;

        self.sequence = (A * self.sequence + B) % M;
        self.sequence
    }
}

pub struct NoiseDistribution<T, const N: usize> {
    values: [T; N],
}

impl<const N: usize> NoiseDistribution<f32, N> {
    pub fn uniform() -> Self {
        let mut values = [0.0f32; N];
        values
            .iter_mut()
            .zip(PseudoRandom::new(123).uniform())
            .for_each(|(v, r)| {
                *v = r;
            });
        Self { values }
    }

    pub fn triangle() -> Self {
        let mut values = [0.0f32; N];
        values
            .iter_mut()
            .zip(PseudoRandom::new(123).triangle())
            .for_each(|(v, r)| {
                *v = r;
            });
        Self { values }
    }
}
impl<const N: usize> fmt::Display for NoiseDistribution<f32, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const INT_COUNT: usize = 20;
        const DELTA: f32 = 1.0 / INT_COUNT as f32;

        let mut frequencies = [0usize; INT_COUNT];
        let total = self.values.len();
        self.values.iter().for_each(|&v| {
            let idx = (((v + 1.0) * 0.5) / DELTA).floor() as usize;
            frequencies[idx.clamp(0, frequencies.len() - 1)] += 1;
        });

        for k in 0..frequencies.len() {
            let stars = (frequencies[k] * 300) / total;
            let idx = k as f32;

            let lbound = idx * DELTA * 2.0 - 1.0;
            let ubound = (idx + 1.0) * DELTA * 2.0 - 1.0;
            write!(f, "[ {:05.2} - {:05.2} ]: ", lbound, ubound)?;
            for _ in 0..stars {
                write!(f, "*")?;
            }
            write!(f, "\n")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "desktop")]
    pub fn sanity() {
        println!("\n\nuniform histogram:\n");
        println!("{}", NoiseDistribution::<f32, 100_000>::uniform());

        println!("triangle histogram:\n");
        println!("{}", NoiseDistribution::<f32, 100_000>::triangle());
    }
}
