use crate::Symbolic;
use common::Type;

#[derive(Clone, PartialEq, Debug)]
pub struct FnData {
    args: Vec<Type>,
    ret_ty: Type,
    is_extern: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct VarData {
    pub ty: Type,
}

#[derive(Clone, PartialEq, Debug)]
pub struct StructData {
    pub fields: Vec<(String, Type)>,
    pub methods: Option<Vec<String>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Symbol {
    pub name: String,
    pub data: AssocData,
}

#[derive(Clone, PartialEq, Debug)]
pub enum AssocData {
    Fn(FnData),
    Var(VarData),
    Struct(StructData),
}

impl Symbol {
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    pub fn ty(&self) -> &Type {
        match &self.data {
            AssocData::Var(v) => &v.ty,
            _ => unreachable!("expected symbol to be a variable"),
        }
    }

    pub fn arg_tys(&self) -> Vec<&Type> {
        match &self.data {
            AssocData::Fn(f) => f.args.iter().collect(),
            _ => unreachable!("expected symbol to be a function"),
        }
    }

    pub fn ret_ty(&self) -> &Type {
        match &self.data {
            AssocData::Fn(f) => &f.ret_ty,
            _ => unreachable!("expected symbol to be a function"),
        }
    }

    pub fn is_extern(&self) -> bool {
        match &self.data {
            AssocData::Fn(f) => f.is_extern,
            _ => unreachable!("expected symbol to be a function"),
        }
    }
}

impl Symbolic for Symbol {
    fn name(&self) -> &str {
        &self.name
    }
}

// For new functions
impl From<(&str, &[Type], &Type, bool)> for Symbol {
    fn from((name, args, ret_ty, is_extern): (&str, &[Type], &Type, bool)) -> Self {
        Symbol {
            name: name.to_owned(),
            data: AssocData::Fn(FnData { args: args.to_owned(), ret_ty: ret_ty.to_owned(), is_extern }),
        }
    }
}

// For new variables
impl From<(&str, &Type)> for Symbol {
    fn from((name, ty): (&str, &Type)) -> Self {
        Symbol { name: name.to_owned(), data: AssocData::Var(VarData { ty: ty.to_owned() }) }
    }
}

impl From<&(String, Type)> for Symbol {
    fn from((name, ty): &(String, Type)) -> Self {
        Symbol { name: name.to_owned(), data: AssocData::Var(VarData { ty: ty.to_owned() }) }
    }
}