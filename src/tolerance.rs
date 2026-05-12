use crate::{BookingTypes, Number, Tolerance, ToleranceCurrency, ToleranceNumber};

// Beancount Precision & Tolerances
// https://docs.google.com/document/d/1lgHxUUEY-UVEgoF6cupz2f_7v7vEF7fiJyiSlYYlhOo
pub(crate) fn tolerance_residual<B, T>(
    tol: &T,
    values: impl Iterator<Item = B::Number>,
    cur: &B::Currency,
) -> Option<B::Number>
where
    B: BookingTypes,
    T: Tolerance<Types = B>,
{
    // TODO don't iterate twice over values
    let values = values.collect::<Vec<_>>();
    let values = values.into_iter();

    let multiplier = tol
        .inferred_tolerance_multiplier()
        .unwrap_or(default_inferred_tolerance_multiplier::<B>());
    let s = values.collect::<SumWithMinNonZeroScale<B>>();
    let residual = s.sum;
    let abs_residual = residual.abs();

    if let Some(min_nonzero_scale) = s.min_nonzero_scale.as_ref() {
        (abs_residual > B::Number::new(1, *min_nonzero_scale) * multiplier).then_some(residual)
    } else {
        let tolerance = tol.inferred_tolerance_default(cur);

        if let Some(tolerance) = tolerance {
            (abs_residual > tolerance).then_some(residual)
        } else {
            (residual != B::Number::zero()).then_some(residual)
        }
    }
}

#[derive(Clone, Debug)]
struct SumWithMinNonZeroScale<B>
where
    B: BookingTypes,
{
    sum: B::Number,
    min_nonzero_scale: Option<u32>,
}

impl<B> FromIterator<B::Number> for SumWithMinNonZeroScale<B>
where
    B: BookingTypes,
{
    fn from_iter<T: IntoIterator<Item = B::Number>>(iter: T) -> Self {
        let mut sum = B::Number::zero();
        let mut min_nonzero_scale = None;
        for value in iter {
            sum += value;
            if value.scale() > 0 {
                if min_nonzero_scale.is_none() {
                    min_nonzero_scale = Some(value.scale());
                } else if let Some(scale) = min_nonzero_scale
                    && value.scale() < scale
                {
                    min_nonzero_scale = Some(value.scale());
                }
            }
        }

        Self {
            sum,
            min_nonzero_scale,
        }
    }
}

pub(crate) fn default_inferred_tolerance_multiplier<B>() -> B::Number
where
    B: BookingTypes,
{
    B::Number::new(5, 1) // 0.5
}

/// A tolerance wrapper that overrides the inferred tolerance multiplier,
/// used to apply cost-inferred widening without modifying the base tolerance.
#[derive(Clone, Debug)]
pub(crate) struct WithMultiplier<T: Tolerance> {
    pub(crate) inner: T,
    pub(crate) multiplier: ToleranceNumber<T>,
}

impl<T: Tolerance> Tolerance for WithMultiplier<T> {
    type Types = T::Types;

    fn inferred_tolerance_default(
        &self,
        cur: &ToleranceCurrency<Self>,
    ) -> Option<ToleranceNumber<Self>> {
        self.inner.inferred_tolerance_default(cur)
    }

    fn inferred_tolerance_multiplier(&self) -> Option<ToleranceNumber<Self>> {
        Some(self.multiplier)
    }
}
