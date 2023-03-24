use std::collections::HashMap;

use smol_str::SmolStr;
use rand_chacha::ChaCha8Rng;
use rand::{SeedableRng, Rng};
use sqlparser::ast::{Ident, ObjectName};

#[derive(Debug, Clone)]
#[derive(Eq, Hash, PartialEq)]
pub struct Relation {
    name: SmolStr,
    free_column_name_index: u32,
}

impl Relation {
    fn with_name(name: SmolStr) -> Self {
        Self {
            name, free_column_name_index: 1
        }
    }

    pub fn gen_ident(&self) -> Ident {
        Ident { value: self.name.to_string(), quote_style: None }
    }

    pub fn gen_object_name(&self) -> ObjectName {
        ObjectName(vec![self.gen_ident()])
    }

    pub fn gen_column_ident(&mut self) -> Ident {
        let name = format!("C{}", self.free_column_name_index);
        self.free_column_name_index += 1;
        Ident { value: name.clone(), quote_style: None }
    }
}

pub struct RelationManager {
    relations: HashMap<SmolStr, Relation>,
    free_relation_name_index: u32,
    rng: ChaCha8Rng,
    columns_in_grouping: HashMap<Relation, Vec<Ident>>,
}

impl RelationManager {
    pub fn new() -> Self {
        Self {
            relations: HashMap::<_, _>::new(),
            free_relation_name_index: 1,
            rng: ChaCha8Rng::seed_from_u64(1),
            columns_in_grouping: HashMap::<_, _>::new(),
        }
    }

    fn new_name(&mut self) -> SmolStr {
        let name = SmolStr::new(format!("T{}", self.free_relation_name_index));
        self.free_relation_name_index += 1;
        name
    }

    pub fn new_relation(&mut self) -> &mut Relation {
        let name = self.new_name();
        let relation = Relation::with_name(name.clone());
        self.relations.insert(name.clone(), relation);
        self.relations.get_mut(&name).unwrap()
    }

    pub fn get_random_relation(&mut self) -> &mut Relation {
        let num_skip = self.rng.gen_range(0..self.relations.len());
        let key = self.relations.iter().skip(num_skip).next().unwrap().0.to_owned();
        self.relations.get_mut(&key).unwrap()
    }

    pub fn new_ident(&mut self) -> Ident {
        Ident::new(self.new_name())
    }

    pub fn push_grouping_ident(&mut self, relation: Relation, ident: Ident) {
        //TODO push to vector instead of inserting
        let k = relation.clone().to_owned();
        let v = vec![ident.clone()].to_owned();
        self.columns_in_grouping.insert(k, v);
    }
}

#[derive(Debug, Clone)]
pub enum TypesSelectedType {
    Numeric, Val3, Array, ListExpr, String, Bool, Any
}

impl PartialEq for TypesSelectedType {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other) ||
        core::mem::discriminant(self) == core::mem::discriminant(&Self::Any) ||
        core::mem::discriminant(other) == core::mem::discriminant(&Self::Any)
    }
}

impl TypesSelectedType {
    fn to_smolstr(&self) -> SmolStr {
        match self {
            TypesSelectedType::Numeric => SmolStr::new("numeric"),
            TypesSelectedType::Val3 => SmolStr::new("3VL Value"),
            TypesSelectedType::Array => SmolStr::new("array"),
            TypesSelectedType::ListExpr => SmolStr::new("list expr"),
            TypesSelectedType::String => SmolStr::new("string"),
            TypesSelectedType::Bool => SmolStr::new("bool"),
            TypesSelectedType::Any => SmolStr::new("any"),
        }
    }

    pub fn get_types(&self) -> Vec<SmolStr> {
        match self {
            TypesSelectedType::Any => vec![
                (TypesSelectedType::String).to_smolstr(),
                (TypesSelectedType::ListExpr).to_smolstr(),
                (TypesSelectedType::Array).to_smolstr(),
                (TypesSelectedType::Val3).to_smolstr(),
                (TypesSelectedType::Numeric).to_smolstr(),
                (TypesSelectedType::Bool).to_smolstr(),
            ],
            _ => vec![self.to_smolstr()],
        }
    }

    /// get a list of compatible types
    pub fn get_compat_types(&self) -> Vec<SmolStr> {
        self.get_types()
    }

    pub fn is_compat_with(&self, other: &TypesSelectedType) -> bool {
        *other == TypesSelectedType::Any || *self == TypesSelectedType::Any ||
        self.get_compat_types().contains(&other.to_smolstr())
    }
}