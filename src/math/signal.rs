#![allow(dead_code)]

pub fn gaussian_filter<const N:usize>(sigma:f32,r:f32)->[f32;N]{
    let mut result = [0f32;N];
    let coef = 1.0/( sigma *  (2.*3.141f32).sqrt()  );
    for k in 0..N{
        let x_scaled =    ( k as f32/ (N-1) as f32  )*2.0*r  - r ;  
        // println!("x={},x_s={}",k,x_scaled);
        let exponent = -0.5*(x_scaled/sigma).powf(2.0);
        result[k] =  coef*exponent.exp();
    }
    result
}

pub fn convolve_1d(samples:&[f32], kernel:&[f32] , result:&mut [f32]){
    let samples_len =  samples.len() as isize; 
    let kernel_len = kernel.len() as isize; 
    for i in 0..samples_len {
        let mut  sum = 0.0;
        for j in 0..kernel_len{
            let idx = j +i -kernel_len/2; 
            let s = if idx < 0 ||  idx >= samples_len { 0.0 } else { samples[idx as usize] }; 
            sum+=s*kernel[j as usize];
        }
        result[i as usize] = sum;
    }
}

pub fn convolve_1d_branchless(samples:&[f32], kernel:&[f32] , result:&mut [f32]){
    let samples_len =  samples.len() as isize; 
    let kernel_len = kernel.len() as isize; 
    for i in 0..samples_len {
        let mut  sum = 0.0;
        for j in 0..kernel_len{
            let idx = j +  (i - (kernel_len>>1)); 
            let idx_out_of_bounds_mask = ((((idx < 0 ||  idx >= samples_len) as u32) << 31) as i32 >> 31) as u32;
            let sample = samples[idx.clamp(0,samples_len-1) as usize];
            let masked_sample = ( idx_out_of_bounds_mask & (0.0f32.to_bits()) ) | 
                                    (!idx_out_of_bounds_mask & (sample.to_bits()) ); 

            sum+= f32::from_bits(masked_sample)  *kernel[j as usize];
        }
        result[i as usize] = sum;
    }
}

pub fn scale_signal<'a>(input:&[f32],dst_len:usize,output:&'a mut [f32])->&'a mut [f32]{
    let src_len = input.len();
    let scale_factor = (src_len-1) as f32/(dst_len-1) as f32 ; 
    for k in 0..dst_len{
        let kf = (k as f32)*scale_factor;
        let w0 = (kf.floor() as usize).min(src_len-1);
        let w1 = (w0+1).min(src_len-1);
        let t = kf.fract();
        output[k] = input[w0]*(1.0-t) + input[w1]*t;
    }
    &mut output[0..dst_len]
}