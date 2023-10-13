use statrs::distribution::{Normal, ContinuousCDF};
use ndarray::{s, arr3, ArrayView1};

const TAU_MAX_C: [f64; 6] = [2.74, 0.92, 0.55, 0.61, 0.79, 1.0];
const TAU_MIN_C: [f64; 6] = [-18.83, -18.86, -23.48, -28.07, -25.96, -23.27];
const TAU_STAR_C: [f64; 6] = [-1.61, -2.62, -3.13, -3.47, -3.78, -3.93];

const TAU_C_SMALLP: [[f64; 3]; 6] = [
  [2.1659 * 1.0, 1.4412 * 1.0, 3.8269 * 1e-2],
  [2.92 * 1.0, 1.5012 * 1.0, 3.9796 * 1e-2],
  [3.4699 * 1.0, 1.4856 * 1.0, 3.164 * 1e-2],
  [3.9673 * 1.0, 1.4777 * 1.0, 2.6315 * 1e-2],
  [4.5509 * 1.0, 1.5338 * 1.0, 2.9545 * 1e-2],
  [5.1399 * 1.0, 1.6036 * 1.0, 3.4445 * 1e-2]];

const TAU_C_LARGEP: [[f64; 4]; 6] = [
  [1.7339 * 1.0, 9.3202 * 1e-1, -1.2745 * 1e-1, -1.0368 * 1e-2],
  [2.1945 * 1.0, 6.4695 * 1e-1, -2.9198 * 1e-1, -4.2377 * 1e-2],
  [2.5893 * 1.0, 4.5168 * 1e-1, -3.6529 * 1e-1, -5.0074 * 1e-2],
  [3.0387 * 1.0, 4.5452 * 1e-1, -3.3666 * 1e-1, -4.1921 * 1e-2],
  [3.5049 * 1.0, 5.2098 * 1e-1, -2.9158 * 1e-1, -3.3468 * 1e-2],
  [3.9489 * 1.0, 5.8933 * 1e-1, -2.5359 * 1e-1, -2.721 * 1e-2]];

const TAU_C_2010: [[[f64; 4]; 3]; 12] = [
  [[-3.43035, -6.5393, -16.786, -79.433],
   [-2.86154, -2.8903, -4.234, -40.040], 
   [-2.56677, -1.5384, -2.809, 0.0]],
  [[-3.89644, -10.9519, -33.527, 0.0],
   [-3.33613, -6.1101, -6.823, 0.0],
   [-3.04445, -4.2412, -2.720, 0.0]],
  [[-4.29374, -14.4354, -33.195, 47.433],
   [-3.74066, -8.5632, -10.852, 27.982],
   [-3.45218, -6.2143, -3.718, 0.0]],
  [[-4.64332, -18.1031, -37.972, 0.0],
   [-4.09600, -11.2349, -11.175, 0.0],
   [-3.81020, -8.3931, -4.137, 0.0]],
  [[-4.95756, -21.8883, -45.142, 0.0],
   [-4.41519, -14.0405, -12.575, 0.0],
   [-4.13157, -10.7417, -3.784, 0.0]],
  [[-5.24568, -25.6688, -57.737, 88.639],
   [-4.70693, -16.9178, -17.492, 60.007],
   [-4.42501, -13.1875, -5.104, 27.877]],
  [[-5.51233, -29.5760, -69.398, 164.295],
   [-4.97684, -19.9021, -22.045, 110.761],
   [-4.69648, -15.7315, -5.104, 27.877]],
  [[-5.76202, -33.5258, -82.189, 256.289],
   [-5.22924, -23.0023, -24.646, 144.479],
   [-4.95007, -18.3959, -7.344, 94.872]],
  [[-5.99742, -37.6572, -87.365, 248.316],
   [-5.46697, -26.2057, -26.627, 176.382],
   [-5.18897, -21.1377, -9.484, 172.704]],
  [[-6.22103, -41.7154, -102.680, 389.33],
   [-5.69244, -29.4521, -30.994, 251.016],
   [-5.41533, -24.0006, -7.514, 163.049]],
  [[-6.43377, -46.0084, -106.809, 352.752],
   [-5.90714, -32.8336, -30.275, 249.994],
   [-5.63086, -26.9693, -4.083, 151.427]],
  [[-6.63790, -50.2095, -124.156, 579.622],
   [-6.11279, -36.2681, -32.505, 314.802],
   [-5.83724, -29.9864, -2.686, 184.116]]];


/// Polyval calculation for p-value
fn polyval(p: &[f64], x: f64) -> f64 {
  let mut res = 0.0;
  for &coeff in p {
    res = res * x + coeff;
  }
  res
}

/// CDF Calculation
fn norm_cdf(x: f64) -> f64 {
  let normal = Normal::new(0.0, 1.0).unwrap();
  normal.cdf(x)
}

/// P Value calculation using MacKinnon
// Inspired by https://github.com/statsmodels/statsmodels/blob/3b61c469ed8d4a6752b5bf01390789512f81f0c6/statsmodels/tsa/adfvalues.py#L407
pub fn p_value_mackinnon_cointegration(t_stat: f64) -> f64 {
  let maxstat: [f64; 6] = TAU_MAX_C;
  let minstat: [f64; 6] = TAU_MIN_C;
  let starstat: [f64; 6] = TAU_STAR_C;

  let n: usize = 2;

  if t_stat > maxstat[n-1] {
    return 1.0;
  } else if t_stat < minstat[n-1]{
    return 0.0;
  }

  let tau_coef: Vec<f64> = if t_stat <= starstat[n-1] {
    TAU_C_SMALLP[n-1].iter().rev().copied().collect::<Vec<f64>>()
  } else {
    TAU_C_LARGEP[n-1].iter().rev().copied().collect::<Vec<f64>>()
  };
  norm_cdf(polyval(&tau_coef, t_stat))
}


/// Critical Value calculation using MacKinnon
// Inspired by https://github.com/statsmodels/statsmodels/blob/3b61c469ed8d4a6752b5bf01390789512f81f0c6/statsmodels/tsa/adfvalues.py#L407
pub fn critical_values_mackinnon_cointegration() -> (f64, f64, f64) {
  let n: usize = 1 - 1;

  // Calculate the result
  let tau_c_2010: ndarray::ArrayBase<ndarray::OwnedRepr<f64>, ndarray::Dim<[usize; 3]>> = arr3(&TAU_C_2010);

  let crit_values: ArrayView1<_> = tau_c_2010.slice(s![n, .., 0]);
  (crit_values.to_vec()[0], crit_values.to_vec()[1], crit_values.to_vec()[2])
}
