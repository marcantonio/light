use serde::{Deserialize, Serialize};
use std::fmt::Display;

use super::Symbolic;
use crate::Type;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FnData {
    fq_name: String,
    args: Vec<(String, Type)>,
    ret_ty: Type,
    is_extern: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct VarData {
    pub ty: Type,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StructData {
    pub fields: Option<Vec<(String, String)>>,
    pub methods: Option<Vec<String>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AssocData {
    Fn(FnData),
    Var(VarData),
    Struct(StructData),
    Module(String),
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub data: AssocData,
    pub module: String,
    pub is_exportable: bool,
}

impl Symbol {
    pub fn new_fn(
        name: &str, fq_name: &str, args: &[(String, Type)], ret_ty: &Type, is_extern: bool, module: &str,
        is_exportable: bool,
    ) -> Self {
        Symbol {
            name: name.to_owned(),
            data: AssocData::Fn(FnData {
                fq_name: fq_name.to_owned(),
                args: args.to_vec(),
                ret_ty: ret_ty.to_owned(),
                is_extern,
            }),
            module: module.to_owned(),
            is_exportable,
        }
    }

    pub fn new_var(name: &str, ty: &Type, module: &str) -> Self {
        Symbol {
            name: name.to_owned(),
            data: AssocData::Var(VarData { ty: ty.to_owned() }),
            module: module.to_owned(),
            is_exportable: false,
        }
    }

    pub fn new_struct(
        name: &str, fields: Option<&[(String, String)]>, methods: Option<&[String]>, module: &str,
        is_exportable: bool,
    ) -> Self {
        Symbol {
            name: name.to_owned(),
            data: AssocData::Struct(StructData {
                fields: fields.map(|x| x.to_vec()),
                methods: methods.map(|x| x.to_vec()),
            }),
            module: module.to_owned(),
            is_exportable,
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    pub fn ty(&self) -> &Type {
        match &self.data {
            AssocData::Var(s) => &s.ty,
            _ => unreachable!("expected symbol to be a variable"),
        }
    }

    pub fn fq_name(&self) -> Option<&str> {
        match &self.data {
            AssocData::Fn(data) => Some(&data.fq_name),
            AssocData::Struct(_) => Some(&self.name),
            _ => None,
        }
    }

    pub fn args(&self) -> Vec<(&str, &Type)> {
        match &self.data {
            AssocData::Fn(s) => s.args.iter().map(|(a, ty)| (a.as_str(), ty)).collect(),
            _ => unreachable!("expected symbol to be a function"),
        }
    }

    pub fn arg_tys(&self) -> Vec<&Type> {
        match &self.data {
            AssocData::Fn(s) => s.args.iter().map(|(_, ty)| ty).collect(),
            _ => unreachable!("expected symbol to be a function"),
        }
    }

    pub fn ret_ty(&self) -> &Type {
        match &self.data {
            AssocData::Fn(s) => &s.ret_ty,
            _ => unreachable!("expected symbol to be a function"),
        }
    }

    pub fn is_extern(&self) -> bool {
        match &self.data {
            AssocData::Fn(s) => s.is_extern,
            _ => unreachable!("expected symbol to be a function"),
        }
    }

    pub fn fields(&self) -> Option<Vec<(&str, &str)>> {
        match &self.data {
            AssocData::Struct(s) => {
                Some(s.fields.as_deref()?.iter().map(|(n, a)| (n.as_str(), a.as_str())).collect())
            },
            _ => unreachable!("expected symbol to be a struct"),
        }
    }

    pub fn methods(&self) -> Option<Vec<&str>> {
        match &self.data {
            AssocData::Struct(s) => Some(s.methods.as_deref()?.iter().map(|m| m.as_str()).collect()),
            _ => unreachable!("expected symbol to be a struct"),
        }
    }

    pub fn is_import(&self, module: &str) -> bool {
        self.module != module && !self.is_extern()
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Symbolic for Symbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &str {
        match self.data {
            AssocData::Fn(_) => "Fn",
            AssocData::Var(_) => "Var",
            AssocData::Struct(_) => "Struct",
            AssocData::Module(_) => "Module",
        }
    }

    fn is_exportable(&self) -> bool {
        self.is_exportable
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output =
            format!("name: {}, module: {}, exportable: {}", self.name, self.module, self.is_exportable);
        match &self.data {
            AssocData::Fn(FnData { fq_name, args, ret_ty, is_extern }) => {
                output += &format!("\n      [Fn] {}(", fq_name);
                if !args.is_empty() {
                    output += &format!("{}: {}", args[0].0, args[0].1);
                    output += &args[1..].iter().fold(String::new(), |mut acc, (name, ty)| {
                        acc += &format!(", {}: {}", name, ty);
                        acc
                    });
                };
                output += &format!(") -> {}, is_extern: {}", ret_ty, is_extern);
            },
            AssocData::Var(VarData { ty }) => output += &format!("\n      [Var] type: {}", ty),
            AssocData::Struct(StructData { fields, methods }) => {
                output += "\n      [Struct] {{ ";
                if let Some(fields) = fields {
                    if !fields.is_empty() {
                        output += &format!("{}: {}", fields[0].0, fields[0].1);
                        output += &fields[1..].iter().fold(String::new(), |mut acc, (name, ty)| {
                            acc += &format!(", {}: {}", name, ty);
                            acc
                        });
                    }
                };
                output += " }";
                if let Some(methods) = methods {
                    if !methods.is_empty() {
                        output += &format!(" | {}()", methods[0]);
                        output += &methods[1..].iter().fold(String::new(), |mut acc, method| {
                            acc += &format!(", {}()", method);
                            acc
                        });
                    }
                }
            },
            AssocData::Module(_) => (),
        }
        write!(f, "{}", output)
    }
}
