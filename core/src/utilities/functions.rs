use std::f64::consts::{LN_2, PI, SQRT_2};

use crate::utilities::{
    cheminfo::lm::ParameterizedFunction,
    math::{erfc_approx, erfcx_approx},
};

/// ## Gaussian
/// Wikipedia https://en.wikipedia.org/wiki/Gaussian_function
/// ### Function
/// f(rt) = h * exp( - (rt-μ)^2 / (2σ^2))
pub fn gaussian_fn(rt: f64, h: f64, mu: f64, sigma: f64) -> f64 {
    let sigma = sigma.max(1e-12);
    let d = rt - mu;
    h * (-(d * d) / (2.0 * sigma * sigma)).exp()
}

/// ## Exponentially modified Gaussian distribution
/// https://en.wikipedia.org/wiki/Exponentially_modified_Gaussian_distribution
/// ### Function
/// f(rt; h, μ, σ, τ) = (h σ / τ) * sqrt(π/2) * exp(0.5*(σ/τ)^2 - (rt - μ)/τ) * erfc( (1/√2) * (σ/τ - (rt - μ)/σ ))
/// ### Evaluation
/// With a = σ/τ, d = (rt - μ)/σ and z = (a - d)/√2, the two forms below are equal and each one is used where
/// none of its factors can overflow (Kalambet et al., J. Chemometrics 25 (2011) 352-356):
/// - z >= 0: exp(-d^2/2) * erfcx(z), both factors at most 1.
/// - z < 0: exp(a^2/2 - a d) * erfc(z), the exponent is below -a^2/2.
pub fn emg_fn(rt: f64, h: f64, mu: f64, sigma: f64, tau: f64) -> f64 {
    let sigma = sigma.max(1e-12);
    let tau = tau.max(1e-12);
    let a = sigma / tau;
    let d = (rt - mu) / sigma;
    let z = (a - d) / SQRT_2;
    let shape = if z >= 0.0 {
        (-0.5 * d * d).exp() * erfcx_approx(z)
    } else {
        (0.5 * a * a - a * d).exp() * erfc_approx(z)
    };
    h * a * (PI / 2.0).sqrt() * shape
}

#[inline]
pub fn sigma_from_fwhm(fwhm: f64) -> f64 {
    // fwhm = 2 * sqrt(2 ln 2) * sigma
    fwhm / (2.0 * (2.0 * LN_2).sqrt())
}

pub fn gaussian(mu: f64) -> Box<ParameterizedFunction> {
    Box::new(move |params: &[f64]| {
        let h = params.first().copied().unwrap_or(1.0);
        let fwhm = params.get(1).copied().unwrap_or(1.0).abs().max(1e-12);
        let sigma = sigma_from_fwhm(fwhm);
        Box::new(move |rt: f64| gaussian_fn(rt, h, mu, sigma))
    })
}

pub fn emg(mu: f64) -> Box<ParameterizedFunction> {
    Box::new(move |params: &[f64]| {
        let h_at_mu = params.first().copied().unwrap_or(1.0);
        let fwhm = params.get(1).copied().unwrap_or(1.0).abs().max(1e-12);
        let tau = params.get(2).copied().unwrap_or(1.0).abs().max(1e-12);

        let sigma = sigma_from_fwhm(fwhm);
        let a = sigma / tau;

        let unit_at_mu = a * (PI / 2.0).sqrt() * erfcx_approx(a / SQRT_2);
        let h_amp = h_at_mu / unit_at_mu.max(1e-24);

        Box::new(move |rt: f64| emg_fn(rt, h_amp, mu, sigma, tau))
    })
}
