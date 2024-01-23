import numpy as np
from scipy.stats import linregress

class SmartError(Exception):
    pass

def half_life_mean_reversion(series):
  if len(series) <= 1:
    raise SmartError("Series length must be greater than 1.")
  difference = np.diff(series)
  lagged_series = series[:-1]
  slope, _, _, _, _ = linregress(lagged_series, difference)
  if np.abs(slope) < np.finfo(float).eps:
    raise SmartError("Cannot calculate half life. Slope value is too close to zero.")
  half_life = -np.log(2) / slope
  return half_life

def calculate_cointegration(series_1, series_2):
  series_1 = np.array(series_1).astype(np.float)
  series_2 = np.array(series_2).astype(np.float)
  coint_flag = 0
  coint_res = coint(series_1, series_2)
  coint_t = coint_res[0]
  p_value = coint_res[1]
  critical_value = coint_res[2][1]

  # different
  series_2_with_constant = sm.add_constant(series_2) # Better way to fit data for this purpose
  model = sm.OLS(series_1, series_2_with_constant).fit()
  hedge_ratio = model.params[1] # use [1] instead of [0] as now fitting the model differently above
  intercept = model.params[0]

  spread = series_1 - (series_2 * hedge_ratio) - intercept
  half_life = half_life_mean_reversion(spread)
  t_check = coint_t < critical_value
  coint_flag = 1 if p_value < 0.05 and t_check else 0
  return coint_flag, hedge_ratio, half_life