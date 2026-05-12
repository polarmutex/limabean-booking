use std::{borrow::Cow, collections::HashMap, marker::PhantomData};

use super::{Booking, BookingTypes, CostSpec, PostingSpec, PriceSpec, Tolerance};
use beancount_parser_lima as parser;
use rust_decimal::Decimal;
use time::Date;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct LimaParserBookingTypes<'a>(PhantomData<&'a str>);

impl<'a> BookingTypes for LimaParserBookingTypes<'a> {
    type Account = &'a str;
    type Date = time::Date;
    type Currency = &'a str;
    type Number = Decimal;
    type Label = Cow<'a, str>;
}

impl<'a> PostingSpec for parser::Spanned<parser::Posting<'a>> {
    type Types = LimaParserBookingTypes<'a>;
    type CostSpec = parser::CostSpec<'a>;
    type PriceSpec = parser::PriceSpec<'a>;

    fn account(&self) -> &'a str {
        parser::Posting::account(self).item().into()
    }

    fn currency(&self) -> Option<&'a str> {
        parser::Posting::currency(self).map(|cur| cur.item().into())
    }

    fn units(&self) -> Option<Decimal> {
        parser::Posting::amount(self).map(|amount| amount.item().value())
    }

    fn cost(&self) -> Option<&Self::CostSpec> {
        self.cost_spec().as_ref().map(|cost_spec| cost_spec.item())
    }

    fn price(&self) -> Option<&Self::PriceSpec> {
        self.price_annotation()
            .as_ref()
            .map(|cost_spec| cost_spec.item())
    }
}

impl<'a> CostSpec for parser::CostSpec<'a> {
    type Types = LimaParserBookingTypes<'a>;

    fn currency(&self) -> Option<&'a str> {
        parser::CostSpec::currency(self).map(|currency| currency.item().into())
    }

    fn per_unit(&self) -> Option<Decimal> {
        parser::CostSpec::per_unit(self).map(|per_unit| per_unit.value())
    }

    fn total(&self) -> Option<Decimal> {
        parser::CostSpec::total(self).map(|total| total.value())
    }

    fn date(&self) -> Option<Date> {
        parser::CostSpec::date(self).map(|date| *date.item())
    }

    fn label(&self) -> Option<Cow<'a, str>> {
        parser::CostSpec::label(self).map(|label| Cow::Borrowed(*label.item()))
    }

    fn merge(&self) -> bool {
        parser::CostSpec::merge(self)
    }
}

impl<'a> PriceSpec for parser::PriceSpec<'a> {
    type Types = LimaParserBookingTypes<'a>;

    fn currency(&self) -> Option<&'a str> {
        use parser::PriceSpec::*;

        match self {
            BareCurrency(currency) => Some(currency.into()),
            CurrencyAmount(_, currency) => Some(currency.into()),
            _ => None,
        }
    }

    fn per_unit(&self) -> Option<Decimal> {
        use parser::PriceSpec::*;
        use parser::ScopedExprValue::*;

        match self {
            BareAmount(PerUnit(expr)) => Some(expr.value()),
            CurrencyAmount(PerUnit(expr), _) => Some(expr.value()),
            _ => None,
        }
    }

    fn total(&self) -> Option<Decimal> {
        use parser::PriceSpec::*;
        use parser::ScopedExprValue::*;

        match self {
            BareAmount(Total(expr)) => Some(expr.value()),
            CurrencyAmount(Total(expr), _) => Some(expr.value()),
            _ => None,
        }
    }
}

impl<'a> BookingTypes for &parser::Options<'a> {
    type Account = &'a str;
    type Date = time::Date;
    type Currency = &'a str;
    type Number = Decimal;
    type Label = &'a str;
}

/// tolerance converted from parser options allowing for
/// currency lookup by string
#[derive(Clone, Debug)]
pub struct LimaTolerance<'a> {
    default: HashMap<&'a str, Decimal>,
    default_fallback: Option<Decimal>,
    multiplier: Option<Decimal>,
    infer_tolerance_from_cost: bool,
}

impl<'a> From<&parser::Options<'a>> for LimaTolerance<'a> {
    fn from(value: &parser::Options<'a>) -> Self {
        let mut default = HashMap::default();
        let mut default_fallback = None;
        let multiplier = value.inferred_tolerance_multiplier().map(|x| *x.item());
        let infer_tolerance_from_cost = value
            .infer_tolerance_from_cost()
            .map(|x| *x.item())
            .unwrap_or(false);

        for (cur, tol) in value.inferred_tolerance_defaults() {
            if let Some(cur) = cur {
                default.insert(cur.into(), tol);
            } else {
                default_fallback = Some(tol);
            }
        }

        LimaTolerance {
            default,
            default_fallback,
            multiplier,
            infer_tolerance_from_cost,
        }
    }
}

impl<'a> Tolerance for LimaTolerance<'a> {
    type Types = LimaParserBookingTypes<'a>;

    fn inferred_tolerance_default(
        &self,
        currency: &<Self::Types as BookingTypes>::Currency,
    ) -> Option<<Self::Types as BookingTypes>::Number> {
        self.default
            .get(currency)
            .copied()
            .or(self.default_fallback)
    }

    fn inferred_tolerance_multiplier(&self) -> Option<<Self::Types as BookingTypes>::Number> {
        self.multiplier
    }

    fn infer_tolerance_from_cost(&self) -> bool {
        self.infer_tolerance_from_cost
    }
}

impl From<parser::Booking> for Booking {
    fn from(value: parser::Booking) -> Self {
        use Booking::*;
        use parser::Booking as parser;

        match value {
            parser::Strict => Strict,
            parser::StrictWithSize => StrictWithSize,
            parser::None => None,
            parser::Average => Average,
            parser::Fifo => Fifo,
            parser::Lifo => Lifo,
            parser::Hifo => Hifo,
        }
    }
}
