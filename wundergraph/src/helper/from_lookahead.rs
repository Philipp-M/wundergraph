use crate::scalar::WundergraphScalarValue;
use juniper::{LookAheadValue, ID};

/// A helper trait marking how to convert a `LookAheadValue` into a specific type
pub trait FromLookAheadValue: Sized {
    /// Try to convert a `LookAheadValue` into a specific type
    ///
    /// For a successful conversion `Some(value)` is returned, otherwise `None`
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self>;
}

impl FromLookAheadValue for i16 {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::SmallInt(ref i)) = *v {
            Some(*i)
        } else {
            None
        }
    }
}

impl FromLookAheadValue for i32 {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        match *v {
            LookAheadValue::Scalar(WundergraphScalarValue::SmallInt(ref i)) => Some(Self::from(*i)),
            LookAheadValue::Scalar(WundergraphScalarValue::Int(ref i)) => Some(*i),
            _ => None,
        }
    }
}

impl FromLookAheadValue for i64 {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        match *v {
            LookAheadValue::Scalar(WundergraphScalarValue::SmallInt(ref i)) => Some(Self::from(*i)),
            LookAheadValue::Scalar(WundergraphScalarValue::Int(ref i)) => Some(Self::from(*i)),
            LookAheadValue::Scalar(WundergraphScalarValue::BigInt(ref i)) => Some(*i),
            _ => None,
        }
    }
}

impl FromLookAheadValue for bool {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::Boolean(ref b)) = *v {
            Some(*b)
        } else {
            None
        }
    }
}

impl FromLookAheadValue for String {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) = *v {
            Some(s.to_owned())
        } else {
            None
        }
    }
}

impl FromLookAheadValue for f32 {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::Float(ref f)) = *v {
            Some(*f)
        } else {
            None
        }
    }
}

impl FromLookAheadValue for f64 {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        match *v {
            LookAheadValue::Scalar(WundergraphScalarValue::Float(ref i)) => Some(Self::from(*i)),
            LookAheadValue::Scalar(WundergraphScalarValue::Double(ref i)) => Some(*i),
            _ => None,
        }
    }
}

impl FromLookAheadValue for ID {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        match *v {
            LookAheadValue::Scalar(WundergraphScalarValue::Int(ref i)) => {
                Some(Self::from(i.to_string()))
            }
            LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) => {
                Some(Self::from(s.to_string()))
            }
            _ => None,
        }
    }
}

impl<T> FromLookAheadValue for Vec<T>
where
    T: FromLookAheadValue,
{
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::List(ref l) = *v {
            l.iter().map(T::from_look_ahead).collect()
        } else {
            None
        }
    }
}

impl<T> FromLookAheadValue for Box<T>
where
    T: FromLookAheadValue,
{
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        T::from_look_ahead(v).map(Box::new)
    }
}

impl<T> FromLookAheadValue for Option<T>
where
    T: FromLookAheadValue,
{
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        Some(T::from_look_ahead(v))
    }
}

#[cfg(feature = "chrono")]
static RFC3339_PARSE_FORMAT: &'static str = "%+";
#[cfg(feature = "chrono")]
static RFC3339_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%.f%:z";


#[cfg(feature = "chrono")]
impl FromLookAheadValue for chrono_internal::NaiveDateTime {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) = *v {
            Self::parse_from_str(s, RFC3339_PARSE_FORMAT).ok()
        } else {
            None
        }
    }
}

#[cfg(feature = "chrono")]
impl FromLookAheadValue for chrono_internal::DateTime<chrono_internal::Utc> {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) = *v {
            s.parse().ok()
        } else {
            None
        }
    }
}

#[cfg(feature = "chrono")]
impl FromLookAheadValue for chrono_internal::DateTime<chrono_internal::FixedOffset> {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) = *v {
            Self::parse_from_rfc3339(s).ok()
        } else {
            None
        }
    }
}

#[cfg(feature = "chrono")]
impl FromLookAheadValue for chrono_internal::NaiveDate {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) = *v {
            Self::parse_from_str(s, RFC3339_FORMAT).ok()
        } else {
            None
        }
    }
}

#[cfg(feature = "uuid")]
impl FromLookAheadValue for uuid_internal::Uuid {
    fn from_look_ahead(v: &LookAheadValue<'_, WundergraphScalarValue>) -> Option<Self> {
        if let LookAheadValue::Scalar(WundergraphScalarValue::String(ref s)) = *v {
            Self::parse_str(s).ok()
        } else {
            None
        }
    }
}
