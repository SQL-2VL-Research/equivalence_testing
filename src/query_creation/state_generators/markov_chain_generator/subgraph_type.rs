use std::fmt::Display;

use sqlparser::ast::{DataType, ExactNumberInfo};

use crate::unwrap_variant;

use super::error::SyntaxError;

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum SubgraphType {
    /// This type is used if the type is yet to be determined
    Undetermined,
    Numeric,
    Val3,
    ListExpr(Box<SubgraphType>),
    Text,
    Date,
}

impl SubgraphType {
    pub fn from_type_name(s: &str) -> Result<Self, SyntaxError> {
        match s {
            "numeric" => Ok(SubgraphType::Numeric),
            "3VL Value" => Ok(SubgraphType::Val3),
            "list expr" => Ok(SubgraphType::ListExpr(Box::new(SubgraphType::Undetermined))),
            "text" => Ok(SubgraphType::Text),
            "date" => Ok(SubgraphType::Date),
            any => Err(SyntaxError::new(format!("Type {any} does not exist!")))
        }
    }

    pub fn has_inner(&self) -> bool {
        match self {
            SubgraphType::ListExpr(..) => true,
            _ => false,
        }
    }

    pub fn inner(&self) -> SubgraphType {
        match self {
            SubgraphType::ListExpr(inner) => *inner.clone(),
            any => panic!("{any} has no inner type"),
        }
    }

    pub fn from_data_type(data_type: &DataType) -> Self {
        match data_type {
            DataType::Integer(_) => Self::Numeric,  /// TODO
            DataType::Varchar(_) => Self::Text,
            DataType::CharVarying(_) => Self::Text,
            DataType::Char(_) => Self::Text,
            DataType::Text => Self::Text,
            DataType::Numeric(_) => Self::Numeric,
            DataType::Date => Self::Date,
            DataType::Boolean => Self::Val3,
            any => panic!("DataType not implemented: {any}"),
        }
    }

    pub fn is_determined(&self) -> bool {
        match self {
            SubgraphType::Undetermined => false,
            SubgraphType::ListExpr(inner) => inner.is_determined(),
            _ => true
        }
    }

    /// Used only for null type casting. Fails if encounters "Undetermined"
    pub fn to_data_type(&self) -> DataType {
        match self {
            SubgraphType::Numeric => DataType::Numeric(ExactNumberInfo::None),
            SubgraphType::Val3 => DataType::Boolean,
            SubgraphType::Text => DataType::Text,
            SubgraphType::Date => DataType::Date,
            any => panic!("Can't convert {any} to DataType"),
        }
    }
}

impl Display for SubgraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            SubgraphType::Numeric => "numeric".to_string(),
            SubgraphType::Val3 => "3VL Value".to_string(),
            SubgraphType::ListExpr(inner) => format!("list expr[{}]", inner),
            SubgraphType::Text => "text".to_string(),
            SubgraphType::Undetermined => "undetermined".to_string(),
            SubgraphType::Date => "date".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl SubgraphType {
    /// get a list of compatible types
    pub fn get_compat_types(&self) -> Vec<SubgraphType> {
        let (inner_type, wrapper): (&Box<SubgraphType>, Box<dyn Fn(SubgraphType) -> SubgraphType>) = match self {
            SubgraphType::ListExpr(inner) => (inner, Box::new(|x| SubgraphType::ListExpr(Box::new(x)))),
            any => return vec![any.clone()],
        };
        inner_type.get_compat_types()
            .into_iter()
            .map(wrapper)
            .collect()
    }

    /// checks is self is convertable to other
    pub fn is_compat_with(&self, other: &SubgraphType) -> bool {
        other.is_same_or_more_determined_or_undetermined(self) ||
        self.is_same_or_more_determined_or_undetermined(other) ||
        self.get_compat_types().iter().any(|x| other.is_same_or_more_determined_or_undetermined(x)) ||
        other.get_compat_types().iter().any(|x| self.is_same_or_more_determined_or_undetermined(x))
    }

    /// returns whether type is same, more determined or undetermined at all (NULL)
    pub fn is_same_or_more_determined_or_undetermined(&self, as_what: &SubgraphType) -> bool {
        if *self == SubgraphType::Undetermined {
            return true;
        }
        if *as_what == SubgraphType::Undetermined {
            return true;
        }
        match self {
            SubgraphType::ListExpr(inner) => {
                if matches!(as_what, SubgraphType::ListExpr(..)) {
                    let other_inner = unwrap_variant!(as_what, SubgraphType::ListExpr);
                    **other_inner == SubgraphType::Undetermined || inner.is_same_or_more_determined_or_undetermined(&other_inner)
                } else {
                    false
                }
            },
            any => any == as_what,
        }
    }
}