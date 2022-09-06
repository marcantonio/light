use inkwell::values::PointerValue;
use std::collections::HashMap;

use common::Type;
use symbol_table::symbol::{AssocData, VarData};
use symbol_table::{Symbol, SymbolTable, Symbolic};

use crate::Codegen;

#[derive(PartialEq, Debug)]
pub struct CodegenSymbol<'a> {
    inner: Symbol,
    pointer: Option<PointerValue<'a>>,
}

impl<'a> CodegenSymbol<'a> {
    pub fn inner(&self) -> &Symbol {
        &self.inner
    }

    pub fn pointer(&self) -> Option<PointerValue<'a>> {
        self.pointer
    }
}

impl<'a> Symbolic for CodegenSymbol<'a> {
    fn name(&self) -> &str {
        self.inner.name()
    }
}

impl<'a> From<(&str, &Type, PointerValue<'a>)> for CodegenSymbol<'a> {
    fn from((name, ty, ptr): (&str, &Type, PointerValue<'a>)) -> Self {
        CodegenSymbol {
            inner: Symbol { name: name.to_owned(), data: AssocData::Var(VarData { ty: ty.to_owned() }) },
            pointer: Some(ptr),
        }
    }
}

impl<'a> From<Symbol> for CodegenSymbol<'a> {
    fn from(sym: Symbol) -> Self {
        CodegenSymbol { inner: sym, pointer: None }
    }
}

impl<'ctx> Codegen<'ctx> {
    pub fn convert_table(mut old: SymbolTable<Symbol>) -> Result<SymbolTable<CodegenSymbol<'ctx>>, String> {
        let symbols = old.dump_table(0)?;
        let mut table = HashMap::with_capacity(symbols.len());
        symbols.for_each(|(k, v)| {
            table.insert(k, CodegenSymbol::from(v));
        });
        Ok(SymbolTable::with_table(table))
    }
}